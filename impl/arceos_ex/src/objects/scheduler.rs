use super::{
    cpu_group::CpuGroup,
    cpu_id_map::CpuIdMap,
    init_mm::InitMm,
    init_task::InitTask,
    per_cpu_storage::PerCpuStorage,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
    static_branch::StaticBranch,
};
use crate::trace::Checkpoint;

pub struct Scheduler {
    lifecycle: Lifecycle,
    default_root_domain: DefaultSchedRootDomain,
    bit_wait_queue_table: BitWaitQueueTable,
    boot_runqueue: BootRunQueue,
    boot_idle_task: BootIdleTask,
    scheduler_running: bool,
    preempt_disabled_passes: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            default_root_domain: DefaultSchedRootDomain::new(),
            bit_wait_queue_table: BitWaitQueueTable::new(),
            boot_runqueue: BootRunQueue::new(),
            boot_idle_task: BootIdleTask::new(),
            scheduler_running: false,
            preempt_disabled_passes: 0,
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

    pub const fn scheduler_running(&self) -> bool {
        self.scheduler_running
    }

    pub const fn preempt_disabled_passes(&self) -> usize {
        self.preempt_disabled_passes
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

    pub fn schedule_preempt_disabled(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.boot_runqueue.curr_task_id() != self.boot_idle_task.task_id()
            || self.boot_runqueue.idle_task_id() != self.boot_idle_task.task_id()
            || crate::arch::riscv64::csr::supervisor_interrupts_enabled()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.preempt_disabled_passes = self.preempt_disabled_passes.wrapping_add(1);
        crate::trace::checkpoint(Checkpoint::SchedulerPreemptDisabledPass);
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
            && self.boot_runqueue.cpu_id() == 0
            && self.boot_runqueue.boot_hartid() == cpu_group.boot_hartid()
            && self.boot_runqueue.curr_task_id() == self.boot_idle_task.task_id()
            && self.boot_runqueue.idle_task_id() == self.boot_idle_task.task_id()
    }
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
    cpu_id: usize,
    uses_current_init_task: bool,
    lazy_tlb_mm_ready: bool,
    no_set_affinity: bool,
}

impl BootIdleTask {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            task_id: usize::MAX,
            cpu_id: usize::MAX,
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
        self.cpu_id
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
        self.cpu_id = boot_runqueue.cpu_id();
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
}
