use super::{
    cpu_control::{CurrentTaskRef, CurrentTaskSlot, LocalInterruptControl, PreemptionControl},
    cpu_group::CpuGroup,
    cpu_id_map::CpuIdMap,
    init_mm::InitMm,
    init_task::InitTask,
    per_cpu_storage::PerCpuStorage,
    rest_init::KernelInitTask,
    state::{
        failed_condition, EventError, EventErrorCode, EventResult, Lifecycle, LifecycleEvent, State,
    },
    static_branch::StaticBranch,
    task::TaskCpuState,
};
use crate::trace::Checkpoint;

pub struct Scheduler {
    lifecycle: Lifecycle,
    default_root_domain: DefaultSchedRootDomain,
    bit_wait_queue_table: BitWaitQueueTable,
    boot_runqueue: BootRunQueue,
    boot_idle_task: BootIdleTask,
    boot_idle_preemption: PreemptionControl,
    scheduler_running: bool,
    selected_runqueue_task_id: usize,
    schedule_passes: usize,
    current_runqueue_resolve_passes: usize,
    pick_next_task_passes: usize,
    smp_initialized: bool,
    sched_domains_ready: bool,
    kernel_init_affinity_released: bool,
    rt_dl_smp_ready: bool,
    granularity_refreshed: bool,
    switch_to_passes: usize,
    identity_switch_passes: usize,
    idle_schedule_passes: usize,
    idle_schedule_returned_passes: usize,
    idle_schedule_identity_passes: usize,
    pick_next_task_exit_prev_ref: CurrentTaskRef,
    pick_next_task_exit_next_ref: CurrentTaskRef,
    pick_next_task_exit_count: usize,
    switch_to_entry_prev_ref: CurrentTaskRef,
    switch_to_entry_next_ref: CurrentTaskRef,
    switch_to_entry_current_ref: CurrentTaskRef,
    switch_to_entry_committed_count: usize,
    switch_to_entry_count: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            default_root_domain: DefaultSchedRootDomain::new(),
            bit_wait_queue_table: BitWaitQueueTable::new(),
            boot_runqueue: BootRunQueue::new(),
            boot_idle_task: BootIdleTask::new(),
            boot_idle_preemption: PreemptionControl::new(),
            scheduler_running: false,
            selected_runqueue_task_id: usize::MAX,
            schedule_passes: 0,
            current_runqueue_resolve_passes: 0,
            pick_next_task_passes: 0,
            smp_initialized: false,
            sched_domains_ready: false,
            kernel_init_affinity_released: false,
            rt_dl_smp_ready: false,
            granularity_refreshed: false,
            switch_to_passes: 0,
            identity_switch_passes: 0,
            idle_schedule_passes: 0,
            idle_schedule_returned_passes: 0,
            idle_schedule_identity_passes: 0,
            pick_next_task_exit_prev_ref: CurrentTaskRef::None,
            pick_next_task_exit_next_ref: CurrentTaskRef::None,
            pick_next_task_exit_count: 0,
            switch_to_entry_prev_ref: CurrentTaskRef::None,
            switch_to_entry_next_ref: CurrentTaskRef::None,
            switch_to_entry_current_ref: CurrentTaskRef::None,
            switch_to_entry_committed_count: 0,
            switch_to_entry_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn default_root_domain(&self) -> &DefaultSchedRootDomain {
        &self.default_root_domain
    }

    pub const fn bit_wait_queue_table(&self) -> &BitWaitQueueTable {
        &self.bit_wait_queue_table
    }

    pub const fn boot_runqueue(&self) -> &BootRunQueue {
        &self.boot_runqueue
    }

    pub const fn boot_idle_task(&self) -> &BootIdleTask {
        &self.boot_idle_task
    }

    pub const fn boot_idle_preemption(&self) -> &PreemptionControl {
        &self.boot_idle_preemption
    }

    pub fn boot_idle_preemption_mut(&mut self) -> &mut PreemptionControl {
        &mut self.boot_idle_preemption
    }

    pub const fn scheduler_running(&self) -> bool {
        self.scheduler_running
    }

    pub const fn selected_runqueue_task_id(&self) -> usize {
        self.selected_runqueue_task_id
    }

    pub const fn schedule_passes(&self) -> usize {
        self.schedule_passes
    }

    pub const fn current_runqueue_resolve_passes(&self) -> usize {
        self.current_runqueue_resolve_passes
    }

    pub const fn pick_next_task_passes(&self) -> usize {
        self.pick_next_task_passes
    }

    pub const fn smp_initialized(&self) -> bool {
        self.smp_initialized
    }

    pub const fn sched_domains_ready(&self) -> bool {
        self.sched_domains_ready
    }

    pub const fn kernel_init_affinity_released(&self) -> bool {
        self.kernel_init_affinity_released
    }

    pub const fn rt_dl_smp_ready(&self) -> bool {
        self.rt_dl_smp_ready
    }

    pub const fn granularity_refreshed(&self) -> bool {
        self.granularity_refreshed
    }

    pub const fn switch_to_passes(&self) -> usize {
        self.switch_to_passes
    }

    pub const fn identity_switch_passes(&self) -> usize {
        self.identity_switch_passes
    }

    pub const fn idle_schedule_passes(&self) -> usize {
        self.idle_schedule_passes
    }

    pub const fn idle_schedule_returned_passes(&self) -> usize {
        self.idle_schedule_returned_passes
    }

    pub const fn idle_schedule_identity_passes(&self) -> usize {
        self.idle_schedule_identity_passes
    }

    pub const fn pick_next_task_exit_prev_ref(&self) -> CurrentTaskRef {
        self.pick_next_task_exit_prev_ref
    }

    pub const fn pick_next_task_exit_next_ref(&self) -> CurrentTaskRef {
        self.pick_next_task_exit_next_ref
    }

    pub const fn pick_next_task_exit_count(&self) -> usize {
        self.pick_next_task_exit_count
    }

    pub const fn switch_to_entry_prev_ref(&self) -> CurrentTaskRef {
        self.switch_to_entry_prev_ref
    }

    pub const fn switch_to_entry_next_ref(&self) -> CurrentTaskRef {
        self.switch_to_entry_next_ref
    }

    pub const fn switch_to_entry_current_ref(&self) -> CurrentTaskRef {
        self.switch_to_entry_current_ref
    }

    pub const fn switch_to_entry_committed_count(&self) -> usize {
        self.switch_to_entry_committed_count
    }

    pub const fn switch_to_entry_count(&self) -> usize {
        self.switch_to_entry_count
    }

    pub fn preset(
        &mut self,
        cpu_group: &CpuGroup,
        cpu_id_map: &CpuIdMap,
        per_cpu_storage: &PerCpuStorage,
        static_branch: &StaticBranch,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.state() != State::Ready
            || cpu_id_map.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
            || static_branch.state() != State::Ready
        {
            return self.failed_preset();
        }

        self.default_root_domain.setup(cpu_group)?;
        self.bit_wait_queue_table.preset()?;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::SchedulerPrepared,
        )
    }

    pub fn setup(
        &mut self,
        cpu_group: &CpuGroup,
        cpu_id_map: &CpuIdMap,
        per_cpu_storage: &PerCpuStorage,
        init_task: &InitTask,
        init_mm: &InitMm,
        current_task_slot: &mut CurrentTaskSlot,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || self.default_root_domain.state() != State::Ready
            || self.bit_wait_queue_table.state() != State::Prepared
            || cpu_group.state() != State::Ready
            || cpu_id_map.state() != State::Ready
            || per_cpu_storage.state() != State::Ready
            || init_task.state() != State::Online
            || init_mm.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.boot_runqueue.setup(
            cpu_group,
            cpu_id_map,
            per_cpu_storage,
            &self.default_root_domain,
        )?;
        self.boot_idle_task
            .setup(init_task, init_mm, &self.boot_runqueue, cpu_group)?;
        self.boot_idle_preemption.setup_disabled(init_task)?;
        current_task_slot.set_current_boot_idle()?;
        if !self.setup_facts_hold(cpu_group) {
            return self.failed_setup();
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::SchedulerReady,
        )
    }

    pub fn enable(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.boot_idle_task.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.scheduler_running = true;
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::SchedulerOnline,
        )
    }

    pub fn schedule(
        &mut self,
        local_interrupt: &mut LocalInterruptControl,
        current_task_slot: &mut CurrentTaskSlot,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.boot_runqueue.curr_task_id() != self.boot_idle_task.task_id()
            || self.boot_runqueue.idle_task_id() != self.boot_idle_task.task_id()
            || local_interrupt.state() != State::Ready
            || current_task_slot.state() != State::Ready
            || !current_task_slot.current_is_boot_idle()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        let prev_ref = current_task_slot.current();

        local_interrupt.save_and_disable()?;
        let current_rq = self.resolve_current_runqueue_ref(prev_ref)?;
        let next_ref = self.pick_next_task(current_rq, prev_ref)?;
        self.switch_to(prev_ref, next_ref, current_task_slot)?;
        self.schedule_passes = self.schedule_passes.wrapping_add(1);
        crate::trace::checkpoint(Checkpoint::SchedulerSchedule);
        local_interrupt.restore()?;
        Ok(())
    }

    pub fn schedule_idle(
        &mut self,
        local_interrupt: &mut LocalInterruptControl,
        current_task_slot: &mut CurrentTaskSlot,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || current_task_slot.state() != State::Ready
            || !current_task_slot.current_is_boot_idle()
        {
            return Err(self.failed_schedule_condition());
        }

        self.schedule(local_interrupt, current_task_slot)?;
        self.idle_schedule_passes = self.idle_schedule_passes.wrapping_add(1);
        self.idle_schedule_returned_passes = self.idle_schedule_returned_passes.wrapping_add(1);
        if current_task_slot.current_is_boot_idle() {
            self.idle_schedule_identity_passes = self.idle_schedule_identity_passes.wrapping_add(1);
        }
        Ok(())
    }

    fn resolve_current_runqueue_ref(
        &mut self,
        current_task_ref: CurrentTaskRef,
    ) -> Result<CurrentRunQueueRef, EventError> {
        if !matches!(current_task_ref, CurrentTaskRef::BootIdle)
            || self.boot_idle_task.cpu_id() != self.boot_runqueue.cpu_id()
            || self.boot_runqueue.curr_task_id() != self.boot_idle_task.task_id()
        {
            return Err(self.failed_schedule_condition());
        }

        self.current_runqueue_resolve_passes = self.current_runqueue_resolve_passes.wrapping_add(1);
        Ok(CurrentRunQueueRef::BootRunQueue)
    }

    fn pick_next_task(
        &mut self,
        current_rq: CurrentRunQueueRef,
        prev_ref: CurrentTaskRef,
    ) -> Result<CurrentTaskRef, EventError> {
        if !matches!(current_rq, CurrentRunQueueRef::BootRunQueue)
            || !matches!(prev_ref, CurrentTaskRef::BootIdle)
            || self.boot_runqueue.curr_task_id() != self.boot_idle_task.task_id()
            || self.boot_runqueue.idle_task_id() != self.boot_idle_task.task_id()
        {
            return Err(self.failed_schedule_condition());
        }

        let next_ref = CurrentTaskRef::BootIdle;
        self.pick_next_task_passes = self.pick_next_task_passes.wrapping_add(1);
        self.pick_next_task_exit_prev_ref = prev_ref;
        self.pick_next_task_exit_next_ref = next_ref;
        self.pick_next_task_exit_count = self.pick_next_task_exit_count.wrapping_add(1);
        crate::trace::checkpoint(Checkpoint::SchedulerPickNextTaskExit);
        Ok(next_ref)
    }

    fn failed_schedule_condition(&self) -> EventError {
        EventError::failed(
            EventErrorCode::ConditionFailed,
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Online,
            State::Online,
        )
    }

    fn switch_to(
        &mut self,
        prev_ref: CurrentTaskRef,
        next_ref: CurrentTaskRef,
        current_task_slot: &mut CurrentTaskSlot,
    ) -> EventResult {
        if !matches!(prev_ref, CurrentTaskRef::BootIdle)
            || !matches!(next_ref, CurrentTaskRef::BootIdle)
            || self.boot_runqueue.curr_task_id() != self.boot_idle_task.task_id()
            || self.boot_runqueue.idle_task_id() != self.boot_idle_task.task_id()
            || current_task_slot.state() != State::Ready
            || current_task_slot.current() != prev_ref
        {
            return self.failed_switch_to();
        }

        self.switch_to_entry_prev_ref = prev_ref;
        self.switch_to_entry_next_ref = next_ref;
        self.switch_to_entry_current_ref = current_task_slot.current();
        self.switch_to_entry_committed_count = current_task_slot.switch_committed_count();
        self.switch_to_entry_count = self.switch_to_entry_count.wrapping_add(1);
        crate::trace::checkpoint(Checkpoint::SchedulerSwitchToEntry);
        self.boot_idle_task.save_core_context()?;
        self.boot_idle_task.restore_core_context()?;
        current_task_slot.commit_switch_to(next_ref)?;
        trace_switch_to(prev_ref, next_ref, current_task_slot.current());
        self.switch_to_passes = self.switch_to_passes.wrapping_add(1);
        if prev_ref == next_ref {
            self.identity_switch_passes = self.identity_switch_passes.wrapping_add(1);
        }
        Ok(())
    }

    fn failed_switch_to(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Online,
            State::Online,
        )
    }

    pub fn select_boot_runqueue_for_task(&mut self, task_id: usize) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.boot_runqueue.state() != State::Ready
            || task_id == usize::MAX
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.selected_runqueue_task_id = task_id;
        Ok(())
    }

    pub fn enqueue_task_on_boot_runqueue(&mut self, task_id: usize) -> EventResult {
        if self.selected_runqueue_task_id != task_id {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.boot_runqueue.enqueue_task(task_id)
    }

    pub fn enable_smp(
        &mut self,
        kernel_init_task: &mut KernelInitTask,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || kernel_init_task.state() != State::Online
            || cpu_group.state() != State::Ready
            || !cpu_group.secondary_cpus_online()
            || !cpu_group.smp_concurrency_open()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        if !kernel_init_task.release_boot_cpu_affinity(cpu_group) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.smp_initialized = true;
        self.sched_domains_ready = true;
        self.kernel_init_affinity_released = true;
        self.rt_dl_smp_ready = true;
        self.granularity_refreshed = true;
        crate::trace::checkpoint(Checkpoint::SchedulerSmpReady);
        Ok(())
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

    fn setup_facts_hold(&self, cpu_group: &CpuGroup) -> bool {
        self.boot_runqueue.state() == State::Ready
            && self.boot_idle_task.state() == State::Ready
            && self.boot_idle_preemption.state() == State::Ready
            && self.boot_idle_preemption.disabled()
            && self.boot_runqueue.cpu_id() == 0
            && self.boot_runqueue.boot_hartid() == cpu_group.boot_hartid()
            && self.boot_runqueue.curr_task_id() == self.boot_idle_task.task_id()
            && self.boot_runqueue.idle_task_id() == self.boot_idle_task.task_id()
    }
}

fn trace_switch_to(
    prev_ref: CurrentTaskRef,
    next_ref: CurrentTaskRef,
    current_after: CurrentTaskRef,
) {
    #[cfg(checkpoint_sbi_char)]
    {
        crate::arch::riscv64::sbi::putstr("trace: Scheduler.SwitchTo prev=");
        crate::arch::riscv64::sbi::putstr(prev_ref.name());
        crate::arch::riscv64::sbi::putstr(" next=");
        crate::arch::riscv64::sbi::putstr(next_ref.name());
        crate::arch::riscv64::sbi::putstr(" current_after=");
        crate::arch::riscv64::sbi::putstr(current_after.name());
        crate::arch::riscv64::sbi::putchar(b'\n');
    }

    #[cfg(not(checkpoint_sbi_char))]
    {
        let _ = (prev_ref, next_ref, current_after);
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum CurrentRunQueueRef {
    BootRunQueue,
}

pub struct DefaultSchedRootDomain {
    lifecycle: Lifecycle,
    possible_cpu_count: usize,
    smp_topology_deferred: bool,
}

impl DefaultSchedRootDomain {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            possible_cpu_count: 0,
            smp_topology_deferred: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn possible_cpu_count(&self) -> usize {
        self.possible_cpu_count
    }

    pub const fn smp_topology_deferred(&self) -> bool {
        self.smp_topology_deferred
    }

    fn setup(&mut self, cpu_group: &CpuGroup) -> EventResult {
        if self.lifecycle.state() != State::Base || cpu_group.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.possible_cpu_count = cpu_group.possible_cpu_count();
        self.smp_topology_deferred = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::DefaultSchedRootDomainReady,
        )
    }
}

pub struct BitWaitQueueTable {
    lifecycle: Lifecycle,
    bucket_count: usize,
}

impl BitWaitQueueTable {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            bucket_count: 0,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn bucket_count(&self) -> usize {
        self.bucket_count
    }

    fn preset(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Base {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.bucket_count = 256;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::BitWaitQueueTablePrepared,
        )
    }
}

pub struct BootRunQueue {
    lifecycle: Lifecycle,
    cpu_id: usize,
    boot_hartid: usize,
    curr_task_id: usize,
    idle_task_id: usize,
    cfs_ready: bool,
    rt_ready: bool,
    dl_ready: bool,
    attached_to_root_domain: bool,
    balance_push_enabled: bool,
    enqueued_task_id: usize,
    kernel_init_task_enqueued: bool,
    kthreadd_task_enqueued: bool,
}

impl BootRunQueue {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            cpu_id: usize::MAX,
            boot_hartid: usize::MAX,
            curr_task_id: usize::MAX,
            idle_task_id: usize::MAX,
            cfs_ready: false,
            rt_ready: false,
            dl_ready: false,
            attached_to_root_domain: false,
            balance_push_enabled: true,
            enqueued_task_id: usize::MAX,
            kernel_init_task_enqueued: false,
            kthreadd_task_enqueued: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn cpu_id(&self) -> usize {
        self.cpu_id
    }

    pub const fn boot_hartid(&self) -> usize {
        self.boot_hartid
    }

    pub const fn curr_task_id(&self) -> usize {
        self.curr_task_id
    }

    pub const fn idle_task_id(&self) -> usize {
        self.idle_task_id
    }

    pub const fn class_queues_ready(&self) -> bool {
        self.cfs_ready && self.rt_ready && self.dl_ready
    }

    pub const fn attached_to_root_domain(&self) -> bool {
        self.attached_to_root_domain
    }

    pub const fn balance_push_enabled(&self) -> bool {
        self.balance_push_enabled
    }

    pub const fn contains_task(&self, task_id: usize) -> bool {
        (self.kernel_init_task_enqueued && task_id == crate::objects::rest_init::KERNEL_INIT_PID)
            || (self.kthreadd_task_enqueued && task_id == crate::objects::rest_init::KTHREADD_PID)
    }

    pub const fn task_count(&self) -> usize {
        self.kernel_init_task_enqueued as usize + self.kthreadd_task_enqueued as usize
    }

    fn setup(
        &mut self,
        cpu_group: &CpuGroup,
        cpu_id_map: &CpuIdMap,
        _per_cpu_storage: &PerCpuStorage,
        root_domain: &DefaultSchedRootDomain,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.state() != State::Ready
            || cpu_id_map.state() != State::Ready
            || root_domain.state() != State::Ready
        {
            return self.failed_setup();
        }

        let Some(boot_entry) = cpu_id_map.entry(0) else {
            return self.failed_setup();
        };
        if boot_entry.hartid() != cpu_group.boot_hartid() {
            return self.failed_setup();
        }

        self.cpu_id = boot_entry.logical_id();
        self.boot_hartid = boot_entry.hartid();
        self.curr_task_id = 0;
        self.idle_task_id = 0;
        self.cfs_ready = true;
        self.rt_ready = true;
        self.dl_ready = true;
        self.attached_to_root_domain = true;
        self.balance_push_enabled = false;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::BootRunQueueReady,
        )
    }

    fn enqueue_task(&mut self, task_id: usize) -> EventResult {
        if self.lifecycle.state() != State::Ready || task_id == usize::MAX {
            return self.failed_setup();
        }

        self.enqueued_task_id = task_id;
        if task_id == crate::objects::rest_init::KERNEL_INIT_PID {
            self.kernel_init_task_enqueued = true;
        }
        if task_id == crate::objects::rest_init::KTHREADD_PID {
            self.kthreadd_task_enqueued = true;
        }
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

pub struct BootIdleTask {
    lifecycle: Lifecycle,
    task_id: usize,
    cpu: TaskCpuState,
    thread_context: TaskThreadContext,
    uses_current_init_task: bool,
    lazy_tlb_mm_ready: bool,
    no_set_affinity: bool,
}

impl BootIdleTask {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            task_id: usize::MAX,
            cpu: TaskCpuState::new(),
            thread_context: TaskThreadContext::new(),
            uses_current_init_task: false,
            lazy_tlb_mm_ready: false,
            no_set_affinity: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn task_id(&self) -> usize {
        self.task_id
    }

    pub const fn cpu_id(&self) -> usize {
        self.cpu.cpu_id()
    }

    pub const fn uses_current_init_task(&self) -> bool {
        self.uses_current_init_task
    }

    pub const fn lazy_tlb_mm_ready(&self) -> bool {
        self.lazy_tlb_mm_ready
    }

    pub const fn no_set_affinity(&self) -> bool {
        self.no_set_affinity
    }

    pub const fn thread_context(&self) -> &TaskThreadContext {
        &self.thread_context
    }

    fn setup(
        &mut self,
        init_task: &InitTask,
        init_mm: &InitMm,
        boot_runqueue: &BootRunQueue,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || init_task.state() != State::Online
            || init_mm.state() != State::Ready
            || boot_runqueue.state() != State::Ready
            || boot_runqueue.boot_hartid() != cpu_group.boot_hartid()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.task_id = boot_runqueue.idle_task_id();
        if !self.cpu.set_task_cpu(boot_runqueue.cpu_id()) {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }
        self.thread_context.setup_boot_idle();
        self.uses_current_init_task = true;
        self.lazy_tlb_mm_ready = true;
        self.no_set_affinity = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::BootIdleTaskReady,
        )
    }

    fn save_core_context(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || !self.thread_context.core_register_set() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.thread_context.save_core();
        Ok(())
    }

    fn restore_core_context(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || !self.thread_context.core_register_set() {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.thread_context.restore_core();
        Ok(())
    }
}

pub struct TaskThreadContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
    core_register_set: bool,
    core_saved_count: usize,
    core_restored_count: usize,
}

impl TaskThreadContext {
    const fn new() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
            core_register_set: false,
            core_saved_count: 0,
            core_restored_count: 0,
        }
    }

    pub const fn core_register_set(&self) -> bool {
        self.core_register_set
    }

    pub const fn core_saved_count(&self) -> usize {
        self.core_saved_count
    }

    pub const fn core_restored_count(&self) -> usize {
        self.core_restored_count
    }

    fn setup_boot_idle(&mut self) {
        self.ra = 0;
        self.sp = 0;
        self.s = [0; 12];
        self.core_register_set = true;
    }

    fn save_core(&mut self) {
        self.core_saved_count = self.core_saved_count.wrapping_add(1);
    }

    fn restore_core(&mut self) {
        self.core_restored_count = self.core_restored_count.wrapping_add(1);
    }
}
