use super::{
    cpu_group::CpuGroup,
    finalize::{InitMemoryCleanupDeferred, KernelMappingProtectionDeferred, PtiFinalizeTrimmed},
    init_task::InitTask,
    process_prepare::{
        CredentialCore, RootPidNamespace, SecurityCore, SignalCore, TaskCreationCore,
        TaskFileContext,
    },
    rcu::RcuCore,
    scheduler::Scheduler,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    workqueue::Workqueue,
};
use crate::trace::Checkpoint;

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
pub enum TaskEntry {
    None,
    KernelInit,
    Kthreadd,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TaskKind {
    None,
    UserModeThread,
    KernelThread,
}

pub struct RcuSchedulerStart {
    lifecycle: Lifecycle,
    scheduler_active: bool,
    single_online_cpu: bool,
    gp_threads_still_deferred: bool,
}

impl RcuSchedulerStart {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            scheduler_active: false,
            single_online_cpu: false,
            gp_threads_still_deferred: true,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn scheduler_active(&self) -> bool {
        self.scheduler_active
    }

    pub const fn single_online_cpu(&self) -> bool {
        self.single_online_cpu
    }

    pub const fn gp_threads_still_deferred(&self) -> bool {
        self.gp_threads_still_deferred
    }

    pub fn setup(
        &mut self,
        rcu_core: &RcuCore,
        scheduler: &Scheduler,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || rcu_core.state() != State::Ready
            || scheduler.state() != State::Online
            || cpu_group.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.scheduler_active = true;
        self.single_online_cpu = cpu_group.boot_cpu_state() == State::Online;
        self.gp_threads_still_deferred = rcu_core.gp_threads_deferred();
        if !self.single_online_cpu || !self.gp_threads_still_deferred {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::RcuSchedulerStartReady,
        )
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
    cpu_id: usize,
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
            cpu_id: usize::MAX,
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

    pub const fn cpu_id(&self) -> usize {
        self.cpu_id
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
        task_creation_core: &TaskCreationCore,
        root_pid_namespace: &RootPidNamespace,
        scheduler: &Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || task_creation_core.state() != State::Ready
            || root_pid_namespace.state() != State::Ready
            || scheduler.state() != State::Online
            || scheduler.boot_runqueue().state() != State::Ready
        {
            return self.failed_setup();
        }

        self.pid = KERNEL_INIT_PID;
        self.thread_context_ready = true;
        self.sched_entity_ready = true;
        self.waiting_for_kthreadd_done = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::KernelInitTaskReady,
        )
    }

    pub fn enable(&mut self, scheduler: &Scheduler) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || scheduler.state() != State::Online
            || scheduler.boot_runqueue().state() != State::Ready
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

        self.enqueued = true;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::KernelInitTaskOnline,
        )
    }

    fn pin_to_boot_cpu(&mut self, root_pid_namespace: &RootPidNamespace, cpu_id: usize) -> bool {
        if self.lifecycle.state() != State::Online
            || root_pid_namespace.state() != State::Ready
            || self.pid != KERNEL_INIT_PID
        {
            return false;
        }

        self.pinned_to_boot_cpu = true;
        self.pf_no_setaffinity = true;
        self.cpu_id = cpu_id;
        true
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
            || !cpu_group.secondary_cpus_online()
            || !cpu_group.smp_concurrency_open()
        {
            return false;
        }

        self.pinned_to_boot_cpu = false;
        self.pf_no_setaffinity = false;
        self.cpu_id = usize::MAX;
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

pub struct KernelInitAffinity {
    lifecycle: Lifecycle,
    pid_lookup_used_root_namespace: bool,
}

impl KernelInitAffinity {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            pid_lookup_used_root_namespace: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn pid_lookup_used_root_namespace(&self) -> bool {
        self.pid_lookup_used_root_namespace
    }

    pub fn setup(
        &mut self,
        kernel_init_task: &mut KernelInitTask,
        root_pid_namespace: &RootPidNamespace,
        scheduler: &Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || kernel_init_task.state() != State::Online
            || root_pid_namespace.state() != State::Ready
            || scheduler.boot_runqueue().state() != State::Ready
        {
            return self.failed_setup();
        }

        if !kernel_init_task.pin_to_boot_cpu(root_pid_namespace, scheduler.boot_runqueue().cpu_id())
        {
            return self.failed_setup();
        }
        self.pid_lookup_used_root_namespace = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::KernelInitAffinityReady,
        )
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

pub struct KthreaddTask {
    lifecycle: Lifecycle,
    pid: usize,
    entry: TaskEntry,
    kind: TaskKind,
    clone_fs: bool,
    clone_files: bool,
    thread_context_ready: bool,
    sched_entity_ready: bool,
    global_ref_bound: bool,
    enqueued: bool,
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
            thread_context_ready: false,
            sched_entity_ready: false,
            global_ref_bound: false,
            enqueued: false,
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

    pub const fn thread_context_ready(&self) -> bool {
        self.thread_context_ready
    }

    pub const fn sched_entity_ready(&self) -> bool {
        self.sched_entity_ready
    }

    pub const fn global_ref_bound(&self) -> bool {
        self.global_ref_bound
    }

    pub const fn enqueued(&self) -> bool {
        self.enqueued
    }

    pub fn preset(&mut self, inputs: TaskSpawnInputs<'_>) -> EventResult {
        if self.lifecycle.state() != State::Base || !inputs.ready_for_kthreadd() {
            return self.failed_preset();
        }

        self.entry = TaskEntry::Kthreadd;
        self.kind = TaskKind::KernelThread;
        self.clone_fs = true;
        self.clone_files = true;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::KthreaddTaskPrepared,
        )
    }

    pub fn setup(
        &mut self,
        task_creation_core: &TaskCreationCore,
        root_pid_namespace: &RootPidNamespace,
        scheduler: &Scheduler,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || task_creation_core.state() != State::Ready
            || root_pid_namespace.state() != State::Ready
            || scheduler.state() != State::Online
            || scheduler.boot_runqueue().state() != State::Ready
        {
            return self.failed_setup();
        }

        self.pid = KTHREADD_PID;
        self.thread_context_ready = true;
        self.sched_entity_ready = true;
        self.global_ref_bound = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::KthreaddTaskReady,
        )
    }

    pub fn enable(&mut self, scheduler: &Scheduler) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || scheduler.state() != State::Online
            || scheduler.boot_runqueue().state() != State::Ready
            || !self.global_ref_bound
            || !self.sched_entity_ready
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.enqueued = true;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::KthreaddTaskOnline,
        )
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
    pending: bool,
    completed: bool,
    release_committed: bool,
}

impl KthreaddReadyGate {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            pending: false,
            completed: false,
            release_committed: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn pending(&self) -> bool {
        self.pending
    }

    pub const fn completed(&self) -> bool {
        self.completed
    }

    pub const fn release_committed(&self) -> bool {
        self.release_committed
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

        self.pending = true;
        self.completed = false;
        self.release_committed = false;
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
        kernel_init_task: &mut KernelInitTask,
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

        if !kernel_init_task.release_for_pre_smp_init() {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        self.pending = false;
        self.completed = true;
        self.release_committed = true;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::KthreaddReadyGateOnline,
        )
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

pub struct KernelInitDispatchGate {
    lifecycle: Lifecycle,
    schedule_committed: bool,
    kernel_init_dispatched: bool,
    boot_idle_tail_pending: bool,
}

impl KernelInitDispatchGate {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            schedule_committed: false,
            kernel_init_dispatched: false,
            boot_idle_tail_pending: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn schedule_committed(&self) -> bool {
        self.schedule_committed
    }

    pub const fn kernel_init_dispatched(&self) -> bool {
        self.kernel_init_dispatched
    }

    pub const fn boot_idle_tail_pending(&self) -> bool {
        self.boot_idle_tail_pending
    }

    pub fn setup(
        &mut self,
        scheduler: &mut Scheduler,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        system_state: &SystemState,
        kthreadd_ready_gate: &KthreaddReadyGate,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || scheduler.state() != State::Online
            || kernel_init_task.state() != State::Online
            || !kernel_init_task.released_for_pre_smp_init()
            || kthreadd_task.state() != State::Online
            || system_state.state() != State::Ready
            || system_state.value() != SystemStateValue::Scheduling
            || kthreadd_ready_gate.state() != State::Online
            || !kthreadd_ready_gate.release_committed()
        {
            return self.failed_setup();
        }

        let interrupts_enabled = crate::arch::riscv64::csr::supervisor_interrupts_enabled();
        if interrupts_enabled {
            crate::arch::riscv64::csr::disable_supervisor_interrupts();
        }
        let schedule_result = scheduler.schedule_preempt_disabled();
        if interrupts_enabled {
            crate::arch::riscv64::csr::enable_supervisor_interrupts();
        }
        if schedule_result.is_err() {
            return self.failed_setup();
        }

        self.schedule_committed = true;
        self.kernel_init_dispatched = true;
        self.boot_idle_tail_pending = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::KernelInitDispatchGateReady,
        )
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
    cpu_startup_entry_ready: bool,
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
            cpu_startup_entry_ready: false,
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

    pub const fn cpu_startup_entry_ready(&self) -> bool {
        self.cpu_startup_entry_ready
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
        dispatch_gate: &KernelInitDispatchGate,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || scheduler.state() != State::Online
            || scheduler.boot_idle_task().state() != State::Ready
            || kernel_init_task.state() != State::Online
            || kthreadd_task.state() != State::Online
            || kthreadd_ready_gate.state() != State::Online
            || dispatch_gate.state() != State::Ready
            || !dispatch_gate.schedule_committed()
            || !dispatch_gate.kernel_init_dispatched()
            || cpu_group.state() != State::Ready
            || cpu_group.boot_cpu_state() != State::Online
        {
            return self.failed_setup();
        }

        self.first_schedule_committed = true;
        self.cpu_startup_entry_ready = true;
        self.boot_init_handoff_complete = true;
        self.boot_cpu_hotplug_online = true;
        self.secondary_cpus_not_started = true;
        self.real_task_switch_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::BootIdleRuntimeReady,
        )
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
