use super::{
    completion::Completion,
    cpu_control::{
        BootCurrentCpu, CurrentTaskSlot, LocalInterruptControl, RawSpinLock, RcuReadSide,
    },
    cpu_group::CpuGroup,
    finalize::{InitMemoryCleanupDeferred, KernelMappingProtectionDeferred, PtiFinalizeTrimmed},
    init_task::InitTask,
    process_prepare::{
        CredentialCore, RootPidNamespace, SecurityCore, SignalCore, TaskCopyProcessInputs,
        TaskCreationCore, TaskFileContext,
    },
    rcu::RcuCore,
    scheduler::Scheduler,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    task::TaskCpuState,
    workqueue::Workqueue,
};
use crate::trace::Checkpoint;

pub use super::task::TaskEntry;

pub const KERNEL_INIT_PID: usize = 1;
pub const KTHREADD_PID: usize = 2;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum SystemStateValue {
    Booting,
    Scheduling,
    FreeingInitmem,
    Running,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TaskKind {
    None,
    UserModeThread,
    KernelThread,
}

pub struct KernelInitTask {
    lifecycle: Lifecycle,
    pid: usize,
    entry: TaskEntry,
    kind: TaskKind,
    clone_fs: bool,
    user_mm_created: bool,
    thread_context_ready: bool,
    sched_entity_ready: bool,
    enqueued: bool,
    waiting_for_kthreadd_done: bool,
    released_for_pre_smp_init: bool,
    pinned_to_boot_cpu: bool,
    pf_no_setaffinity: bool,
    pid_lookup_under_rcu_read: bool,
    pid_lookup_rcu_guard_balanced: bool,
    cpu: TaskCpuState,
    running: bool,
}

impl KernelInitTask {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            pid: 0,
            entry: TaskEntry::None,
            kind: TaskKind::None,
            clone_fs: false,
            user_mm_created: false,
            thread_context_ready: false,
            sched_entity_ready: false,
            enqueued: false,
            waiting_for_kthreadd_done: false,
            released_for_pre_smp_init: false,
            pinned_to_boot_cpu: false,
            pf_no_setaffinity: false,
            pid_lookup_under_rcu_read: false,
            pid_lookup_rcu_guard_balanced: false,
            cpu: TaskCpuState::new(),
            running: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn pid(&self) -> usize {
        self.pid
    }

    pub const fn entry(&self) -> TaskEntry {
        self.entry
    }

    pub const fn kind(&self) -> TaskKind {
        self.kind
    }

    pub const fn clone_fs(&self) -> bool {
        self.clone_fs
    }

    pub const fn user_mm_created(&self) -> bool {
        self.user_mm_created
    }

    pub const fn thread_context_ready(&self) -> bool {
        self.thread_context_ready
    }

    pub const fn sched_entity_ready(&self) -> bool {
        self.sched_entity_ready
    }

    pub const fn enqueued(&self) -> bool {
        self.enqueued
    }

    pub const fn waiting_for_kthreadd_done(&self) -> bool {
        self.waiting_for_kthreadd_done
    }

    pub const fn released_for_pre_smp_init(&self) -> bool {
        self.released_for_pre_smp_init
    }

    pub const fn pinned_to_boot_cpu(&self) -> bool {
        self.pinned_to_boot_cpu
    }

    pub const fn pf_no_setaffinity(&self) -> bool {
        self.pf_no_setaffinity
    }

    pub const fn pid_lookup_under_rcu_read(&self) -> bool {
        self.pid_lookup_under_rcu_read
    }

    pub const fn pid_lookup_rcu_guard_balanced(&self) -> bool {
        self.pid_lookup_rcu_guard_balanced
    }

    pub const fn cpu_id(&self) -> usize {
        self.cpu.cpu_id()
    }

    pub const fn running(&self) -> bool {
        self.running
    }

    pub fn preset(&mut self, inputs: TaskSpawnInputs<'_>) -> EventResult {
        if self.lifecycle.state() != State::Base || !inputs.ready_for_kernel_init() {
            return self.failed_preset();
        }

        self.entry = TaskEntry::KernelInit;
        self.kind = TaskKind::UserModeThread;
        self.clone_fs = true;
        self.user_mm_created = false;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::KernelInitTaskPrepared,
        )
    }

    pub fn setup(
        &mut self,
        task_creation_core: &mut TaskCreationCore,
        root_pid_namespace: &RootPidNamespace,
        credential_core: &CredentialCore,
        signal_core: &SignalCore,
        task_file_context: &TaskFileContext,
        security_core: &SecurityCore,
        init_task: &InitTask,
        scheduler: &Scheduler,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || task_creation_core.state() != State::Ready
            || root_pid_namespace.state() != State::Ready
            || scheduler.state() != State::Online
            || scheduler.boot_cpu_owned_scheduler_view(cpu_group).is_none()
        {
            return self.failed_setup();
        }

        let copy_result = task_creation_core.copy_process(
            TaskCopyProcessInputs {
                src_task: init_task,
                root_pid_namespace,
                credential_core,
                signal_core,
                task_file_context,
                security_core,
                scheduler,
                cpu_group,
                entry: TaskEntry::KernelInit,
            },
            self.lifecycle.state(),
            self.entry,
        )?;
        if copy_result.entry() != TaskEntry::KernelInit
            || !copy_result.task_struct_allocated()
            || !copy_result.thread_context_ready()
            || !copy_result.sched_entity_ready()
            || !copy_result.task_state_new()
            || !copy_result.task_not_enqueued()
        {
            return self.failed_setup();
        }

        self.pid = KERNEL_INIT_PID;
        self.thread_context_ready = copy_result.thread_context_ready();
        self.sched_entity_ready = copy_result.sched_entity_ready();
        self.waiting_for_kthreadd_done = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::KernelInitTaskReady,
        )
    }

    pub fn enable(
        &mut self,
        scheduler: &mut Scheduler,
        cpu_group: &CpuGroup,
        current_cpu: &BootCurrentCpu,
        local_interrupt: &mut LocalInterruptControl,
        current_task_slot: &CurrentTaskSlot,
        pi_lock: &mut RawSpinLock,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || scheduler.state() != State::Online
            || scheduler.boot_cpu_owned_scheduler_view(cpu_group).is_none()
            || cpu_group.state() != State::Ready
            || current_cpu.state() != State::Online
            || local_interrupt.state() != State::Ready
            || current_task_slot.state() != State::Ready
            || !current_task_slot.current_is_boot_idle()
            || pi_lock.state() != State::Ready
            || scheduler.boot_idle_preemption().state() != State::Ready
            || self.pid != KERNEL_INIT_PID
            || !self.sched_entity_ready
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        pi_lock.lock_irqsave(local_interrupt, scheduler.boot_idle_preemption_mut())?;
        let guarded_result = (|| {
            self.running = true;
            let selected_rq = scheduler.select_runqueue_for_task(self.pid, cpu_group)?;
            self.set_task_cpu(selected_rq.cpu_id())?;
            scheduler.enqueue_task_on_runqueue(self.pid, selected_rq)?;
            self.enqueued = true;
            self.lifecycle.transition(
                LifecycleEvent::Enable,
                State::Ready,
                State::Online,
                Checkpoint::KernelInitTaskOnline,
            )
        })();
        let unlock_result =
            pi_lock.unlock_irqrestore(local_interrupt, scheduler.boot_idle_preemption_mut());
        guarded_result.and(unlock_result)
    }

    pub fn pin_to_boot_cpu(
        &mut self,
        cpu_id: usize,
        root_pid_namespace: &RootPidNamespace,
        boot_idle_rcu_read_side: &mut RcuReadSide,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || self.pid != KERNEL_INIT_PID
            || self.cpu_id() != cpu_id
            || root_pid_namespace.state() != State::Ready
            || boot_idle_rcu_read_side.state() != State::Prepared
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        let locks_before = boot_idle_rcu_read_side.read_lock_count();
        boot_idle_rcu_read_side.read_lock()?;
        let guarded_result = (|| {
            self.pid_lookup_under_rcu_read =
                boot_idle_rcu_read_side.read_lock_count() == locks_before.wrapping_add(1);
            self.pinned_to_boot_cpu = true;
            self.pf_no_setaffinity = true;
            Ok(())
        })();
        let unlock_result = boot_idle_rcu_read_side.read_unlock();
        if guarded_result.is_ok() && unlock_result.is_ok() {
            self.pid_lookup_rcu_guard_balanced = boot_idle_rcu_read_side.balanced();
        }
        guarded_result.and(unlock_result)
    }

    fn set_task_cpu(&mut self, cpu_id: usize) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || self.pid != KERNEL_INIT_PID
            || cpu_id == usize::MAX
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        if self.cpu.set_task_cpu(cpu_id) {
            Ok(())
        } else {
            failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            )
        }
    }

    fn release_for_pre_smp_init(&mut self) -> bool {
        if self.lifecycle.state() != State::Online || !self.waiting_for_kthreadd_done {
            return false;
        }

        self.waiting_for_kthreadd_done = false;
        self.released_for_pre_smp_init = true;
        true
    }

    pub fn release_boot_cpu_affinity(&mut self, cpu_group: &CpuGroup) -> bool {
        if self.lifecycle.state() != State::Online
            || self.pid != KERNEL_INIT_PID
            || !self.pinned_to_boot_cpu
            || !self.pf_no_setaffinity
            || self.cpu_id() == usize::MAX
            || !cpu_group.secondary_cpus_online()
            || !cpu_group.smp_concurrency_open()
        {
            return false;
        }

        self.pinned_to_boot_cpu = false;
        self.pf_no_setaffinity = false;
        true
    }

    fn failed_preset(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            State::Base,
            State::Prepared,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Prepared,
            State::Ready,
        )
    }
}

pub struct KthreaddTask {
    lifecycle: Lifecycle,
    pid: usize,
    entry: TaskEntry,
    kind: TaskKind,
    clone_fs: bool,
    clone_files: bool,
    clone_vm: bool,
    clone_untraced: bool,
    kernel_thread_flag: bool,
    thread_context_ready: bool,
    sched_entity_ready: bool,
    global_ref_bound: bool,
    provider_ready: bool,
    pid_lookup_under_rcu_read: bool,
    pid_lookup_rcu_guard_balanced: bool,
    schedule_loop_deferred: bool,
    enqueued: bool,
    cpu: TaskCpuState,
    running: bool,
}

impl KthreaddTask {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            pid: 0,
            entry: TaskEntry::None,
            kind: TaskKind::None,
            clone_fs: false,
            clone_files: false,
            clone_vm: false,
            clone_untraced: false,
            kernel_thread_flag: false,
            thread_context_ready: false,
            sched_entity_ready: false,
            global_ref_bound: false,
            provider_ready: false,
            pid_lookup_under_rcu_read: false,
            pid_lookup_rcu_guard_balanced: false,
            schedule_loop_deferred: true,
            enqueued: false,
            cpu: TaskCpuState::new(),
            running: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn pid(&self) -> usize {
        self.pid
    }

    pub const fn entry(&self) -> TaskEntry {
        self.entry
    }

    pub const fn kind(&self) -> TaskKind {
        self.kind
    }

    pub const fn clone_fs(&self) -> bool {
        self.clone_fs
    }

    pub const fn clone_files(&self) -> bool {
        self.clone_files
    }

    pub const fn clone_vm(&self) -> bool {
        self.clone_vm
    }

    pub const fn clone_untraced(&self) -> bool {
        self.clone_untraced
    }

    pub const fn kernel_thread_flag(&self) -> bool {
        self.kernel_thread_flag
    }

    pub const fn thread_context_ready(&self) -> bool {
        self.thread_context_ready
    }

    pub const fn sched_entity_ready(&self) -> bool {
        self.sched_entity_ready
    }

    pub const fn global_ref_bound(&self) -> bool {
        self.global_ref_bound
    }

    pub const fn provider_ready(&self) -> bool {
        self.provider_ready
    }

    pub const fn pid_lookup_under_rcu_read(&self) -> bool {
        self.pid_lookup_under_rcu_read
    }

    pub const fn pid_lookup_rcu_guard_balanced(&self) -> bool {
        self.pid_lookup_rcu_guard_balanced
    }

    pub const fn schedule_loop_deferred(&self) -> bool {
        self.schedule_loop_deferred
    }

    pub const fn enqueued(&self) -> bool {
        self.enqueued
    }

    pub const fn cpu_id(&self) -> usize {
        self.cpu.cpu_id()
    }

    pub const fn running(&self) -> bool {
        self.running
    }

    pub fn preset(&mut self, inputs: TaskSpawnInputs<'_>) -> EventResult {
        if self.lifecycle.state() != State::Base || !inputs.ready_for_kthreadd() {
            return self.failed_preset();
        }

        self.entry = TaskEntry::Kthreadd;
        self.kind = TaskKind::KernelThread;
        self.clone_fs = true;
        self.clone_files = true;
        self.clone_vm = true;
        self.clone_untraced = true;
        self.kernel_thread_flag = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::KthreaddTaskPrepared,
        )
    }

    pub fn setup(
        &mut self,
        task_creation_core: &mut TaskCreationCore,
        root_pid_namespace: &RootPidNamespace,
        credential_core: &CredentialCore,
        signal_core: &SignalCore,
        task_file_context: &TaskFileContext,
        security_core: &SecurityCore,
        init_task: &InitTask,
        scheduler: &Scheduler,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || task_creation_core.state() != State::Ready
            || root_pid_namespace.state() != State::Ready
            || scheduler.state() != State::Online
            || scheduler.boot_cpu_owned_scheduler_view(cpu_group).is_none()
        {
            return self.failed_setup();
        }

        let copy_result = task_creation_core.copy_process(
            TaskCopyProcessInputs {
                src_task: init_task,
                root_pid_namespace,
                credential_core,
                signal_core,
                task_file_context,
                security_core,
                scheduler,
                cpu_group,
                entry: TaskEntry::Kthreadd,
            },
            self.lifecycle.state(),
            self.entry,
        )?;
        if copy_result.entry() != TaskEntry::Kthreadd
            || !copy_result.task_struct_allocated()
            || !copy_result.thread_context_ready()
            || !copy_result.sched_entity_ready()
            || !copy_result.task_state_new()
            || !copy_result.task_not_enqueued()
        {
            return self.failed_setup();
        }

        self.pid = KTHREADD_PID;
        self.thread_context_ready = copy_result.thread_context_ready();
        self.sched_entity_ready = copy_result.sched_entity_ready();
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::KthreaddTaskReady,
        )
    }

    pub fn enable(
        &mut self,
        scheduler: &mut Scheduler,
        cpu_group: &CpuGroup,
        current_cpu: &BootCurrentCpu,
        local_interrupt: &mut LocalInterruptControl,
        current_task_slot: &CurrentTaskSlot,
        pi_lock: &mut RawSpinLock,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || scheduler.state() != State::Online
            || scheduler.boot_cpu_owned_scheduler_view(cpu_group).is_none()
            || cpu_group.state() != State::Ready
            || current_cpu.state() != State::Online
            || local_interrupt.state() != State::Ready
            || current_task_slot.state() != State::Ready
            || !current_task_slot.current_is_boot_idle()
            || pi_lock.state() != State::Ready
            || scheduler.boot_idle_preemption().state() != State::Ready
            || self.pid != KTHREADD_PID
            || !self.sched_entity_ready
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        pi_lock.lock_irqsave(local_interrupt, scheduler.boot_idle_preemption_mut())?;
        let guarded_result = (|| {
            self.running = true;
            let selected_rq = scheduler.select_runqueue_for_task(self.pid, cpu_group)?;
            self.set_task_cpu(selected_rq.cpu_id())?;
            scheduler.enqueue_task_on_runqueue(self.pid, selected_rq)?;
            self.enqueued = true;
            self.lifecycle.transition(
                LifecycleEvent::Enable,
                State::Ready,
                State::Online,
                Checkpoint::KthreaddTaskOnline,
            )
        })();
        let unlock_result =
            pi_lock.unlock_irqrestore(local_interrupt, scheduler.boot_idle_preemption_mut());
        guarded_result.and(unlock_result)
    }

    fn set_task_cpu(&mut self, cpu_id: usize) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || self.pid != KTHREADD_PID
            || cpu_id == usize::MAX
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        if self.cpu.set_task_cpu(cpu_id) {
            Ok(())
        } else {
            failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            )
        }
    }

    pub fn bind_global_ref(
        &mut self,
        root_pid_namespace: &RootPidNamespace,
        boot_idle_rcu_read_side: &mut RcuReadSide,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || self.pid != KTHREADD_PID
            || !self.running
            || !self.enqueued
            || root_pid_namespace.state() != State::Ready
            || boot_idle_rcu_read_side.state() != State::Prepared
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        let locks_before = boot_idle_rcu_read_side.read_lock_count();
        boot_idle_rcu_read_side.read_lock()?;
        let guarded_result = (|| {
            self.pid_lookup_under_rcu_read =
                boot_idle_rcu_read_side.read_lock_count() == locks_before.wrapping_add(1);
            self.global_ref_bound = true;
            self.provider_ready = true;
            crate::trace::checkpoint(Checkpoint::KthreaddTaskGlobalRefBound);
            Ok(())
        })();
        let unlock_result = boot_idle_rcu_read_side.read_unlock();
        if guarded_result.is_ok() && unlock_result.is_ok() {
            self.pid_lookup_rcu_guard_balanced = boot_idle_rcu_read_side.balanced();
        }
        guarded_result.and(unlock_result)
    }

    fn failed_preset(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Preset,
            self.lifecycle.state(),
            State::Base,
            State::Prepared,
        )
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Prepared,
            State::Ready,
        )
    }
}

pub struct SystemState {
    lifecycle: Lifecycle,
    value: SystemStateValue,
}

impl SystemState {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            value: SystemStateValue::Booting,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn value(&self) -> SystemStateValue {
        self.value
    }

    pub fn preset(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.value = SystemStateValue::Booting;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::SystemStatePrepared,
        )
    }

    pub fn setup(
        &mut self,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || kernel_init_task.state() != State::Online
            || kthreadd_task.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.value = SystemStateValue::Scheduling;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::SystemStateReady,
        )
    }

    pub fn enable(
        &mut self,
        init_memory: &InitMemoryCleanupDeferred,
        mapping: &KernelMappingProtectionDeferred,
        pti_finalize: &PtiFinalizeTrimmed,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || self.value != SystemStateValue::Scheduling
            || init_memory.state() != State::Ready
            || !init_memory.system_state_freeing_window_entered()
            || mapping.state() != State::Ready
            || !mapping.enable_deferred()
            || pti_finalize.state() != State::Ready
            || !pti_finalize.trimmed_noop()
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.value = SystemStateValue::FreeingInitmem;
        crate::trace::checkpoint(Checkpoint::SystemStateFreeingInitmemCheckpoint);
        self.value = SystemStateValue::Running;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::SystemStateOnline,
        )
    }
}

pub struct KthreaddReadyGate {
    lifecycle: Lifecycle,
    completion: Completion,
    release_committed: bool,
    complete_wait_lock_guard_used: bool,
    complete_wait_lock_irqsave_count: usize,
    complete_wait_lock_irqrestore_count: usize,
    complete_done_increment_guarded: bool,
    complete_wake_guarded: bool,
}

impl KthreaddReadyGate {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            completion: Completion::new(),
            release_committed: false,
            complete_wait_lock_guard_used: false,
            complete_wait_lock_irqsave_count: 0,
            complete_wait_lock_irqrestore_count: 0,
            complete_done_increment_guarded: false,
            complete_wake_guarded: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn completion(&self) -> &Completion {
        &self.completion
    }

    pub fn pending(&self) -> bool {
        self.completion.pending()
    }

    pub fn completed(&self) -> bool {
        self.completion.completed()
    }

    pub const fn release_committed(&self) -> bool {
        self.release_committed
    }

    pub const fn complete_wait_lock_guard_used(&self) -> bool {
        self.complete_wait_lock_guard_used
    }

    pub const fn complete_wait_lock_irqsave_count(&self) -> usize {
        self.complete_wait_lock_irqsave_count
    }

    pub const fn complete_wait_lock_irqrestore_count(&self) -> usize {
        self.complete_wait_lock_irqrestore_count
    }

    pub const fn complete_done_increment_guarded(&self) -> bool {
        self.complete_done_increment_guarded
    }

    pub const fn complete_wake_guarded(&self) -> bool {
        self.complete_wake_guarded
    }

    pub fn setup(
        &mut self,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_init_task.state() != State::Online
            || kthreadd_task.state() != State::Online
            || !kernel_init_task.waiting_for_kthreadd_done()
        {
            return self.failed_setup();
        }

        self.completion.setup()?;
        self.release_committed = false;
        self.complete_wait_lock_guard_used = false;
        self.complete_wait_lock_irqsave_count = 0;
        self.complete_wait_lock_irqrestore_count = 0;
        self.complete_done_increment_guarded = false;
        self.complete_wake_guarded = false;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::KthreaddReadyGateReady,
        )
    }

    pub fn enable(
        &mut self,
        system_state: &SystemState,
        kthreadd_task: &KthreaddTask,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || system_state.state() != State::Ready
            || system_state.value() != SystemStateValue::Scheduling
            || kthreadd_task.state() != State::Online
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.completion.enable()?;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::KthreaddReadyGateOnline,
        )
    }

    pub fn complete(
        &mut self,
        system_state: &SystemState,
        kthreadd_task: &KthreaddTask,
        kernel_init_task: &mut KernelInitTask,
        wait_lock: &mut RawSpinLock,
        local_interrupt: &mut LocalInterruptControl,
        scheduler: &mut Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || system_state.state() != State::Ready
            || system_state.value() != SystemStateValue::Scheduling
            || kthreadd_task.state() != State::Online
            || kernel_init_task.state() != State::Online
            || !kernel_init_task.waiting_for_kthreadd_done()
            || wait_lock.state() != State::Ready
            || local_interrupt.state() != State::Ready
            || scheduler.boot_idle_preemption().state() != State::Ready
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        wait_lock.lock_irqsave(local_interrupt, scheduler.boot_idle_preemption_mut())?;
        let guarded_result = (|| {
            self.complete_wait_lock_guard_used = wait_lock.locked();
            self.complete_wait_lock_irqsave_count = wait_lock.irqsave_entered_count();
            self.completion.complete()?;
            self.complete_done_increment_guarded =
                wait_lock.locked() && self.completion.completed();
            self.complete_wake_guarded = wait_lock.locked() && self.completion.wakes_one_waiter();
            Ok(())
        })();
        let unlock_result =
            wait_lock.unlock_irqrestore(local_interrupt, scheduler.boot_idle_preemption_mut());
        if guarded_result.is_ok() && unlock_result.is_ok() {
            self.complete_wait_lock_irqrestore_count = wait_lock.irqrestore_exited_count();
        }
        guarded_result.and(unlock_result)?;
        if !kernel_init_task.release_for_pre_smp_init() {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }
        self.release_committed = true;
        Ok(())
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct BootIdleRuntime {
    lifecycle: Lifecycle,
    first_schedule_committed: bool,
    idle_entry_prepared: bool,
    cpu_startup_entry_ready: bool,
    idle_loop_entered: bool,
    idle_cycle_committed: bool,
    idle_cycle_started: bool,
    need_resched_clear_before_wait: bool,
    observed_no_need_resched: bool,
    idle_polling_set: bool,
    nohz_idle_entered: bool,
    idle_wait_committed: bool,
    idle_wait_path_deferred: bool,
    need_resched_set_for_schedule: bool,
    observed_need_resched: bool,
    idle_polling_cleared: bool,
    nohz_idle_exited: bool,
    idle_schedule_requested: bool,
    idle_schedule_returned: bool,
    need_resched_drained: bool,
    idle_loop_continues: bool,
    boot_init_handoff_complete: bool,
    boot_cpu_hotplug_online: bool,
    secondary_cpus_not_started: bool,
    real_task_switch_deferred: bool,
}

impl BootIdleRuntime {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            first_schedule_committed: false,
            idle_entry_prepared: false,
            cpu_startup_entry_ready: false,
            idle_loop_entered: false,
            idle_cycle_committed: false,
            idle_cycle_started: false,
            need_resched_clear_before_wait: false,
            observed_no_need_resched: false,
            idle_polling_set: false,
            nohz_idle_entered: false,
            idle_wait_committed: false,
            idle_wait_path_deferred: false,
            need_resched_set_for_schedule: false,
            observed_need_resched: false,
            idle_polling_cleared: false,
            nohz_idle_exited: false,
            idle_schedule_requested: false,
            idle_schedule_returned: false,
            need_resched_drained: false,
            idle_loop_continues: false,
            boot_init_handoff_complete: false,
            boot_cpu_hotplug_online: false,
            secondary_cpus_not_started: true,
            real_task_switch_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn first_schedule_committed(&self) -> bool {
        self.first_schedule_committed
    }

    pub const fn idle_entry_prepared(&self) -> bool {
        self.idle_entry_prepared
    }

    pub const fn cpu_startup_entry_ready(&self) -> bool {
        self.cpu_startup_entry_ready
    }

    pub const fn idle_loop_entered(&self) -> bool {
        self.idle_loop_entered
    }

    pub const fn idle_cycle_committed(&self) -> bool {
        self.idle_cycle_committed
    }

    pub const fn idle_cycle_started(&self) -> bool {
        self.idle_cycle_started
    }

    pub const fn need_resched_clear_before_wait(&self) -> bool {
        self.need_resched_clear_before_wait
    }

    pub const fn observed_no_need_resched(&self) -> bool {
        self.observed_no_need_resched
    }

    pub const fn idle_polling_set(&self) -> bool {
        self.idle_polling_set
    }

    pub const fn nohz_idle_entered(&self) -> bool {
        self.nohz_idle_entered
    }

    pub const fn idle_wait_committed(&self) -> bool {
        self.idle_wait_committed
    }

    pub const fn idle_wait_path_deferred(&self) -> bool {
        self.idle_wait_path_deferred
    }

    pub const fn need_resched_set_for_schedule(&self) -> bool {
        self.need_resched_set_for_schedule
    }

    pub const fn observed_need_resched(&self) -> bool {
        self.observed_need_resched
    }

    pub const fn idle_polling_cleared(&self) -> bool {
        self.idle_polling_cleared
    }

    pub const fn nohz_idle_exited(&self) -> bool {
        self.nohz_idle_exited
    }

    pub const fn idle_schedule_requested(&self) -> bool {
        self.idle_schedule_requested
    }

    pub const fn idle_schedule_returned(&self) -> bool {
        self.idle_schedule_returned
    }

    pub const fn need_resched_drained(&self) -> bool {
        self.need_resched_drained
    }

    pub const fn idle_loop_continues(&self) -> bool {
        self.idle_loop_continues
    }

    pub const fn representative_need_resched_cycle_committed(&self) -> bool {
        self.idle_cycle_started
            && self.need_resched_clear_before_wait
            && self.observed_no_need_resched
            && self.idle_polling_set
            && self.nohz_idle_entered
            && self.idle_wait_committed
            && self.idle_wait_path_deferred
            && self.need_resched_set_for_schedule
            && self.observed_need_resched
            && self.idle_polling_cleared
            && self.nohz_idle_exited
            && self.idle_schedule_requested
            && self.idle_schedule_returned
            && self.need_resched_drained
            && self.idle_loop_continues
    }

    pub const fn boot_init_handoff_complete(&self) -> bool {
        self.boot_init_handoff_complete
    }

    pub const fn boot_cpu_hotplug_online(&self) -> bool {
        self.boot_cpu_hotplug_online
    }

    pub const fn secondary_cpus_not_started(&self) -> bool {
        self.secondary_cpus_not_started
    }

    pub const fn real_task_switch_deferred(&self) -> bool {
        self.real_task_switch_deferred
    }

    pub fn setup(
        &mut self,
        scheduler: &Scheduler,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        kthreadd_ready_gate: &KthreaddReadyGate,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || scheduler.state() != State::Online
            || scheduler.boot_cpu_owned_scheduler_view(cpu_group).is_none()
            || kernel_init_task.state() != State::Online
            || !kernel_init_task.released_for_pre_smp_init()
            || kthreadd_task.state() != State::Online
            || kthreadd_ready_gate.state() != State::Online
            || scheduler.schedule_passes() == 0
            || cpu_group.state() != State::Ready
            || cpu_group.boot_cpu_state() != State::Online
        {
            return self.failed_setup();
        }

        self.first_schedule_committed = true;
        self.idle_entry_prepared = false;
        self.cpu_startup_entry_ready = false;
        self.idle_loop_entered = false;
        self.idle_cycle_committed = false;
        self.idle_cycle_started = false;
        self.need_resched_clear_before_wait = false;
        self.observed_no_need_resched = false;
        self.idle_polling_set = false;
        self.nohz_idle_entered = false;
        self.idle_wait_committed = false;
        self.idle_wait_path_deferred = false;
        self.need_resched_set_for_schedule = false;
        self.observed_need_resched = false;
        self.idle_polling_cleared = false;
        self.nohz_idle_exited = false;
        self.idle_schedule_requested = false;
        self.idle_schedule_returned = false;
        self.need_resched_drained = false;
        self.idle_loop_continues = false;
        self.boot_init_handoff_complete = false;
        self.boot_cpu_hotplug_online = false;
        self.secondary_cpus_not_started = true;
        self.real_task_switch_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::BootIdleRuntimeReady,
        )
    }

    pub fn prepare_idle_entry(
        &mut self,
        scheduler: &Scheduler,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.first_schedule_committed
            || scheduler.state() != State::Online
            || scheduler.boot_cpu_owned_scheduler_view(cpu_group).is_none()
            || cpu_group.state() != State::Ready
            || cpu_group.boot_cpu_state() != State::Online
        {
            return self.failed_ready_action();
        }

        self.idle_entry_prepared = true;
        self.cpu_startup_entry_ready = true;
        self.boot_init_handoff_complete = true;
        self.boot_cpu_hotplug_online = true;
        self.need_resched_clear_before_wait = true;
        Ok(())
    }

    pub fn run_idle_loop(
        &mut self,
        scheduler: &mut Scheduler,
        cpu_group: &CpuGroup,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        local_interrupt: &mut LocalInterruptControl,
        current_task_slot: &mut CurrentTaskSlot,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.idle_entry_prepared
            || scheduler.state() != State::Online
        {
            return self.failed_ready_action();
        }

        self.do_idle_cycle(
            scheduler,
            cpu_group,
            kernel_init_task,
            kthreadd_task,
            local_interrupt,
            current_task_slot,
        )?;
        self.idle_loop_entered = true;
        self.idle_loop_continues = true;
        Ok(())
    }

    fn do_idle_cycle(
        &mut self,
        scheduler: &mut Scheduler,
        cpu_group: &CpuGroup,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        local_interrupt: &mut LocalInterruptControl,
        current_task_slot: &mut CurrentTaskSlot,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.idle_entry_prepared
            || scheduler.state() != State::Online
        {
            return self.failed_ready_action();
        }

        self.wait_while_no_need_resched()?;
        self.observe_need_resched()?;
        self.schedule_if_need_resched(
            scheduler,
            cpu_group,
            kernel_init_task,
            kthreadd_task,
            local_interrupt,
            current_task_slot,
        )?;
        self.idle_cycle_committed = true;
        self.secondary_cpus_not_started = true;
        self.real_task_switch_deferred = true;
        Ok(())
    }

    fn wait_while_no_need_resched(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || !self.idle_entry_prepared {
            return self.failed_ready_action();
        }

        self.idle_cycle_started = true;
        self.need_resched_clear_before_wait = true;
        self.observed_no_need_resched = true;
        self.idle_polling_set = true;
        self.nohz_idle_entered = true;
        self.idle_wait_committed = true;
        self.idle_wait_path_deferred = true;
        Ok(())
    }

    fn observe_need_resched(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.idle_entry_prepared
            || !self.idle_wait_committed
            || !self.need_resched_clear_before_wait
        {
            return self.failed_ready_action();
        }

        self.need_resched_set_for_schedule = true;
        self.observed_need_resched = true;
        self.idle_polling_cleared = true;
        self.nohz_idle_exited = true;
        Ok(())
    }

    fn schedule_if_need_resched(
        &mut self,
        scheduler: &mut Scheduler,
        cpu_group: &CpuGroup,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        local_interrupt: &mut LocalInterruptControl,
        current_task_slot: &mut CurrentTaskSlot,
    ) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.idle_entry_prepared
            || !self.need_resched_set_for_schedule
            || !self.observed_need_resched
            || scheduler.state() != State::Online
            || scheduler.schedule_passes() == 0
            || current_task_slot.state() != State::Ready
            || !current_task_slot.current_is_boot_idle()
        {
            return self.failed_ready_action();
        }

        self.idle_schedule_requested = true;
        scheduler.schedule_idle(
            cpu_group,
            kernel_init_task,
            kthreadd_task,
            local_interrupt,
            current_task_slot,
        )?;
        self.idle_schedule_returned = true;
        self.need_resched_drained = true;
        self.idle_loop_continues = true;
        Ok(())
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }

    fn failed_ready_action(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Ready,
            State::Ready,
        )
    }
}

pub struct TaskSpawnInputs<'a> {
    pub task_creation_core: &'a TaskCreationCore,
    pub root_pid_namespace: &'a RootPidNamespace,
    pub credential_core: &'a CredentialCore,
    pub signal_core: &'a SignalCore,
    pub task_file_context: &'a TaskFileContext,
    pub security_core: &'a SecurityCore,
    pub init_task: &'a InitTask,
}

impl TaskSpawnInputs<'_> {
    fn ready_for_kernel_init(&self) -> bool {
        self.task_creation_core.state() == State::Ready
            && self.root_pid_namespace.state() == State::Ready
            && self.credential_core.state() == State::Prepared
            && self.signal_core.state() == State::Prepared
            && self.task_file_context.state() == State::Prepared
            && self.security_core.state() == State::Ready
            && self.init_task.state() == State::Online
    }

    fn ready_for_kthreadd(&self) -> bool {
        self.task_creation_core.state() == State::Ready
            && self.root_pid_namespace.state() == State::Ready
            && self.credential_core.state() == State::Prepared
            && self.task_file_context.state() == State::Prepared
            && self.init_task.state() == State::Online
    }
}

pub fn runtime_services_still_deferred(
    workqueue: &Workqueue,
    rcu_core: &RcuCore,
    cpu_group: &CpuGroup,
) -> bool {
    !workqueue.workers_running()
        && (rcu_core.gp_threads_deferred() || rcu_core.tasks_rcu().gp_threads_ready())
        && cpu_group.boot_cpu_state() == State::Online
}
