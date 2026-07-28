use super::{
    boot_task::BootTask,
    cpu::{CpuRef, MAX_CPUS},
    cpu_control::{LocalInterruptControl, PreemptionControl, RawSpinLock, RcuReadSide},
    cpu_group::{CpuGroup, CurrentCpu},
    current_task::CurrentTask,
    default_sched_root_domain::DefaultSchedRootDomain,
    init_mm::InitMm,
    mutex::{Mutex, MutexLockOutcome, MutexOwner},
    per_cpu_storage::PerCpuStorage,
    rest_init::{KernelInitFlow, KernelInitTask, KthreaddFlow, KthreaddTask},
    state::{
        EventError, EventErrorCode, EventResult, Lifecycle, LifecycleEvent, State, failed_condition,
    },
    static_branch::StaticBranch,
    task::{Task, TaskEntry, TaskKind, TaskRef},
    task_flow::{TaskFlow, TaskFlowRef},
    user_boot::{USER_CHILD_PID, UserAppFlow, UserTaskSet},
};
use crate::arch::riscv64::task_switch::{self, TaskSwitchContext};
use crate::checkpoint::Checkpoint;
use crate::flows::boot_idle_flow::BootIdleFlow;

const SMOKE_SCHEDULER_TASK_ID: usize = 1001;
const SMOKE_MUTEX_TASK_ID: usize = 1002;
const SMOKE_RWSEM_TASK_ID: usize = 1003;
const SMOKE_RWLOCK_TASK_ID: usize = 1004;
const SMOKE_SCHEDULER_STACK_WORDS: usize = 512;
const BIT_WAIT_TABLE_SIZE: usize = 256;

// Scheduler diagnostic views and local-subject helpers are consumed by smoke/KUnit cases.
#[cfg_attr(not(app_smoke), allow(dead_code))]
pub struct Scheduler {
    lifecycle: Lifecycle,
    sched_domains_mutex: Mutex,
    default_root_domain: DefaultSchedRootDomain,
    bit_wait_queue_table: BitWaitQueueTable,
    boot_idle_rcu_read_side: RcuReadSide,
    boot_runqueue: BootRunQueue,
    boot_init_preemption: PreemptionControl,
    boot_idle_setup_state: BootIdleSetupState,
    boot_idle_preemption: PreemptionControl,
    cpu_runqueues: [CpuRunQueueMetadata; MAX_CPUS],
    cpu_runqueue_count: usize,
    scheduler_running: bool,
    task_stack_switching_online: bool,
    selected_runqueue_task_id: usize,
    schedule_passes: usize,
    current_runqueue_resolve_passes: usize,
    pick_next_task_passes: usize,
    smp_initialized: bool,
    sched_domains_ready: bool,
    sched_domains_mutex_guard_used: bool,
    smp_cpu_masks_stable: bool,
    kernel_init_affinity_released: bool,
    rt_dl_smp_ready: bool,
    granularity_refreshed: bool,
    switch_to_passes: usize,
    identity_switch_passes: usize,
    idle_schedule_passes: usize,
    idle_schedule_returned_passes: usize,
    idle_schedule_identity_passes: usize,
    schedule_preemption_disable_count: usize,
    schedule_preemption_enable_no_resched_count: usize,
    scheduler_rcu_context_switch_count: usize,
    scheduler_rq_lock_mb_after_spinlock_count: usize,
    scheduler_rq_clock_update_count: usize,
    scheduler_need_resched_clear_count: usize,
    scheduler_rq_curr_publish_rcu_count: usize,
    scheduler_trace_sched_switch_count: usize,
    scheduler_prepare_task_switch_count: usize,
    scheduler_finish_task_switch_count: usize,
    scheduler_finish_released_rq_lock_count: usize,
    scheduler_finish_preempt_count_restore_count: usize,
    task_switch_register_sentinel_passes: usize,
    task_switch_tp_identity_passes: usize,
    physical_switch_pending: bool,
    scheduler_switch_mm_or_lazy_tlb_deferred: bool,
    scheduler_membarrier_switch_barrier_deferred: bool,
    pick_next_task_exit_prev_ref: TaskRef,
    pick_next_task_exit_next_ref: TaskRef,
    pick_next_task_exit_count: usize,
    switch_to_entry_prev_ref: TaskRef,
    switch_to_entry_next_ref: TaskRef,
    switch_to_entry_current_ref: TaskRef,
    switch_to_entry_count: usize,
    switch_to_exit_prev_ref: TaskRef,
    switch_to_exit_next_ref: TaskRef,
    switch_to_exit_current_ref: TaskRef,
    switch_to_exit_count: usize,
    schedule_exit_prev_ref: TaskRef,
    schedule_exit_next_ref: TaskRef,
    schedule_exit_current_ref: TaskRef,
    schedule_exit_saved_interrupt_count: usize,
    schedule_exit_restored_interrupt_count: usize,
    schedule_exit_count: usize,
    kernel_init_stack_switch_started_count: usize,
    kernel_init_stack_switch_returned_count: usize,
    smoke_scheduler_task: SmokeSchedulerTask,
    smoke_mutex_task: SmokeSchedulerTask,
    smoke_rwsem_task: SmokeSchedulerTask,
    smoke_rwlock_task: SmokeSchedulerTask,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl Scheduler {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            sched_domains_mutex: Mutex::new_static(),
            default_root_domain: DefaultSchedRootDomain::new(),
            bit_wait_queue_table: BitWaitQueueTable::new(),
            boot_idle_rcu_read_side: RcuReadSide::new(),
            boot_runqueue: BootRunQueue::new(),
            boot_init_preemption: PreemptionControl::new(),
            boot_idle_setup_state: BootIdleSetupState::new(),
            boot_idle_preemption: PreemptionControl::new(),
            cpu_runqueues: [const { CpuRunQueueMetadata::invalid() }; MAX_CPUS],
            cpu_runqueue_count: 0,
            scheduler_running: false,
            task_stack_switching_online: false,
            selected_runqueue_task_id: usize::MAX,
            schedule_passes: 0,
            current_runqueue_resolve_passes: 0,
            pick_next_task_passes: 0,
            smp_initialized: false,
            sched_domains_ready: false,
            sched_domains_mutex_guard_used: false,
            smp_cpu_masks_stable: false,
            kernel_init_affinity_released: false,
            rt_dl_smp_ready: false,
            granularity_refreshed: false,
            switch_to_passes: 0,
            identity_switch_passes: 0,
            idle_schedule_passes: 0,
            idle_schedule_returned_passes: 0,
            idle_schedule_identity_passes: 0,
            schedule_preemption_disable_count: 0,
            schedule_preemption_enable_no_resched_count: 0,
            scheduler_rcu_context_switch_count: 0,
            scheduler_rq_lock_mb_after_spinlock_count: 0,
            scheduler_rq_clock_update_count: 0,
            scheduler_need_resched_clear_count: 0,
            scheduler_rq_curr_publish_rcu_count: 0,
            scheduler_trace_sched_switch_count: 0,
            scheduler_prepare_task_switch_count: 0,
            scheduler_finish_task_switch_count: 0,
            scheduler_finish_released_rq_lock_count: 0,
            scheduler_finish_preempt_count_restore_count: 0,
            task_switch_register_sentinel_passes: 0,
            task_switch_tp_identity_passes: 0,
            physical_switch_pending: false,
            scheduler_switch_mm_or_lazy_tlb_deferred: true,
            scheduler_membarrier_switch_barrier_deferred: true,
            pick_next_task_exit_prev_ref: TaskRef::NONE,
            pick_next_task_exit_next_ref: TaskRef::NONE,
            pick_next_task_exit_count: 0,
            switch_to_entry_prev_ref: TaskRef::NONE,
            switch_to_entry_next_ref: TaskRef::NONE,
            switch_to_entry_current_ref: TaskRef::NONE,
            switch_to_entry_count: 0,
            switch_to_exit_prev_ref: TaskRef::NONE,
            switch_to_exit_next_ref: TaskRef::NONE,
            switch_to_exit_current_ref: TaskRef::NONE,
            switch_to_exit_count: 0,
            schedule_exit_prev_ref: TaskRef::NONE,
            schedule_exit_next_ref: TaskRef::NONE,
            schedule_exit_current_ref: TaskRef::NONE,
            schedule_exit_saved_interrupt_count: 0,
            schedule_exit_restored_interrupt_count: 0,
            schedule_exit_count: 0,
            kernel_init_stack_switch_started_count: 0,
            kernel_init_stack_switch_returned_count: 0,
            smoke_scheduler_task: SmokeSchedulerTask::new(SMOKE_SCHEDULER_TASK_ID),
            smoke_mutex_task: SmokeSchedulerTask::new(SMOKE_MUTEX_TASK_ID),
            smoke_rwsem_task: SmokeSchedulerTask::new(SMOKE_RWSEM_TASK_ID),
            smoke_rwlock_task: SmokeSchedulerTask::new(SMOKE_RWLOCK_TASK_ID),
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn kernel_init_stack_switch_started_count(&self) -> usize {
        self.kernel_init_stack_switch_started_count
    }

    // Preserved for stack-switch diagnostics even though normal handoff does not return.
    #[allow(dead_code)]
    pub const fn kernel_init_stack_switch_returned_count(&self) -> usize {
        self.kernel_init_stack_switch_returned_count
    }

    pub const fn sched_domains_mutex(&self) -> &Mutex {
        &self.sched_domains_mutex
    }

    pub const fn default_root_domain(&self) -> &DefaultSchedRootDomain {
        &self.default_root_domain
    }

    pub const fn bit_wait_queue_table(&self) -> &BitWaitQueueTable {
        &self.bit_wait_queue_table
    }

    pub const fn boot_idle_rcu_read_side(&self) -> &RcuReadSide {
        &self.boot_idle_rcu_read_side
    }

    pub fn boot_idle_rcu_read_side_mut(&mut self) -> &mut RcuReadSide {
        &mut self.boot_idle_rcu_read_side
    }

    pub const fn boot_runqueue(&self) -> &BootRunQueue {
        &self.boot_runqueue
    }

    pub const fn boot_runqueue_lock(&self) -> &RawSpinLock {
        self.boot_runqueue.lock()
    }

    pub const fn boot_idle_pi_lock(&self) -> &RawSpinLock {
        self.boot_idle_setup_state.pi_lock()
    }

    pub const fn boot_idle_preemption(&self) -> &PreemptionControl {
        &self.boot_idle_preemption
    }

    pub const fn boot_init_preemption(&self) -> &PreemptionControl {
        &self.boot_init_preemption
    }

    pub const fn cpu_runqueue_count(&self) -> usize {
        self.cpu_runqueue_count
    }

    pub fn cpu_runqueue(&self, logical_id: usize) -> Option<CpuRunQueueView> {
        if logical_id < self.cpu_runqueue_count {
            self.cpu_runqueues[logical_id].view()
        } else {
            None
        }
    }

    pub fn possible_cpu_runqueues_ready(&self, cpu_group: &CpuGroup) -> bool {
        if cpu_group.state() != State::Ready
            || !cpu_group.possible_cpu_boundary_ready()
            || !self
                .default_root_domain
                .covers_cpu_group_possible(cpu_group)
            || self.cpu_runqueue_count == 0
            || self.cpu_runqueue_count != cpu_group.possible_cpu_count()
        {
            return false;
        }

        let mut logical_id = 0usize;
        while logical_id < self.cpu_runqueue_count {
            let Some(runqueue) = self.cpu_runqueue(logical_id) else {
                return false;
            };
            let Some(cpu_ref) = cpu_group.possible_cpu_ref_at(logical_id) else {
                return false;
            };
            if runqueue.state() != State::Ready
                || runqueue.cpu_ref() != cpu_ref
                || runqueue.cpu_id() != logical_id
                || !runqueue.class_queues_ready()
                || !runqueue.attached_to_root_domain()
                || runqueue.balance_push_enabled()
                || !self.default_root_domain.covers_cpu_ref(cpu_ref)
            {
                return false;
            }
            if logical_id == 0 && !self.boot_runqueue_matches_metadata() {
                return false;
            }
            logical_id += 1;
        }

        self.cpu_runqueue(self.cpu_runqueue_count).is_none()
    }

    pub fn boot_runqueue_matches_metadata(&self) -> bool {
        let Some(runqueue) = self.cpu_runqueue(0) else {
            return false;
        };
        self.boot_runqueue.state() == State::Ready
            && runqueue.is_boot_backed()
            && runqueue.cpu_ref() == self.boot_runqueue.cpu_ref()
            && runqueue.cpu_hartid() == self.boot_runqueue.cpu_hartid()
            && runqueue.class_queues_ready() == self.boot_runqueue.class_queues_ready()
            && runqueue.attached_to_root_domain() == self.boot_runqueue.attached_to_root_domain()
            && runqueue.balance_push_enabled() == self.boot_runqueue.balance_push_enabled()
    }

    pub fn boot_cpu_owned_scheduler_view(
        &self,
        cpu_group: &CpuGroup,
    ) -> Option<CpuOwnedSchedulerView> {
        let boot_cpu = cpu_group.boot_cpu()?;
        let runqueue = self.cpu_runqueue(boot_cpu.logical_id())?;
        let idle_task = self.boot_idle_setup_state.view()?;
        if cpu_group.state() != State::Ready
            || !boot_cpu.cpu_ref().is_boot_cpu()
            || !self.boot_runqueue_matches_metadata()
            || !self
                .boot_idle_setup_state
                .is_boot_cpu_idle_task_view(cpu_group, &self.boot_runqueue)
            || runqueue.cpu_ref() != boot_cpu.cpu_ref()
            || runqueue.cpu_hartid() != boot_cpu.hartid()
            || idle_task.cpu_ref() != boot_cpu.cpu_ref()
            || idle_task.cpu_id() != boot_cpu.logical_id()
        {
            return None;
        }

        Some(CpuOwnedSchedulerView {
            cpu_ref: boot_cpu.cpu_ref(),
            cpu_hartid: boot_cpu.hartid(),
            runqueue,
            idle_task,
            runqueue_current_task_id: self.boot_runqueue.curr_task_id(),
            runqueue_idle_task_id: self.boot_runqueue.idle_task_id(),
            runqueue_task_count: self.boot_runqueue.task_count(),
            runqueue_kernel_init_task_enqueued: self
                .boot_runqueue
                .contains_task(crate::objects::rest_init::KERNEL_INIT_PID),
            runqueue_kthreadd_task_enqueued: self
                .boot_runqueue
                .contains_task(crate::objects::rest_init::KTHREADD_PID),
            runqueue_user_child_task_enqueued: self.boot_runqueue.contains_task(USER_CHILD_PID),
            runqueue_smoke_scheduler_task_enqueued: self
                .boot_runqueue
                .contains_task(SMOKE_SCHEDULER_TASK_ID),
            runqueue_smoke_mutex_task_enqueued: self
                .boot_runqueue
                .contains_task(SMOKE_MUTEX_TASK_ID),
            runqueue_smoke_rwsem_task_enqueued: self
                .boot_runqueue
                .contains_task(SMOKE_RWSEM_TASK_ID),
            runqueue_smoke_rwlock_task_enqueued: self
                .boot_runqueue
                .contains_task(SMOKE_RWLOCK_TASK_ID),
        })
    }

    pub fn boot_cpu_owned_scheduler_view_ready(&self, cpu_group: &CpuGroup) -> bool {
        let Some(view) = self.boot_cpu_owned_scheduler_view(cpu_group) else {
            return false;
        };
        view.runqueue().state() == State::Ready
            && view.runqueue().is_boot_backed()
            && view.idle_task().state() == State::Ready
            && view.runqueue_idle_task_matches()
            && view.idle_task().uses_current_init_task()
            && view.idle_task().lazy_tlb_mm_ready()
            && view.idle_task().no_set_affinity()
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

    pub const fn sched_domains_mutex_guard_used(&self) -> bool {
        self.sched_domains_mutex_guard_used
    }

    pub const fn smp_cpu_masks_stable(&self) -> bool {
        self.smp_cpu_masks_stable
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

    pub const fn schedule_preemption_disable_count(&self) -> usize {
        self.schedule_preemption_disable_count
    }

    pub const fn schedule_preemption_enable_no_resched_count(&self) -> usize {
        self.schedule_preemption_enable_no_resched_count
    }

    pub const fn scheduler_rcu_context_switch_count(&self) -> usize {
        self.scheduler_rcu_context_switch_count
    }

    pub const fn scheduler_rq_lock_mb_after_spinlock_count(&self) -> usize {
        self.scheduler_rq_lock_mb_after_spinlock_count
    }

    pub const fn scheduler_rq_clock_update_count(&self) -> usize {
        self.scheduler_rq_clock_update_count
    }

    pub const fn scheduler_need_resched_clear_count(&self) -> usize {
        self.scheduler_need_resched_clear_count
    }

    pub const fn scheduler_rq_curr_publish_rcu_count(&self) -> usize {
        self.scheduler_rq_curr_publish_rcu_count
    }

    pub const fn scheduler_trace_sched_switch_count(&self) -> usize {
        self.scheduler_trace_sched_switch_count
    }

    pub const fn scheduler_prepare_task_switch_count(&self) -> usize {
        self.scheduler_prepare_task_switch_count
    }

    pub const fn scheduler_finish_task_switch_count(&self) -> usize {
        self.scheduler_finish_task_switch_count
    }

    pub const fn scheduler_finish_released_rq_lock_count(&self) -> usize {
        self.scheduler_finish_released_rq_lock_count
    }

    pub const fn scheduler_finish_preempt_count_restore_count(&self) -> usize {
        self.scheduler_finish_preempt_count_restore_count
    }

    pub const fn task_switch_register_sentinel_passes(&self) -> usize {
        self.task_switch_register_sentinel_passes
    }

    pub const fn task_switch_tp_identity_passes(&self) -> usize {
        self.task_switch_tp_identity_passes
    }

    pub const fn scheduler_switch_mm_or_lazy_tlb_deferred(&self) -> bool {
        self.scheduler_switch_mm_or_lazy_tlb_deferred
    }

    pub const fn scheduler_membarrier_switch_barrier_deferred(&self) -> bool {
        self.scheduler_membarrier_switch_barrier_deferred
    }

    pub const fn pick_next_task_exit_prev_ref(&self) -> TaskRef {
        self.pick_next_task_exit_prev_ref
    }

    pub const fn pick_next_task_exit_next_ref(&self) -> TaskRef {
        self.pick_next_task_exit_next_ref
    }

    pub const fn pick_next_task_exit_count(&self) -> usize {
        self.pick_next_task_exit_count
    }

    #[allow(dead_code)]
    pub const fn switch_to_entry_prev_ref(&self) -> TaskRef {
        self.switch_to_entry_prev_ref
    }

    #[allow(dead_code)]
    pub const fn switch_to_entry_next_ref(&self) -> TaskRef {
        self.switch_to_entry_next_ref
    }

    #[allow(dead_code)]
    pub const fn switch_to_entry_current_ref(&self) -> TaskRef {
        self.switch_to_entry_current_ref
    }

    pub const fn switch_to_entry_count(&self) -> usize {
        self.switch_to_entry_count
    }

    #[allow(dead_code)]
    pub const fn switch_to_exit_prev_ref(&self) -> TaskRef {
        self.switch_to_exit_prev_ref
    }

    #[allow(dead_code)]
    pub const fn switch_to_exit_next_ref(&self) -> TaskRef {
        self.switch_to_exit_next_ref
    }

    #[allow(dead_code)]
    pub const fn switch_to_exit_current_ref(&self) -> TaskRef {
        self.switch_to_exit_current_ref
    }

    #[allow(dead_code)]
    pub const fn switch_to_exit_count(&self) -> usize {
        self.switch_to_exit_count
    }

    #[allow(dead_code)]
    pub const fn schedule_exit_prev_ref(&self) -> TaskRef {
        self.schedule_exit_prev_ref
    }

    #[allow(dead_code)]
    pub const fn schedule_exit_next_ref(&self) -> TaskRef {
        self.schedule_exit_next_ref
    }

    #[allow(dead_code)]
    pub const fn schedule_exit_current_ref(&self) -> TaskRef {
        self.schedule_exit_current_ref
    }

    #[allow(dead_code)]
    pub const fn schedule_exit_saved_interrupt_count(&self) -> usize {
        self.schedule_exit_saved_interrupt_count
    }

    #[allow(dead_code)]
    pub const fn schedule_exit_restored_interrupt_count(&self) -> usize {
        self.schedule_exit_restored_interrupt_count
    }

    #[allow(dead_code)]
    pub const fn schedule_exit_count(&self) -> usize {
        self.schedule_exit_count
    }

    pub const fn smoke_scheduler_task(&self) -> &SmokeSchedulerTask {
        &self.smoke_scheduler_task
    }

    pub fn smoke_scheduler_task_mut(&mut self) -> &mut SmokeSchedulerTask {
        &mut self.smoke_scheduler_task
    }

    pub const fn smoke_mutex_task(&self) -> &SmokeSchedulerTask {
        &self.smoke_mutex_task
    }

    pub fn smoke_mutex_task_mut(&mut self) -> &mut SmokeSchedulerTask {
        &mut self.smoke_mutex_task
    }

    pub const fn smoke_rwsem_task(&self) -> &SmokeSchedulerTask {
        &self.smoke_rwsem_task
    }

    pub fn smoke_rwsem_task_mut(&mut self) -> &mut SmokeSchedulerTask {
        &mut self.smoke_rwsem_task
    }

    pub const fn smoke_rwlock_task(&self) -> &SmokeSchedulerTask {
        &self.smoke_rwlock_task
    }

    pub fn smoke_rwlock_task_mut(&mut self) -> &mut SmokeSchedulerTask {
        &mut self.smoke_rwlock_task
    }

    pub(crate) fn current_task_candidate_by_identity(
        &self,
        identity: usize,
    ) -> Option<super::current_task::CurrentTaskCandidate<'_>> {
        let candidates = [
            &self.smoke_scheduler_task,
            &self.smoke_mutex_task,
            &self.smoke_rwsem_task,
            &self.smoke_rwlock_task,
        ];
        let mut index = 0usize;
        while index < candidates.len() {
            if candidates[index].task_ptr() == identity {
                return Some(candidates[index].current_task_candidate());
            }
            index += 1;
        }
        None
    }

    pub(crate) fn current_task_candidate_by_ref(
        &self,
        task_ref: TaskRef,
    ) -> Option<super::current_task::CurrentTaskCandidate<'_>> {
        match task_ref {
            TaskRef::SMOKE_SCHEDULER => Some(self.smoke_scheduler_task.current_task_candidate()),
            TaskRef::SMOKE_MUTEX => Some(self.smoke_mutex_task.current_task_candidate()),
            TaskRef::SMOKE_RWSEM => Some(self.smoke_rwsem_task.current_task_candidate()),
            TaskRef::SMOKE_RWLOCK => Some(self.smoke_rwlock_task.current_task_candidate()),
            _ => None,
        }
    }

    pub fn preset(
        &mut self,
        cpu_group: &CpuGroup,
        per_cpu_storage: &PerCpuStorage,
        static_branch: &StaticBranch,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_group.state() != State::Ready
            || !cpu_group.possible_cpu_boundary_ready()
            || per_cpu_storage.state() != State::Ready
            || static_branch.state() != State::Ready
        {
            return self.failed_preset();
        }

        self.default_root_domain.setup(cpu_group)?;
        self.bit_wait_queue_table.preset()?;
        self.boot_idle_rcu_read_side
            .preset_incomplete_first_slice(Checkpoint::BootIdleRcuReadSidePrepared)?;
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::SchedulerPrepared,
        )
    }

    pub fn setup(
        &mut self,
        cpu_group: &mut CpuGroup,
        per_cpu_storage: &PerCpuStorage,
        boot_task: &mut BootTask,
        init_mm: &InitMm,
    ) -> EventResult {
        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return self.failed_setup();
        };
        let boot_cpu_ref = boot_cpu.cpu_ref();
        let boot_cpu_hartid = boot_cpu.hartid();
        if self.lifecycle.state() != State::Prepared
            || self.default_root_domain.state() != State::Ready
            || self.bit_wait_queue_table.state() != State::Prepared
            || self.boot_idle_rcu_read_side.state() != State::Prepared
            || cpu_group.state() != State::Ready
            || !cpu_group.possible_cpu_boundary_ready()
            || per_cpu_storage.state() != State::Ready
            || boot_task.state() != State::OnCpu
            || init_mm.state() != State::Ready
            || cpu_group
                .boot_cpu_local_interrupt()
                .map(|control| control.state() != State::Ready)
                .unwrap_or(true)
        {
            return self.failed_setup();
        }

        {
            let Some(local_interrupt) = cpu_group.boot_cpu_local_interrupt_mut() else {
                return self.failed_setup();
            };
            self.boot_runqueue.setup(
                boot_cpu_ref,
                boot_cpu_hartid,
                per_cpu_storage,
                &self.default_root_domain,
                &*boot_task,
                local_interrupt,
                &mut self.boot_init_preemption,
            )?;
        }
        self.setup_cpu_runqueue_metadata(cpu_group)?;
        {
            let Some(local_interrupt) = cpu_group.boot_cpu_local_interrupt_mut() else {
                return self.failed_setup();
            };
            self.boot_idle_setup_state.setup(
                boot_task,
                init_mm,
                &mut self.boot_runqueue,
                &mut self.boot_idle_rcu_read_side,
                boot_cpu_ref,
                local_interrupt,
                &mut self.boot_idle_preemption,
            )?;
        }
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
        if self.lifecycle.state() != State::Ready
            || self.boot_idle_setup_state.state() != State::Ready
        {
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

    #[allow(clippy::too_many_arguments)]
    pub fn schedule(
        &mut self,
        current_task: CurrentTask,
        current_cpu: CurrentCpu,
        kernel_init_task: &mut KernelInitTask,
        kernel_init_flow: &mut KernelInitFlow,
        user_app_flow: &UserAppFlow,
        kthreadd_task: &mut KthreaddTask,
        kthreadd_flow: &mut KthreaddFlow,
        boot_idle_flow: &BootIdleFlow,
        user_task_set: &mut UserTaskSet,
        local_interrupt: &mut LocalInterruptControl,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.boot_runqueue.idle_task_id() != self.boot_idle_setup_state.task_id()
            || local_interrupt.state() != State::Ready
            || !current_task.task_ref().is_boot_scheduler_ref()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        let prev_ref = current_task.task_ref();
        let mut next_ref = TaskRef::NONE;

        self.boot_idle_preemption.disable()?;
        self.schedule_preemption_disable_count =
            self.schedule_preemption_disable_count.wrapping_add(1);
        local_interrupt.save_and_disable()?;
        let guarded_result = (|| {
            self.scheduler_rcu_context_switch_count =
                self.scheduler_rcu_context_switch_count.wrapping_add(1);
            let current_rq = self.resolve_current_runqueue_ref(
                current_cpu,
                kernel_init_flow,
                user_app_flow,
                kthreadd_flow,
                boot_idle_flow,
                user_task_set,
                prev_ref,
            )?;
            self.boot_runqueue
                .lock
                .lock_irqsave(local_interrupt, &mut self.boot_idle_preemption)?;
            let runqueue_result = (|| {
                self.scheduler_rq_lock_mb_after_spinlock_count = self
                    .scheduler_rq_lock_mb_after_spinlock_count
                    .wrapping_add(1);
                self.scheduler_rq_clock_update_count =
                    self.scheduler_rq_clock_update_count.wrapping_add(1);
                next_ref = self.pick_next_task(current_rq, prev_ref)?;
                self.scheduler_need_resched_clear_count =
                    self.scheduler_need_resched_clear_count.wrapping_add(1);
                self.scheduler_rq_curr_publish_rcu_count =
                    self.scheduler_rq_curr_publish_rcu_count.wrapping_add(1);
                self.scheduler_trace_sched_switch_count =
                    self.scheduler_trace_sched_switch_count.wrapping_add(1);
                self.switch_to(
                    prev_ref,
                    next_ref,
                    current_task,
                    kernel_init_task,
                    kthreadd_task,
                    user_task_set,
                )?;
                self.schedule_passes = self.schedule_passes.wrapping_add(1);
                crate::checkpoint::checkpoint(Checkpoint::SchedulerSchedule);
                Ok(())
            })();
            let unlock_result = self
                .boot_runqueue
                .lock
                .unlock_irqrestore(local_interrupt, &mut self.boot_idle_preemption);
            if runqueue_result.is_ok() && unlock_result.is_ok() {
                self.scheduler_finish_released_rq_lock_count =
                    self.scheduler_finish_released_rq_lock_count.wrapping_add(1);
            }
            runqueue_result.and(unlock_result)
        })();
        let restore_result = local_interrupt.restore();
        let enable_result = self.boot_idle_preemption.enable_no_resched();
        if guarded_result.is_ok() && restore_result.is_ok() && enable_result.is_ok() {
            self.schedule_preemption_enable_no_resched_count = self
                .schedule_preemption_enable_no_resched_count
                .wrapping_add(1);
            self.scheduler_finish_preempt_count_restore_count = self
                .scheduler_finish_preempt_count_restore_count
                .wrapping_add(1);
        }
        guarded_result.and(restore_result).and(enable_result)?;
        self.schedule_exit_prev_ref = prev_ref;
        self.schedule_exit_next_ref = next_ref;
        self.schedule_exit_saved_interrupt_count = local_interrupt.saved_and_disabled_count();
        self.schedule_exit_restored_interrupt_count = local_interrupt.restored_count();
        let resumed =
            self.cooperative_context_switch(prev_ref, next_ref, kernel_init_task, kthreadd_task)?;
        if resumed {
            self.continue_task_after_switch(
                prev_ref,
                kernel_init_task,
                kernel_init_flow,
                user_app_flow,
                kthreadd_task,
                kthreadd_flow,
                boot_idle_flow,
                user_task_set,
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn schedule_idle(
        &mut self,
        current_task: CurrentTask,
        current_cpu: CurrentCpu,
        kernel_init_task: &mut KernelInitTask,
        kernel_init_flow: &mut KernelInitFlow,
        user_app_flow: &UserAppFlow,
        kthreadd_task: &mut KthreaddTask,
        kthreadd_flow: &mut KthreaddFlow,
        boot_idle_flow: &BootIdleFlow,
        user_task_set: &mut UserTaskSet,
        local_interrupt: &mut LocalInterruptControl,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || !current_task.task_ref().same_identity(TaskRef::BOOT)
        {
            return Err(self.failed_schedule_condition());
        }

        self.schedule(
            current_task,
            current_cpu,
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            boot_idle_flow,
            user_task_set,
            local_interrupt,
        )?;
        self.idle_schedule_passes = self.idle_schedule_passes.wrapping_add(1);
        self.idle_schedule_returned_passes = self.idle_schedule_returned_passes.wrapping_add(1);
        if crate::arch::riscv64::csr::read_tp()
            == BootTask::canonical_task() as *const Task as usize
            && BootTask::canonical_task().state() == State::OnCpu
        {
            self.idle_schedule_identity_passes = self.idle_schedule_identity_passes.wrapping_add(1);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_current_runqueue_ref(
        &mut self,
        current_cpu: CurrentCpu,
        kernel_init_flow: &KernelInitFlow,
        user_app_flow: &UserAppFlow,
        kthreadd_flow: &KthreaddFlow,
        boot_idle_flow: &BootIdleFlow,
        user_task_set: &UserTaskSet,
        current_task_ref: TaskRef,
    ) -> Result<CurrentRunQueueRef, EventError> {
        let Some(cpu_ref) = self.effective_cpu_ref(
            current_task_ref,
            kernel_init_flow,
            user_app_flow,
            kthreadd_flow,
            boot_idle_flow,
            user_task_set,
        ) else {
            return Err(self.failed_schedule_condition());
        };
        if current_cpu.cpu_ref() != cpu_ref {
            return Err(self.failed_schedule_condition());
        }
        let cpu_id = cpu_ref.logical_id();
        let runqueue_ref = CurrentRunQueueRef::cpu_owned(cpu_id);
        if cpu_id != runqueue_ref.cpu_id() {
            return Err(self.failed_schedule_condition());
        }

        self.current_runqueue_resolve_passes = self.current_runqueue_resolve_passes.wrapping_add(1);
        Ok(runqueue_ref)
    }

    pub fn effective_cpu_ref(
        &self,
        current_task_ref: TaskRef,
        kernel_init_flow: &KernelInitFlow,
        user_app_flow: &UserAppFlow,
        kthreadd_flow: &KthreaddFlow,
        boot_idle_flow: &BootIdleFlow,
        user_task_set: &UserTaskSet,
    ) -> Option<CpuRef> {
        if current_task_ref == TaskRef::BOOT {
            boot_idle_flow.cpu_ref()
        } else if current_task_ref == TaskRef::KERNEL_INIT {
            let user_cpu_ref = user_app_flow.cpu_ref();
            if user_cpu_ref.is_none() {
                kernel_init_flow.cpu_ref()
            } else {
                user_cpu_ref
            }
        } else if current_task_ref == TaskRef::KTHREADD {
            kthreadd_flow.cpu_ref()
        } else if current_task_ref == TaskRef::SMOKE_SCHEDULER {
            self.smoke_scheduler_task.cpu_ref()
        } else if current_task_ref == TaskRef::SMOKE_MUTEX {
            self.smoke_mutex_task.cpu_ref()
        } else if current_task_ref == TaskRef::SMOKE_RWSEM {
            self.smoke_rwsem_task.cpu_ref()
        } else if current_task_ref == TaskRef::SMOKE_RWLOCK {
            self.smoke_rwlock_task.cpu_ref()
        } else if current_task_ref.is_user() {
            user_task_set.cpu_ref_for_task(current_task_ref)
        } else {
            None
        }
    }

    fn pick_next_task(
        &mut self,
        current_rq: CurrentRunQueueRef,
        prev_ref: TaskRef,
    ) -> Result<TaskRef, EventError> {
        if !current_rq.matches_cpu_owned_runqueue(self.boot_runqueue.cpu_id())
            || !prev_ref.is_boot_scheduler_ref()
            || self.boot_runqueue.idle_task_id() != self.boot_idle_setup_state.task_id()
            || self.boot_runqueue.task_count() == 0
        {
            return Err(self.failed_schedule_condition());
        }

        let next_ref = self
            .boot_runqueue
            .pick_next_task(current_rq, prev_ref)
            .map_err(|_| self.failed_schedule_condition())?;

        self.pick_next_task_passes = self.pick_next_task_passes.wrapping_add(1);
        self.pick_next_task_exit_prev_ref = prev_ref;
        self.pick_next_task_exit_next_ref = next_ref;
        self.pick_next_task_exit_count = self.pick_next_task_exit_count.wrapping_add(1);
        crate::checkpoint::checkpoint(Checkpoint::SchedulerPickNextTaskExit);
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

    #[allow(clippy::too_many_arguments)]
    fn switch_to(
        &mut self,
        prev_ref: TaskRef,
        next_ref: TaskRef,
        current_task: CurrentTask,
        kernel_init_task: &mut KernelInitTask,
        kthreadd_task: &mut KthreaddTask,
        user_task_set: &mut UserTaskSet,
    ) -> EventResult {
        if !prev_ref.is_boot_scheduler_ref()
            || !next_ref.is_boot_scheduler_ref()
            || self.boot_runqueue.curr_task_id() != self.boot_idle_setup_state.task_id()
            || self.boot_runqueue.idle_task_id() != self.boot_idle_setup_state.task_id()
            || current_task.task_ref() != prev_ref
        {
            return self.failed_switch_to();
        }

        if prev_ref == next_ref {
            self.identity_switch_passes = self.identity_switch_passes.wrapping_add(1);
            return Ok(());
        }

        if !self.task_switch_out_ready(prev_ref, kernel_init_task, kthreadd_task, user_task_set)
            || !self.task_switch_in_ready(next_ref, kernel_init_task, kthreadd_task, user_task_set)
        {
            return self.failed_switch_to();
        }

        self.switch_to_entry_prev_ref = prev_ref;
        self.switch_to_entry_next_ref = next_ref;
        self.switch_to_entry_current_ref = current_task.task_ref();
        self.switch_to_entry_count = self.switch_to_entry_count.wrapping_add(1);
        self.scheduler_prepare_task_switch_count =
            self.scheduler_prepare_task_switch_count.wrapping_add(1);
        self.physical_switch_pending = true;
        crate::checkpoint::checkpoint(Checkpoint::SchedulerSwitchToEntry);
        trace_switch_to(prev_ref, next_ref, current_task.task_ref());
        self.switch_to_passes = self.switch_to_passes.wrapping_add(1);
        Ok(())
    }

    fn task_switch_out_ready(
        &self,
        task_ref: TaskRef,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        user_task_set: &UserTaskSet,
    ) -> bool {
        match task_ref {
            TaskRef::BOOT => BootTask::canonical_task().switch_out_ready(),
            TaskRef::KERNEL_INIT => {
                kernel_init_task.task().switch_out_ready()
                    || kernel_init_task.task().terminal_switch_out_ready()
            }
            TaskRef::KTHREADD => {
                kthreadd_task.task().switch_out_ready()
                    || kthreadd_task.task().terminal_switch_out_ready()
            }
            TaskRef::SMOKE_SCHEDULER => self.smoke_scheduler_task.task.switch_out_ready(),
            TaskRef::SMOKE_MUTEX => self.smoke_mutex_task.task.switch_out_ready(),
            TaskRef::SMOKE_RWSEM => self.smoke_rwsem_task.task.switch_out_ready(),
            TaskRef::SMOKE_RWLOCK => self.smoke_rwlock_task.task.switch_out_ready(),
            _ if task_ref.is_user() => user_task_set.task_switch_out_ready(task_ref),
            _ => false,
        }
    }

    fn task_switch_in_ready(
        &self,
        task_ref: TaskRef,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        user_task_set: &UserTaskSet,
    ) -> bool {
        match task_ref {
            TaskRef::BOOT => BootTask::canonical_task().switch_in_ready(),
            TaskRef::KERNEL_INIT => kernel_init_task.task().switch_in_ready(),
            TaskRef::KTHREADD => kthreadd_task.task().switch_in_ready(),
            TaskRef::SMOKE_SCHEDULER => self.smoke_scheduler_task.task.switch_in_ready(),
            TaskRef::SMOKE_MUTEX => self.smoke_mutex_task.task.switch_in_ready(),
            TaskRef::SMOKE_RWSEM => self.smoke_rwsem_task.task.switch_in_ready(),
            TaskRef::SMOKE_RWLOCK => self.smoke_rwlock_task.task.switch_in_ready(),
            _ if task_ref.is_user() => user_task_set.task_switch_in_ready(task_ref),
            _ => false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_simulated_task_switch(
        &mut self,
        prev_ref: TaskRef,
        next_ref: TaskRef,
        current_task: CurrentTask,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        user_task_set: &UserTaskSet,
    ) -> EventResult {
        if prev_ref == next_ref {
            self.identity_switch_passes = self.identity_switch_passes.wrapping_add(1);
            return Ok(());
        }
        if current_task.task_ref() != prev_ref
            || !self.task_switch_out_ready(prev_ref, kernel_init_task, kthreadd_task, user_task_set)
            || !self.task_switch_in_ready(next_ref, kernel_init_task, kthreadd_task, user_task_set)
        {
            return self.failed_switch_to();
        }
        self.switch_to_entry_prev_ref = prev_ref;
        self.switch_to_entry_next_ref = next_ref;
        self.switch_to_entry_current_ref = current_task.task_ref();
        self.switch_to_entry_count = self.switch_to_entry_count.wrapping_add(1);
        self.scheduler_prepare_task_switch_count =
            self.scheduler_prepare_task_switch_count.wrapping_add(1);
        self.physical_switch_pending = false;
        crate::checkpoint::checkpoint(Checkpoint::SchedulerSwitchToEntry);
        Ok(())
    }

    pub(crate) fn suspend_task(
        &mut self,
        task_ref: TaskRef,
        kernel_init_task: &mut KernelInitTask,
        kthreadd_task: &mut KthreaddTask,
        user_task_set: &mut UserTaskSet,
    ) -> EventResult {
        match task_ref {
            TaskRef::BOOT => BootTask::suspend_canonical(),
            TaskRef::KERNEL_INIT => {
                if kernel_init_task.task().active_flow().is_valid() {
                    kernel_init_task.suspend_from_cpu()
                } else {
                    kernel_init_task.task_mut().disable()
                }
            }
            TaskRef::KTHREADD => {
                if kthreadd_task.task().active_flow().is_valid() {
                    kthreadd_task.suspend_from_cpu()
                } else {
                    kthreadd_task.task_mut().disable()
                }
            }
            TaskRef::SMOKE_SCHEDULER => self.smoke_scheduler_task.suspend_from_cpu(),
            TaskRef::SMOKE_MUTEX => self.smoke_mutex_task.suspend_from_cpu(),
            TaskRef::SMOKE_RWSEM => self.smoke_rwsem_task.suspend_from_cpu(),
            TaskRef::SMOKE_RWLOCK => self.smoke_rwlock_task.suspend_from_cpu(),
            _ if task_ref.is_user() => user_task_set.suspend_task(task_ref),
            _ => self.failed_switch_to(),
        }
    }

    /// Finish the Task.Continue signal only after execution is running on the
    /// selected task's entry/resume stack. Task and Flow state are committed
    /// here; the caller must resolve CurrentTask again before observing exit.
    #[allow(clippy::too_many_arguments)]
    pub fn continue_task_after_switch(
        &mut self,
        task_ref: TaskRef,
        kernel_init_task: &mut KernelInitTask,
        kernel_init_flow: &mut KernelInitFlow,
        user_app_flow: &UserAppFlow,
        kthreadd_task: &mut KthreaddTask,
        kthreadd_flow: &mut KthreaddFlow,
        boot_idle_flow: &BootIdleFlow,
        user_task_set: &mut UserTaskSet,
    ) -> EventResult {
        let prev_ref = self.switch_to_entry_prev_ref;
        if prev_ref == task_ref || self.switch_to_entry_next_ref != task_ref {
            return self.failed_switch_to();
        }

        let Some(expected_task_identity) =
            self.task_identity_ptr(task_ref, kernel_init_task, kthreadd_task, user_task_set)
        else {
            return self.failed_switch_to();
        };
        if crate::arch::riscv64::csr::read_tp() != expected_task_identity {
            return self.failed_switch_to();
        }
        self.task_switch_tp_identity_passes = self.task_switch_tp_identity_passes.wrapping_add(1);
        self.physical_switch_pending = false;

        self.suspend_task(prev_ref, kernel_init_task, kthreadd_task, user_task_set)?;

        let continue_result = match task_ref {
            TaskRef::BOOT => {
                BootTask::continue_canonical()?;
                boot_idle_flow.continue_active(BootTask::canonical_task())
            }
            TaskRef::KERNEL_INIT => {
                kernel_init_task.continue_on_cpu()?;
                if kernel_init_flow.state() == State::Base
                    && !kernel_init_task.task().active_flow().is_valid()
                {
                    kernel_init_flow.start_initial(kernel_init_task)
                } else if kernel_init_task
                    .task()
                    .active_flow()
                    .same_identity(kernel_init_flow.flow_ref())
                {
                    kernel_init_flow.continue_active(kernel_init_task)
                } else {
                    user_app_flow.continue_active(kernel_init_task)
                }
            }
            TaskRef::KTHREADD => {
                kthreadd_task.continue_on_cpu()?;
                if kthreadd_flow.state() == State::Base
                    && !kthreadd_task.task().active_flow().is_valid()
                {
                    kthreadd_flow.start_initial(kthreadd_task)
                } else {
                    kthreadd_flow.continue_active(kthreadd_task)
                }
            }
            TaskRef::SMOKE_SCHEDULER => {
                self.smoke_scheduler_task.continue_on_cpu()?;
                Ok(())
            }
            TaskRef::SMOKE_MUTEX => {
                self.smoke_mutex_task.continue_on_cpu()?;
                Ok(())
            }
            TaskRef::SMOKE_RWSEM => {
                self.smoke_rwsem_task.continue_on_cpu()?;
                Ok(())
            }
            TaskRef::SMOKE_RWLOCK => {
                self.smoke_rwlock_task.continue_on_cpu()?;
                Ok(())
            }
            _ if task_ref.is_user() => {
                user_task_set.continue_task(task_ref, self.boot_runqueue.cpu_ref())?;
                Ok(())
            }
            _ => self.failed_switch_to(),
        };
        continue_result?;

        Ok(())
    }

    pub(crate) fn record_task_switch_finish(&mut self, current_task: CurrentTask) -> EventResult {
        let task_ref = current_task.task_ref();
        let prev_ref = self.switch_to_entry_prev_ref;
        if prev_ref == task_ref || self.switch_to_entry_next_ref != task_ref {
            return self.failed_switch_to();
        }
        self.switch_to_exit_prev_ref = prev_ref;
        self.switch_to_exit_next_ref = task_ref;
        self.switch_to_exit_current_ref = task_ref;
        self.switch_to_exit_count = self.switch_to_exit_count.wrapping_add(1);
        self.scheduler_finish_task_switch_count =
            self.scheduler_finish_task_switch_count.wrapping_add(1);
        crate::checkpoint::checkpoint(Checkpoint::SchedulerSwitchToExit);

        self.schedule_exit_current_ref = task_ref;
        self.schedule_exit_count = self.schedule_exit_count.wrapping_add(1);
        crate::checkpoint::checkpoint(Checkpoint::SchedulerScheduleExit);
        Ok(())
    }

    fn task_identity_ptr(
        &self,
        task_ref: TaskRef,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        user_task_set: &UserTaskSet,
    ) -> Option<usize> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::canonical_task() as *const Task as usize),
            TaskRef::KERNEL_INIT => Some(kernel_init_task.task() as *const Task as usize),
            TaskRef::KTHREADD => Some(kthreadd_task.task() as *const Task as usize),
            TaskRef::SMOKE_SCHEDULER => Some(self.smoke_scheduler_task.task_ptr()),
            TaskRef::SMOKE_MUTEX => Some(self.smoke_mutex_task.task_ptr()),
            TaskRef::SMOKE_RWSEM => Some(self.smoke_rwsem_task.task_ptr()),
            TaskRef::SMOKE_RWLOCK => Some(self.smoke_rwlock_task.task_ptr()),
            _ if task_ref.is_user() => user_task_set.task_identity_ptr(task_ref),
            _ => None,
        }
    }

    fn cooperative_context_switch(
        &mut self,
        prev_ref: TaskRef,
        next_ref: TaskRef,
        kernel_init_task: &mut KernelInitTask,
        kthreadd_task: &mut KthreaddTask,
    ) -> Result<bool, EventError> {
        if prev_ref == next_ref {
            return Ok(false);
        }

        let first_boot_handoff = !self.task_stack_switching_online;
        if first_boot_handoff {
            if prev_ref != TaskRef::BOOT
                || next_ref != TaskRef::KERNEL_INIT
                || BootTask::canonical_task().state() != State::OnCpu
                || kernel_init_task.state() != State::Online
                || self.kernel_init_stack_switch_started_count != 0
            {
                self.failed_switch_to()?;
            }
            self.task_stack_switching_online = true;
            self.kernel_init_stack_switch_started_count =
                self.kernel_init_stack_switch_started_count.wrapping_add(1);
            crate::arch::riscv64::sbi::putstr("-> switch BootTask -> KernelInitTask\n");
        }

        let prev: *mut TaskSwitchContext = match prev_ref {
            TaskRef::BOOT => BootTask::canonical_switch_context_mut(),
            TaskRef::KERNEL_INIT => kernel_init_task.switch_context_mut(),
            TaskRef::KTHREADD => kthreadd_task.switch_context_mut(),
            TaskRef::SMOKE_SCHEDULER => self.smoke_scheduler_task.switch_context_mut(),
            TaskRef::SMOKE_MUTEX => self.smoke_mutex_task.switch_context_mut(),
            TaskRef::SMOKE_RWSEM => self.smoke_rwsem_task.switch_context_mut(),
            TaskRef::SMOKE_RWLOCK => self.smoke_rwlock_task.switch_context_mut(),
            _ => return Ok(false),
        };
        let next: &TaskSwitchContext = match next_ref {
            TaskRef::BOOT => BootTask::canonical_switch_context(),
            TaskRef::KERNEL_INIT => kernel_init_task.switch_context(),
            TaskRef::KTHREADD => kthreadd_task.switch_context(),
            TaskRef::SMOKE_SCHEDULER => self.smoke_scheduler_task.switch_context(),
            TaskRef::SMOKE_MUTEX => self.smoke_mutex_task.switch_context(),
            TaskRef::SMOKE_RWSEM => self.smoke_rwsem_task.switch_context(),
            TaskRef::SMOKE_RWLOCK => self.smoke_rwlock_task.switch_context(),
            _ => return Ok(false),
        };
        if !next.initialized() {
            self.failed_switch_to()?;
        }
        let next_task_identity = match next_ref {
            TaskRef::BOOT => BootTask::canonical_task() as *const Task as usize,
            TaskRef::KERNEL_INIT => kernel_init_task.task() as *const Task as usize,
            TaskRef::KTHREADD => kthreadd_task.task() as *const Task as usize,
            TaskRef::SMOKE_SCHEDULER => self.smoke_scheduler_task.task_ptr(),
            TaskRef::SMOKE_MUTEX => self.smoke_mutex_task.task_ptr(),
            TaskRef::SMOKE_RWSEM => self.smoke_rwsem_task.task_ptr(),
            TaskRef::SMOKE_RWLOCK => self.smoke_rwlock_task.task_ptr(),
            _ => return Ok(false),
        };
        let sentinel_mask = unsafe { task_switch::switch(&mut *prev, next, next_task_identity) };
        if sentinel_mask != 0xfff {
            self.failed_switch_to()?;
        }
        self.task_switch_register_sentinel_passes =
            self.task_switch_register_sentinel_passes.wrapping_add(1);
        if first_boot_handoff {
            self.kernel_init_stack_switch_returned_count =
                self.kernel_init_stack_switch_returned_count.wrapping_add(1);
            crate::arch::riscv64::sbi::putstr("<- switch BootTask restored\n");
        }
        Ok(true)
    }

    pub fn setup_smoke_scheduler_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.boot_runqueue.state() != State::Ready
            || !self
                .smoke_scheduler_task
                .setup(entry, self.boot_runqueue.cpu_id())
        {
            return Err(self.failed_schedule_condition());
        }
        Ok(())
    }

    pub fn enqueue_smoke_scheduler_task(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.smoke_scheduler_task.state() != State::Ready
            || self.smoke_scheduler_task.enqueued()
        {
            return Err(self.failed_schedule_condition());
        }

        self.boot_runqueue.enqueue_task_ref(
            RunQueueRef::cpu_owned(self.boot_runqueue.cpu_id()),
            TaskRef::SMOKE_SCHEDULER,
        )?;
        self.smoke_scheduler_task.mark_enqueued()
    }

    pub fn setup_smoke_mutex_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.boot_runqueue.state() != State::Ready
            || !self
                .smoke_mutex_task
                .setup(entry, self.boot_runqueue.cpu_id())
        {
            return Err(self.failed_schedule_condition());
        }
        Ok(())
    }

    pub fn enqueue_smoke_mutex_task(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.smoke_mutex_task.state() != State::Ready
            || self.smoke_mutex_task.enqueued()
        {
            return Err(self.failed_schedule_condition());
        }

        self.boot_runqueue.enqueue_task_ref(
            RunQueueRef::cpu_owned(self.boot_runqueue.cpu_id()),
            TaskRef::SMOKE_MUTEX,
        )?;
        self.smoke_mutex_task.mark_enqueued()
    }

    pub fn dequeue_smoke_mutex_task(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.smoke_mutex_task.state() != State::OnCpu
            || !self.smoke_mutex_task.enqueued()
        {
            return Err(self.failed_schedule_condition());
        }

        self.boot_runqueue.dequeue_task_ref(
            RunQueueRef::cpu_owned(self.boot_runqueue.cpu_id()),
            TaskRef::SMOKE_MUTEX,
        )?;
        self.smoke_mutex_task.mark_dequeued();
        Ok(())
    }

    pub fn setup_smoke_rwsem_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.boot_runqueue.state() != State::Ready
            || !self
                .smoke_rwsem_task
                .setup(entry, self.boot_runqueue.cpu_id())
        {
            return Err(self.failed_schedule_condition());
        }
        Ok(())
    }

    pub fn enqueue_smoke_rwsem_task(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.smoke_rwsem_task.state() != State::Ready
            || self.smoke_rwsem_task.enqueued()
        {
            return Err(self.failed_schedule_condition());
        }

        self.boot_runqueue.enqueue_task_ref(
            RunQueueRef::cpu_owned(self.boot_runqueue.cpu_id()),
            TaskRef::SMOKE_RWSEM,
        )?;
        self.smoke_rwsem_task.mark_enqueued()
    }

    pub fn dequeue_smoke_rwsem_task(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.smoke_rwsem_task.state() != State::OnCpu
            || !self.smoke_rwsem_task.enqueued()
        {
            return Err(self.failed_schedule_condition());
        }

        self.boot_runqueue.dequeue_task_ref(
            RunQueueRef::cpu_owned(self.boot_runqueue.cpu_id()),
            TaskRef::SMOKE_RWSEM,
        )?;
        self.smoke_rwsem_task.mark_dequeued();
        Ok(())
    }

    pub fn setup_smoke_rwlock_task(&mut self, entry: extern "C" fn() -> !) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.boot_runqueue.state() != State::Ready
            || !self
                .smoke_rwlock_task
                .setup(entry, self.boot_runqueue.cpu_id())
        {
            return Err(self.failed_schedule_condition());
        }
        Ok(())
    }

    pub fn enqueue_smoke_rwlock_task(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.smoke_rwlock_task.state() != State::Ready
            || self.smoke_rwlock_task.enqueued()
        {
            return Err(self.failed_schedule_condition());
        }

        self.boot_runqueue.enqueue_task_ref(
            RunQueueRef::cpu_owned(self.boot_runqueue.cpu_id()),
            TaskRef::SMOKE_RWLOCK,
        )?;
        self.smoke_rwlock_task.mark_enqueued()
    }

    pub fn dequeue_smoke_rwlock_task(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.smoke_rwlock_task.state() != State::OnCpu
            || !self.smoke_rwlock_task.enqueued()
        {
            return Err(self.failed_schedule_condition());
        }

        self.boot_runqueue.dequeue_task_ref(
            RunQueueRef::cpu_owned(self.boot_runqueue.cpu_id()),
            TaskRef::SMOKE_RWLOCK,
        )?;
        self.smoke_rwlock_task.mark_dequeued();
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

    pub fn select_runqueue_for_task(
        &mut self,
        task_id: usize,
        cpu_group: &CpuGroup,
    ) -> Result<RunQueueRef, EventError> {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.boot_runqueue.state() != State::Ready
            || task_id == usize::MAX
        {
            return Err(self.failed_enable_condition());
        }

        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return Err(self.failed_enable_condition());
        };
        let Some(runqueue_ref) =
            self.resolve_selected_runqueue_ref_for_cpu(cpu_group, boot_cpu.logical_id())
        else {
            return Err(self.failed_enable_condition());
        };
        self.selected_runqueue_task_id = task_id;
        Ok(runqueue_ref)
    }

    pub fn enqueue_task_on_runqueue(
        &mut self,
        task_id: usize,
        task_ref: TaskRef,
        runqueue_ref: RunQueueRef,
    ) -> EventResult {
        if self.selected_runqueue_task_id != task_id
            || !runqueue_ref.matches_cpu_owned_runqueue(self.boot_runqueue.cpu_id())
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.boot_runqueue
            .enqueue_task_with_id(runqueue_ref, task_ref, task_id)
    }

    // Used by the user-boot syscall continuation configuration.
    #[allow(dead_code)]
    pub fn dequeue_user_child_from_runqueue(
        &mut self,
        cpu_group: &CpuGroup,
        task_ref: TaskRef,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || self.boot_runqueue.state() != State::Ready
        {
            return Err(self.failed_enable_condition());
        }

        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return Err(self.failed_enable_condition());
        };
        let Some(runqueue_ref) =
            self.resolve_selected_runqueue_ref_for_cpu(cpu_group, boot_cpu.logical_id())
        else {
            return Err(self.failed_enable_condition());
        };
        self.boot_runqueue.dequeue_task_ref(runqueue_ref, task_ref)
    }

    pub fn replace_user_task_on_runqueue(
        &mut self,
        cpu_group: &CpuGroup,
        previous: TaskRef,
        next: TaskRef,
        next_pid: usize,
    ) -> EventResult {
        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return Err(self.failed_enable_condition());
        };
        let Some(runqueue_ref) =
            self.resolve_selected_runqueue_ref_for_cpu(cpu_group, boot_cpu.logical_id())
        else {
            return Err(self.failed_enable_condition());
        };
        self.boot_runqueue
            .replace_user_task_ref(runqueue_ref, previous, next, next_pid)
    }

    pub fn can_replace_user_task_on_runqueue(
        &self,
        cpu_group: &CpuGroup,
        previous: TaskRef,
    ) -> bool {
        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return false;
        };
        self.resolve_selected_runqueue_ref_for_cpu(cpu_group, boot_cpu.logical_id())
            .is_some()
            && previous.is_user()
            && self.boot_runqueue.contains_task_ref(previous)
    }

    fn resolve_selected_runqueue_ref_for_cpu(
        &self,
        cpu_group: &CpuGroup,
        cpu_id: usize,
    ) -> Option<RunQueueRef> {
        let cpu = cpu_group.cpu(cpu_id)?;
        let runqueue = self.cpu_runqueue(cpu_id)?;
        if cpu_group.state() == State::Ready
            && cpu_group.possible_contains(cpu.cpu_ref())
            && self.default_root_domain.covers_cpu_ref(cpu.cpu_ref())
            && runqueue.state() == State::Ready
            && runqueue.is_boot_backed()
            && runqueue.cpu_ref() == cpu.cpu_ref()
            && runqueue.cpu_id() == cpu.logical_id()
            && runqueue.cpu_hartid() == cpu.hartid()
            && self.boot_runqueue_matches_metadata()
        {
            Some(RunQueueRef::cpu_owned(runqueue.cpu_id()))
        } else {
            None
        }
    }

    fn failed_enable_condition(&self) -> EventError {
        EventError::failed(
            EventErrorCode::ConditionFailed,
            LifecycleEvent::Enable,
            self.lifecycle.state(),
            State::Online,
            State::Online,
        )
    }

    pub fn enable_smp(
        &mut self,
        kernel_init_task: &mut KernelInitTask,
        kernel_init_flow: &KernelInitFlow,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || kernel_init_task.state() != State::OnCpu
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

        if self.sched_domains_mutex.state() == State::Base {
            self.sched_domains_mutex.preset_static()?;
            self.sched_domains_mutex.setup()?;
        }
        if self.sched_domains_mutex.state() != State::Ready
            || self
                .sched_domains_mutex
                .lock_owner(MutexOwner::KernelInitTask)?
                != MutexLockOutcome::Acquired
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }
        self.sched_domains_mutex_guard_used = true;
        self.smp_cpu_masks_stable = cpu_group.secondary_cpus_online()
            && cpu_group.smp_concurrency_open()
            && cpu_group.possible_cpu_count() > 0;
        self.sched_domains_mutex
            .unlock_owner(MutexOwner::KernelInitTask)?;

        if !kernel_init_task.release_boot_cpu_affinity(kernel_init_flow, cpu_group) {
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
        crate::checkpoint::checkpoint(Checkpoint::SchedulerSmpReady);
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
            && self.boot_runqueue.lock().state() == State::Ready
            && !self.boot_runqueue.lock().locked()
            && self.boot_runqueue.lock().acquired_count() != 0
            && self.boot_runqueue.lock().released_count() != 0
            && self.boot_runqueue.root_attach_held_runqueue_lock()
            && self.boot_runqueue.lock().irqsave_entered_count() != 0
            && self.boot_runqueue.lock().irqrestore_exited_count() != 0
            && self.boot_init_preemption.state() == State::Ready
            && self.boot_init_preemption.disabled()
            && self.boot_idle_setup_state.state() == State::Ready
            && self.boot_idle_setup_state.pi_lock().state() == State::Ready
            && !self.boot_idle_setup_state.pi_lock().locked()
            && self.boot_idle_setup_state.pi_lock().irqsave_entered_count() != 0
            && self
                .boot_idle_setup_state
                .pi_lock()
                .irqrestore_exited_count()
                != 0
            && self.boot_idle_rcu_read_side.state() == State::Prepared
            && self.boot_idle_rcu_read_side.incomplete_first_slice()
            && self.boot_idle_rcu_read_side.full_semantics_deferred()
            && self.boot_idle_rcu_read_side.read_lock_count() != 0
            && self.boot_idle_rcu_read_side.read_unlock_count() != 0
            && self.boot_idle_rcu_read_side.balanced()
            && self.boot_idle_setup_state.init_held_pi_lock()
            && self.boot_idle_setup_state.init_held_runqueue_lock()
            && self.boot_idle_setup_state.cpu_set_under_rcu_read()
            && self
                .boot_idle_setup_state
                .runqueue_current_published_with_rcu()
            && self.boot_idle_preemption.state() == State::Ready
            && self.boot_idle_preemption.disabled()
            && self.possible_cpu_runqueues_ready(cpu_group)
            && self.boot_runqueue_matches_metadata()
            && self.boot_cpu_owned_scheduler_view_ready(cpu_group)
            && self.boot_runqueue.cpu_ref().is_boot_cpu()
            && cpu_group
                .boot_cpu()
                .map(|cpu| self.boot_runqueue.cpu_ref() == cpu.cpu_ref())
                .unwrap_or(false)
            && self
                .default_root_domain
                .covers_cpu_group_possible(cpu_group)
            && self
                .default_root_domain
                .covers_cpu_ref(self.boot_runqueue.cpu_ref())
            && self.boot_runqueue.curr_task_id() == self.boot_idle_setup_state.task_id()
            && self.boot_runqueue.idle_task_id() == self.boot_idle_setup_state.task_id()
    }

    fn setup_cpu_runqueue_metadata(&mut self, cpu_group: &CpuGroup) -> EventResult {
        if cpu_group.state() != State::Ready
            || !cpu_group.possible_cpu_boundary_ready()
            || !self
                .default_root_domain
                .covers_cpu_group_possible(cpu_group)
            || self.boot_runqueue.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.cpu_runqueues = [const { CpuRunQueueMetadata::invalid() }; MAX_CPUS];
        self.cpu_runqueue_count = 0;

        let mut logical_id = 0usize;
        while logical_id < cpu_group.possible_cpu_count() {
            let Some(cpu) = cpu_group.cpu(logical_id) else {
                return self.failed_setup();
            };
            let Some(cpu_ref) = cpu_group.possible_cpu_ref_at(logical_id) else {
                return self.failed_setup();
            };
            if cpu.cpu_ref() != cpu_ref || !self.default_root_domain.covers_cpu_ref(cpu_ref) {
                return self.failed_setup();
            }

            self.cpu_runqueues[logical_id] =
                CpuRunQueueMetadata::ready(cpu_ref, cpu.hartid(), cpu_ref.is_boot_cpu());
            self.cpu_runqueue_count += 1;
            logical_id += 1;
        }

        if !self.possible_cpu_runqueues_ready(cpu_group) {
            return self.failed_setup();
        }
        Ok(())
    }
}

fn trace_switch_to(prev_ref: TaskRef, next_ref: TaskRef, current_after: TaskRef) {
    #[cfg(checkpoint_handler_announce)]
    {
        crate::arch::riscv64::sbi::putstr("checkpoint: Scheduler.SwitchTo prev=");
        crate::arch::riscv64::sbi::putstr(prev_ref.name());
        crate::arch::riscv64::sbi::putstr(" next=");
        crate::arch::riscv64::sbi::putstr(next_ref.name());
        crate::arch::riscv64::sbi::putstr(" current_after=");
        crate::arch::riscv64::sbi::putstr(current_after.name());
        crate::arch::riscv64::sbi::putchar(b'\n');
    }

    #[cfg(not(checkpoint_handler_announce))]
    {
        let _ = (prev_ref, next_ref, current_after);
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CurrentRunQueueRef {
    BootRunQueue { cpu_id: usize },
}

impl CurrentRunQueueRef {
    pub const fn cpu_owned(cpu_id: usize) -> Self {
        Self::BootRunQueue { cpu_id }
    }

    pub const fn cpu_id(self) -> usize {
        match self {
            Self::BootRunQueue { cpu_id } => cpu_id,
        }
    }

    pub const fn matches_cpu_owned_runqueue(self, cpu_id: usize) -> bool {
        match self {
            Self::BootRunQueue { cpu_id: ref_cpu_id } => ref_cpu_id == cpu_id,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RunQueueRef {
    BootRunQueue { cpu_id: usize },
}

impl RunQueueRef {
    pub const fn cpu_owned(cpu_id: usize) -> Self {
        Self::BootRunQueue { cpu_id }
    }

    pub const fn cpu_id(self) -> usize {
        match self {
            Self::BootRunQueue { cpu_id } => cpu_id,
        }
    }

    pub const fn matches_cpu_owned_runqueue(self, cpu_id: usize) -> bool {
        match self {
            Self::BootRunQueue { cpu_id: ref_cpu_id } => ref_cpu_id == cpu_id,
        }
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(not(app_smoke), allow(dead_code))]
struct CpuRunQueueMetadata {
    state: State,
    cpu_ref: CpuRef,
    cpu_hartid: usize,
    class_queues_ready: bool,
    attached_to_root_domain: bool,
    balance_push_enabled: bool,
    boot_backed: bool,
}

impl CpuRunQueueMetadata {
    const fn invalid() -> Self {
        Self {
            state: State::Base,
            cpu_ref: CpuRef::invalid(),
            cpu_hartid: usize::MAX,
            class_queues_ready: false,
            attached_to_root_domain: false,
            balance_push_enabled: true,
            boot_backed: false,
        }
    }

    const fn ready(cpu_ref: CpuRef, cpu_hartid: usize, boot_backed: bool) -> Self {
        Self {
            state: State::Ready,
            cpu_ref,
            cpu_hartid,
            class_queues_ready: true,
            attached_to_root_domain: true,
            balance_push_enabled: false,
            boot_backed,
        }
    }

    fn view(self) -> Option<CpuRunQueueView> {
        if self.state == State::Ready {
            Some(CpuRunQueueView {
                state: self.state,
                cpu_ref: self.cpu_ref,
                cpu_hartid: self.cpu_hartid,
                class_queues_ready: self.class_queues_ready,
                attached_to_root_domain: self.attached_to_root_domain,
                balance_push_enabled: self.balance_push_enabled,
                boot_backed: self.boot_backed,
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
pub struct CpuRunQueueView {
    state: State,
    cpu_ref: CpuRef,
    cpu_hartid: usize,
    class_queues_ready: bool,
    attached_to_root_domain: bool,
    balance_push_enabled: bool,
    boot_backed: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl CpuRunQueueView {
    pub const fn state(self) -> State {
        self.state
    }

    pub const fn cpu_ref(self) -> CpuRef {
        self.cpu_ref
    }

    pub const fn cpu_id(self) -> usize {
        self.cpu_ref.logical_id()
    }

    pub const fn cpu_hartid(self) -> usize {
        self.cpu_hartid
    }

    pub const fn class_queues_ready(self) -> bool {
        self.class_queues_ready
    }

    pub const fn attached_to_root_domain(self) -> bool {
        self.attached_to_root_domain
    }

    pub const fn balance_push_enabled(self) -> bool {
        self.balance_push_enabled
    }

    pub const fn is_boot_backed(self) -> bool {
        self.boot_backed
    }
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
#[derive(Clone, Copy)]
pub struct CpuOwnedSchedulerView {
    cpu_ref: CpuRef,
    cpu_hartid: usize,
    runqueue: CpuRunQueueView,
    idle_task: CpuIdleTaskView,
    runqueue_current_task_id: usize,
    runqueue_idle_task_id: usize,
    runqueue_task_count: usize,
    runqueue_kernel_init_task_enqueued: bool,
    runqueue_kthreadd_task_enqueued: bool,
    runqueue_user_child_task_enqueued: bool,
    runqueue_smoke_scheduler_task_enqueued: bool,
    runqueue_smoke_mutex_task_enqueued: bool,
    runqueue_smoke_rwsem_task_enqueued: bool,
    runqueue_smoke_rwlock_task_enqueued: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl CpuOwnedSchedulerView {
    pub const fn cpu_ref(self) -> CpuRef {
        self.cpu_ref
    }

    pub const fn cpu_id(self) -> usize {
        self.cpu_ref.logical_id()
    }

    pub const fn cpu_hartid(self) -> usize {
        self.cpu_hartid
    }

    pub const fn runqueue(self) -> CpuRunQueueView {
        self.runqueue
    }

    pub const fn idle_task(self) -> CpuIdleTaskView {
        self.idle_task
    }

    pub const fn runqueue_current_task_id(self) -> usize {
        self.runqueue_current_task_id
    }

    pub const fn runqueue_idle_task_id(self) -> usize {
        self.runqueue_idle_task_id
    }

    pub const fn runqueue_task_count(self) -> usize {
        self.runqueue_task_count
    }

    pub const fn runqueue_contains_task_id(self, task_id: usize) -> bool {
        (self.runqueue_kernel_init_task_enqueued
            && task_id == crate::objects::rest_init::KERNEL_INIT_PID)
            || (self.runqueue_kthreadd_task_enqueued
                && task_id == crate::objects::rest_init::KTHREADD_PID)
            || (self.runqueue_user_child_task_enqueued && task_id == USER_CHILD_PID)
            || (self.runqueue_smoke_scheduler_task_enqueued && task_id == SMOKE_SCHEDULER_TASK_ID)
            || (self.runqueue_smoke_mutex_task_enqueued && task_id == SMOKE_MUTEX_TASK_ID)
            || (self.runqueue_smoke_rwsem_task_enqueued && task_id == SMOKE_RWSEM_TASK_ID)
            || (self.runqueue_smoke_rwlock_task_enqueued && task_id == SMOKE_RWLOCK_TASK_ID)
    }

    pub const fn runqueue_idle_task_matches(self) -> bool {
        self.runqueue.cpu_ref().logical_id() == self.idle_task.cpu_id()
            && self.runqueue.cpu_ref().logical_id() == self.cpu_ref.logical_id()
            && self.runqueue_idle_task_id == self.idle_task.task_id()
            && self.runqueue_current_task_id == self.idle_task.task_id()
    }
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
#[derive(Clone, Copy)]
pub struct CpuIdleTaskView {
    state: State,
    task_id: usize,
    cpu_ref: CpuRef,
    cpu_id: usize,
    uses_current_init_task: bool,
    lazy_tlb_mm_ready: bool,
    no_set_affinity: bool,
    switch_ctx_ra: usize,
    switch_ctx_initialized: bool,
    core_saved_count: usize,
    core_restored_count: usize,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl CpuIdleTaskView {
    pub const fn state(self) -> State {
        self.state
    }

    pub const fn task_id(self) -> usize {
        self.task_id
    }

    pub const fn cpu_ref(self) -> CpuRef {
        self.cpu_ref
    }

    pub const fn cpu_id(self) -> usize {
        self.cpu_id
    }

    pub const fn uses_current_init_task(self) -> bool {
        self.uses_current_init_task
    }

    pub const fn lazy_tlb_mm_ready(self) -> bool {
        self.lazy_tlb_mm_ready
    }

    pub const fn no_set_affinity(self) -> bool {
        self.no_set_affinity
    }

    // Retained in the stable CPU-idle diagnostic view.
    #[allow(dead_code)]
    pub const fn switch_ctx_ra(self) -> usize {
        self.switch_ctx_ra
    }

    pub const fn switch_ctx_initialized(self) -> bool {
        self.switch_ctx_initialized
    }

    pub const fn core_saved_count(self) -> usize {
        self.core_saved_count
    }

    pub const fn core_restored_count(self) -> usize {
        self.core_restored_count
    }
}

fn task_id_for_current_task_ref(task_ref: TaskRef) -> Option<usize> {
    if task_ref.is_user() {
        return Some(USER_CHILD_PID);
    }
    match task_ref {
        TaskRef::KERNEL_INIT => Some(crate::objects::rest_init::KERNEL_INIT_PID),
        TaskRef::KTHREADD => Some(crate::objects::rest_init::KTHREADD_PID),
        TaskRef::SMOKE_SCHEDULER => Some(SMOKE_SCHEDULER_TASK_ID),
        TaskRef::SMOKE_MUTEX => Some(SMOKE_MUTEX_TASK_ID),
        TaskRef::SMOKE_RWSEM => Some(SMOKE_RWSEM_TASK_ID),
        TaskRef::SMOKE_RWLOCK => Some(SMOKE_RWLOCK_TASK_ID),
        TaskRef::NONE | TaskRef::BOOT => None,
        _ => None,
    }
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub struct SmokeSchedulerTask {
    task: Task,
    flow: TaskFlow,
    stack: [usize; SMOKE_SCHEDULER_STACK_WORDS],
    enqueued: bool,
    entry_ran: bool,
    yielded_back: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl SmokeSchedulerTask {
    const fn new(task_id: usize) -> Self {
        Self {
            task: Task::with_ref(smoke_task_ref(task_id)),
            flow: TaskFlow::new_static(TaskFlowRef::smoke(smoke_flow_index(task_id))),
            stack: [0; SMOKE_SCHEDULER_STACK_WORDS],
            enqueued: false,
            entry_ran: false,
            yielded_back: false,
        }
    }

    pub const fn state(&self) -> State {
        self.task.state()
    }

    pub const fn task_id(&self) -> usize {
        self.task.pid()
    }

    pub const fn cpu_id(&self) -> usize {
        self.flow.cpu_id()
    }

    pub const fn cpu_ref(&self) -> Option<CpuRef> {
        self.flow.cpu_ref()
    }

    pub const fn enqueued(&self) -> bool {
        self.enqueued
    }

    pub const fn entry_ran(&self) -> bool {
        self.entry_ran
    }

    pub const fn yielded_back(&self) -> bool {
        self.yielded_back
    }

    pub const fn thread_context(&self) -> &super::task::TaskThreadContext {
        self.task.thread_context()
    }

    pub const fn task_ref(&self) -> TaskRef {
        self.task.task_ref()
    }

    fn task_ptr(&self) -> usize {
        &self.task as *const Task as usize
    }

    fn current_task_candidate(&self) -> super::current_task::CurrentTaskCandidate<'_> {
        super::current_task::CurrentTaskCandidate {
            task: &self.task,
            flow: &self.flow,
        }
    }

    pub const fn flow_ref(&self) -> TaskFlowRef {
        self.flow.flow_ref()
    }

    pub fn unified_carrier_ready(&self) -> bool {
        self.flow.owner().same_identity(self.task.task_ref())
            && self.task.initial_flow().same_identity(self.flow.flow_ref())
            && ((self.task.state() == State::Ready
                && self.flow.state() == State::Base
                && !self.flow.active()
                && !self.task.active_flow().is_valid())
                || (self.task.state() == State::Online
                    && self.flow.state() == State::Online
                    && self.flow.active()
                    && self.task.active_flow().same_identity(self.flow.flow_ref()))
                || (self.task.state() == State::OnCpu
                    && self.flow.state() == State::Online
                    && self.flow.active()
                    && self.task.active_flow().same_identity(self.flow.flow_ref())))
    }

    fn setup(&mut self, entry: extern "C" fn() -> !, cpu_id: usize) -> bool {
        if self.task.state() != State::Base || cpu_id == usize::MAX {
            return false;
        }

        let stack_top = self.stack.as_ptr() as usize + core::mem::size_of_val(&self.stack);
        if self
            .task
            .set_identity_metadata(
                self.task_id_from_ref(),
                TaskEntry::SmokeScheduler,
                TaskKind::TestOnly,
            )
            .is_err()
            || self.task.adopt_preset().is_err()
            || self.flow.bind(&mut self.task, TaskFlowRef::NONE).is_err()
            || self.task.bind_initial_flow(&self.flow).is_err()
        {
            return false;
        }
        self.task.init_switch_context(entry, stack_top);
        if self.task.adopt_setup().is_err() {
            return false;
        }
        if !self.flow.bind_cpu_ref(CpuRef::new(cpu_id)) {
            return false;
        }
        true
    }

    const fn task_id_from_ref(&self) -> usize {
        match self.task.task_ref() {
            TaskRef::SMOKE_SCHEDULER => SMOKE_SCHEDULER_TASK_ID,
            TaskRef::SMOKE_MUTEX => SMOKE_MUTEX_TASK_ID,
            TaskRef::SMOKE_RWSEM => SMOKE_RWSEM_TASK_ID,
            TaskRef::SMOKE_RWLOCK => SMOKE_RWLOCK_TASK_ID,
            _ => usize::MAX,
        }
    }

    fn mark_enqueued(&mut self) -> EventResult {
        self.enqueued = true;
        self.task.set_runtime_running()?;
        self.task.publish_runqueue_binding()?;
        self.task.adopt_enable()
    }

    fn mark_dequeued(&mut self) {
        self.enqueued = false;
    }

    pub fn mark_entry_ran(&mut self) -> EventResult {
        if self.task.state() != State::OnCpu || self.flow.state() != State::Online || !self.enqueued
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.task.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.entry_ran = true;
        Ok(())
    }

    pub fn mark_yielded_back(&mut self) -> EventResult {
        if self.task.state() != State::OnCpu || !self.entry_ran {
            return failed_condition(
                LifecycleEvent::Setup,
                self.task.state(),
                State::Ready,
                State::Ready,
            );
        }

        self.yielded_back = true;
        Ok(())
    }

    fn switch_context(&self) -> &TaskSwitchContext {
        self.task.switch_context()
    }

    fn switch_context_mut(&mut self) -> &mut TaskSwitchContext {
        self.task.switch_context_mut()
    }

    fn suspend_from_cpu(&mut self) -> EventResult {
        self.task.suspend_from_cpu()
    }

    fn continue_on_cpu(&mut self) -> EventResult {
        self.task.continue_on_cpu()?;
        if self.flow.state() == State::Base && !self.task.active_flow().is_valid() {
            self.flow.start_initial(&mut self.task, None, None)
        } else if self.flow.state() == State::Online
            && self.flow.active()
            && self.task.active_flow().same_identity(self.flow.flow_ref())
        {
            Ok(())
        } else {
            failed_condition(
                LifecycleEvent::Continue,
                self.flow.state(),
                State::Online,
                State::Online,
            )
        }
    }
}

const fn smoke_task_ref(task_id: usize) -> TaskRef {
    match task_id {
        SMOKE_SCHEDULER_TASK_ID => TaskRef::SMOKE_SCHEDULER,
        SMOKE_MUTEX_TASK_ID => TaskRef::SMOKE_MUTEX,
        SMOKE_RWSEM_TASK_ID => TaskRef::SMOKE_RWSEM,
        SMOKE_RWLOCK_TASK_ID => TaskRef::SMOKE_RWLOCK,
        _ => TaskRef::NONE,
    }
}

const fn smoke_flow_index(task_id: usize) -> usize {
    match task_id {
        SMOKE_SCHEDULER_TASK_ID => 0,
        SMOKE_MUTEX_TASK_ID => 1,
        SMOKE_RWSEM_TASK_ID => 2,
        SMOKE_RWLOCK_TASK_ID => 3,
        _ => 0,
    }
}

pub struct BitWaitQueueTable {
    lifecycle: Lifecycle,
    bucket_count: usize,
    bucket_waitqueues_ready: bool,
    bucket_locks_ready: bool,
    bucket_lists_empty: bool,
}

impl BitWaitQueueTable {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            bucket_count: 0,
            bucket_waitqueues_ready: false,
            bucket_locks_ready: false,
            bucket_lists_empty: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn bucket_count(&self) -> usize {
        self.bucket_count
    }

    pub const fn bucket_count_matches_wait_table_size(&self) -> bool {
        self.bucket_count == BIT_WAIT_TABLE_SIZE
    }

    pub const fn bucket_waitqueues_ready(&self) -> bool {
        self.bucket_waitqueues_ready
    }

    pub const fn bucket_locks_ready(&self) -> bool {
        self.bucket_locks_ready
    }

    pub const fn bucket_lists_empty(&self) -> bool {
        self.bucket_lists_empty
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

        self.bucket_count = BIT_WAIT_TABLE_SIZE;
        self.bucket_waitqueues_ready = true;
        self.bucket_locks_ready = true;
        self.bucket_lists_empty = true;
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
    lock: RawSpinLock,
    cpu_ref: CpuRef,
    cpu_hartid: usize,
    curr_task_id: usize,
    idle_task_id: usize,
    cfs_ready: bool,
    rt_ready: bool,
    dl_ready: bool,
    attached_to_root_domain: bool,
    root_attach_held_runqueue_lock: bool,
    balance_push_enabled: bool,
    enqueued_task_id: usize,
    kernel_init_task_enqueued: bool,
    kthreadd_task_enqueued: bool,
    user_child_task_enqueued: bool,
    user_child_task_ref: TaskRef,
    user_child_task_pid: usize,
    smoke_scheduler_task_enqueued: bool,
    smoke_mutex_task_enqueued: bool,
    smoke_rwsem_task_enqueued: bool,
    smoke_rwlock_task_enqueued: bool,
}

impl BootRunQueue {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            lock: RawSpinLock::new(),
            cpu_ref: CpuRef::invalid(),
            cpu_hartid: usize::MAX,
            curr_task_id: usize::MAX,
            idle_task_id: usize::MAX,
            cfs_ready: false,
            rt_ready: false,
            dl_ready: false,
            attached_to_root_domain: false,
            root_attach_held_runqueue_lock: false,
            balance_push_enabled: true,
            enqueued_task_id: usize::MAX,
            kernel_init_task_enqueued: false,
            kthreadd_task_enqueued: false,
            user_child_task_enqueued: false,
            user_child_task_ref: TaskRef::NONE,
            user_child_task_pid: 0,
            smoke_scheduler_task_enqueued: false,
            smoke_mutex_task_enqueued: false,
            smoke_rwsem_task_enqueued: false,
            smoke_rwlock_task_enqueued: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn lock(&self) -> &RawSpinLock {
        &self.lock
    }

    pub const fn cpu_id(&self) -> usize {
        self.cpu_ref.logical_id()
    }

    pub const fn cpu_ref(&self) -> CpuRef {
        self.cpu_ref
    }

    pub const fn cpu_hartid(&self) -> usize {
        self.cpu_hartid
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

    pub const fn root_attach_held_runqueue_lock(&self) -> bool {
        self.root_attach_held_runqueue_lock
    }

    pub fn is_boot_cpu_runqueue_view(&self, cpu_group: &CpuGroup) -> bool {
        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return false;
        };
        self.lifecycle.state() == State::Ready
            && self.cpu_ref == boot_cpu.cpu_ref()
            && self.cpu_hartid == boot_cpu.hartid()
            && self.cpu_ref.is_boot_cpu()
    }

    pub const fn balance_push_enabled(&self) -> bool {
        self.balance_push_enabled
    }

    pub const fn contains_task(&self, task_id: usize) -> bool {
        (self.kernel_init_task_enqueued && task_id == crate::objects::rest_init::KERNEL_INIT_PID)
            || (self.kthreadd_task_enqueued && task_id == crate::objects::rest_init::KTHREADD_PID)
            || (self.user_child_task_enqueued && task_id == self.user_child_task_pid)
            || (self.smoke_scheduler_task_enqueued && task_id == SMOKE_SCHEDULER_TASK_ID)
            || (self.smoke_mutex_task_enqueued && task_id == SMOKE_MUTEX_TASK_ID)
            || (self.smoke_rwsem_task_enqueued && task_id == SMOKE_RWSEM_TASK_ID)
            || (self.smoke_rwlock_task_enqueued && task_id == SMOKE_RWLOCK_TASK_ID)
    }

    pub const fn task_count(&self) -> usize {
        self.kernel_init_task_enqueued as usize
            + self.kthreadd_task_enqueued as usize
            + self.user_child_task_enqueued as usize
            + self.smoke_scheduler_task_enqueued as usize
            + self.smoke_mutex_task_enqueued as usize
            + self.smoke_rwsem_task_enqueued as usize
            + self.smoke_rwlock_task_enqueued as usize
    }

    pub const fn contains_task_ref(&self, task_ref: TaskRef) -> bool {
        if task_ref.is_user() {
            self.user_child_task_enqueued && self.user_child_task_ref.same_identity(task_ref)
        } else if task_ref.same_identity(TaskRef::KERNEL_INIT) {
            self.kernel_init_task_enqueued
        } else if task_ref.same_identity(TaskRef::KTHREADD) {
            self.kthreadd_task_enqueued
        } else if task_ref.same_identity(TaskRef::SMOKE_SCHEDULER) {
            self.smoke_scheduler_task_enqueued
        } else if task_ref.same_identity(TaskRef::SMOKE_MUTEX) {
            self.smoke_mutex_task_enqueued
        } else if task_ref.same_identity(TaskRef::SMOKE_RWSEM) {
            self.smoke_rwsem_task_enqueued
        } else if task_ref.same_identity(TaskRef::SMOKE_RWLOCK) {
            self.smoke_rwlock_task_enqueued
        } else {
            false
        }
    }

    pub const fn first_runnable_task_ref(&self) -> TaskRef {
        if self.smoke_rwlock_task_enqueued {
            TaskRef::SMOKE_RWLOCK
        } else if self.smoke_rwsem_task_enqueued {
            TaskRef::SMOKE_RWSEM
        } else if self.smoke_mutex_task_enqueued {
            TaskRef::SMOKE_MUTEX
        } else if self.smoke_scheduler_task_enqueued {
            TaskRef::SMOKE_SCHEDULER
        } else if self.user_child_task_enqueued {
            self.user_child_task_ref
        } else if self.kernel_init_task_enqueued {
            TaskRef::KERNEL_INIT
        } else if self.kthreadd_task_enqueued {
            TaskRef::KTHREADD
        } else {
            TaskRef::NONE
        }
    }

    // Formal RunQueue.Event::Setup implementation boundary. Scheduler.setup()
    // drives it for the production boot path; local subject tests may call the
    // same lifecycle API without creating a test-only entry point.
    #[allow(clippy::too_many_arguments)]
    pub fn setup(
        &mut self,
        boot_cpu_ref: CpuRef,
        boot_cpu_hartid: usize,
        _per_cpu_storage: &PerCpuStorage,
        root_domain: &DefaultSchedRootDomain,
        boot_task: &BootTask,
        local_interrupt: &mut LocalInterruptControl,
        boot_init_preemption: &mut PreemptionControl,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.lock.state() != State::Base
            || root_domain.state() != State::Ready
            || boot_task.state() != State::OnCpu
            || local_interrupt.state() != State::Ready
            || boot_init_preemption.state() != State::Base
        {
            return self.failed_setup();
        }

        if !boot_cpu_ref.is_boot_cpu() || !root_domain.covers_cpu_ref(boot_cpu_ref) {
            return self.failed_setup();
        }

        self.lock
            .setup_with_checkpoint(Checkpoint::BootRunQueueLockReady)?;
        boot_init_preemption
            .setup_disabled_with_checkpoint(boot_task, Checkpoint::BootInitPreemptionReady)?;
        self.cpu_ref = boot_cpu_ref;
        self.cpu_hartid = boot_cpu_hartid;
        self.curr_task_id = 0;
        self.idle_task_id = 0;
        self.cfs_ready = true;
        self.rt_ready = true;
        self.dl_ready = true;
        self.lock
            .lock_irqsave(local_interrupt, boot_init_preemption)?;
        let attach_result: EventResult = {
            self.attached_to_root_domain = true;
            self.root_attach_held_runqueue_lock = true;
            Ok(())
        };
        let unlock_result = self
            .lock
            .unlock_irqrestore(local_interrupt, boot_init_preemption);
        attach_result.and(unlock_result)?;
        self.attached_to_root_domain = true;
        self.balance_push_enabled = false;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::BootRunQueueReady,
        )
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn setup_for_local_subject(
        &mut self,
        cpu_group: &CpuGroup,
        per_cpu_storage: &PerCpuStorage,
        root_domain: &DefaultSchedRootDomain,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.lock.state() != State::Base
            || cpu_group.state() != State::Ready
            || !cpu_group.possible_cpu_boundary_ready()
            || per_cpu_storage.state() != State::Ready
            || root_domain.state() != State::Ready
        {
            return self.failed_setup();
        }

        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return self.failed_setup();
        };
        if !boot_cpu.cpu_ref().is_boot_cpu()
            || !cpu_group.possible_contains(boot_cpu.cpu_ref())
            || !root_domain.covers_cpu_ref(boot_cpu.cpu_ref())
        {
            return self.failed_setup();
        }

        self.lock
            .setup_with_checkpoint(Checkpoint::BootRunQueueLockReady)?;
        self.cpu_ref = boot_cpu.cpu_ref();
        self.cpu_hartid = boot_cpu.hartid();
        self.curr_task_id = 0;
        self.idle_task_id = 0;
        self.cfs_ready = true;
        self.rt_ready = true;
        self.dl_ready = true;
        self.attached_to_root_domain = true;
        self.root_attach_held_runqueue_lock = true;
        self.balance_push_enabled = false;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            Checkpoint::BootRunQueueReady,
        )
    }

    pub fn enqueue_task_ref(
        &mut self,
        runqueue_ref: RunQueueRef,
        task_ref: TaskRef,
    ) -> EventResult {
        if !runqueue_ref.matches_cpu_owned_runqueue(self.cpu_id()) {
            return self.failed_setup();
        }

        if task_ref.is_user() {
            return self.enqueue_task_with_id(runqueue_ref, task_ref, USER_CHILD_PID);
        }

        let Some(task_id) = task_id_for_current_task_ref(task_ref) else {
            return self.failed_setup();
        };
        self.enqueue_task(task_id)
    }

    pub fn enqueue_task_with_id(
        &mut self,
        runqueue_ref: RunQueueRef,
        task_ref: TaskRef,
        task_id: usize,
    ) -> EventResult {
        if !runqueue_ref.matches_cpu_owned_runqueue(self.cpu_id())
            || !task_ref.is_boot_scheduler_ref()
            || task_ref.same_identity(TaskRef::BOOT)
            || task_id == usize::MAX
        {
            return self.failed_setup();
        }
        if task_ref.is_user() {
            if self.user_child_task_enqueued || task_id < USER_CHILD_PID {
                return self.failed_setup();
            }
            self.user_child_task_ref = task_ref;
            self.user_child_task_pid = task_id;
            self.user_child_task_enqueued = true;
            self.enqueued_task_id = task_id;
            return Ok(());
        }
        if task_id_for_current_task_ref(task_ref) != Some(task_id) {
            return self.failed_setup();
        }
        self.enqueue_task(task_id)
    }

    pub fn pick_next_task(
        &self,
        runqueue_ref: CurrentRunQueueRef,
        prev_ref: TaskRef,
    ) -> Result<TaskRef, EventError> {
        if !runqueue_ref.matches_cpu_owned_runqueue(self.cpu_id())
            || self.lifecycle.state() != State::Ready
            || !prev_ref.is_boot_scheduler_ref()
            || self.task_count() == 0
        {
            return Err(self.failed_setup_error());
        }

        let next_ref = if prev_ref.is_user()
            || matches!(
                prev_ref,
                TaskRef::SMOKE_SCHEDULER
                    | TaskRef::SMOKE_MUTEX
                    | TaskRef::SMOKE_RWSEM
                    | TaskRef::SMOKE_RWLOCK
            ) {
            if self.kernel_init_task_enqueued {
                TaskRef::KERNEL_INIT
            } else {
                TaskRef::NONE
            }
        } else {
            match prev_ref {
                TaskRef::KERNEL_INIT => {
                    if self.smoke_rwlock_task_enqueued {
                        TaskRef::SMOKE_RWLOCK
                    } else if self.smoke_rwsem_task_enqueued {
                        TaskRef::SMOKE_RWSEM
                    } else if self.smoke_mutex_task_enqueued {
                        TaskRef::SMOKE_MUTEX
                    } else if self.smoke_scheduler_task_enqueued {
                        TaskRef::SMOKE_SCHEDULER
                    } else if self.user_child_task_enqueued {
                        self.user_child_task_ref
                    } else if self.kthreadd_task_enqueued {
                        TaskRef::KTHREADD
                    } else {
                        TaskRef::BOOT
                    }
                }
                TaskRef::KTHREADD => {
                    if self.kernel_init_task_enqueued {
                        TaskRef::KERNEL_INIT
                    } else {
                        TaskRef::BOOT
                    }
                }
                TaskRef::BOOT => self.first_runnable_task_ref(),
                TaskRef::NONE => TaskRef::NONE,
                _ => TaskRef::NONE,
            }
        };
        if matches!(next_ref, TaskRef::NONE) {
            return Err(self.failed_setup_error());
        }
        Ok(next_ref)
    }

    fn enqueue_task(&mut self, task_id: usize) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || task_id == usize::MAX
            || self.contains_task(task_id)
            || (task_id != crate::objects::rest_init::KERNEL_INIT_PID
                && task_id != crate::objects::rest_init::KTHREADD_PID
                && task_id != USER_CHILD_PID
                && task_id != SMOKE_SCHEDULER_TASK_ID
                && task_id != SMOKE_MUTEX_TASK_ID
                && task_id != SMOKE_RWSEM_TASK_ID
                && task_id != SMOKE_RWLOCK_TASK_ID)
        {
            return self.failed_setup();
        }

        self.enqueued_task_id = task_id;
        if task_id == crate::objects::rest_init::KERNEL_INIT_PID {
            self.kernel_init_task_enqueued = true;
        }
        if task_id == crate::objects::rest_init::KTHREADD_PID {
            self.kthreadd_task_enqueued = true;
        }
        if task_id == USER_CHILD_PID {
            self.user_child_task_enqueued = true;
        }
        if task_id == SMOKE_SCHEDULER_TASK_ID {
            self.smoke_scheduler_task_enqueued = true;
        }
        if task_id == SMOKE_MUTEX_TASK_ID {
            self.smoke_mutex_task_enqueued = true;
        }
        if task_id == SMOKE_RWSEM_TASK_ID {
            self.smoke_rwsem_task_enqueued = true;
        }
        if task_id == SMOKE_RWLOCK_TASK_ID {
            self.smoke_rwlock_task_enqueued = true;
        }
        Ok(())
    }

    pub fn dequeue_task_ref(
        &mut self,
        runqueue_ref: RunQueueRef,
        task_ref: TaskRef,
    ) -> EventResult {
        if !runqueue_ref.matches_cpu_owned_runqueue(self.cpu_id()) {
            return self.failed_setup();
        }

        if task_ref.is_user() {
            if !self.user_child_task_enqueued || !self.user_child_task_ref.same_identity(task_ref) {
                return self.failed_setup();
            }
            self.user_child_task_enqueued = false;
            self.user_child_task_ref = TaskRef::NONE;
            self.user_child_task_pid = 0;
            self.enqueued_task_id = usize::MAX;
            return Ok(());
        }

        let Some(task_id) = task_id_for_current_task_ref(task_ref) else {
            return self.failed_setup();
        };
        self.dequeue_task(task_id)
    }

    pub fn replace_user_task_ref(
        &mut self,
        runqueue_ref: RunQueueRef,
        previous: TaskRef,
        next: TaskRef,
        next_pid: usize,
    ) -> EventResult {
        if !runqueue_ref.matches_cpu_owned_runqueue(self.cpu_id())
            || !previous.is_user()
            || !next.is_user()
            || next_pid < USER_CHILD_PID
            || !self.user_child_task_enqueued
            || !self.user_child_task_ref.same_identity(previous)
        {
            return self.failed_setup();
        }
        self.user_child_task_ref = next;
        self.user_child_task_pid = next_pid;
        self.enqueued_task_id = next_pid;
        Ok(())
    }

    fn dequeue_task(&mut self, task_id: usize) -> EventResult {
        if self.lifecycle.state() != State::Ready || !self.contains_task(task_id) {
            return self.failed_setup();
        }

        if task_id == crate::objects::rest_init::KERNEL_INIT_PID {
            self.kernel_init_task_enqueued = false;
        }
        if task_id == crate::objects::rest_init::KTHREADD_PID {
            self.kthreadd_task_enqueued = false;
        }
        if task_id == USER_CHILD_PID {
            self.user_child_task_enqueued = false;
            self.user_child_task_ref = TaskRef::NONE;
            self.user_child_task_pid = 0;
        }
        if task_id == SMOKE_SCHEDULER_TASK_ID {
            self.smoke_scheduler_task_enqueued = false;
        }
        if task_id == SMOKE_MUTEX_TASK_ID {
            self.smoke_mutex_task_enqueued = false;
        }
        if task_id == SMOKE_RWSEM_TASK_ID {
            self.smoke_rwsem_task_enqueued = false;
        }
        if task_id == SMOKE_RWLOCK_TASK_ID {
            self.smoke_rwlock_task_enqueued = false;
        }
        self.enqueued_task_id = usize::MAX;
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

    fn failed_setup_error(&self) -> EventError {
        EventError::failed(
            EventErrorCode::ConditionFailed,
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Base,
            State::Ready,
        )
    }
}

pub struct BootIdleSetupState {
    lifecycle: Lifecycle,
    pi_lock: RawSpinLock,
    cpu_ref: CpuRef,
    uses_current_init_task: bool,
    lazy_tlb_mm_ready: bool,
    no_set_affinity: bool,
    init_held_pi_lock: bool,
    init_held_runqueue_lock: bool,
    cpu_set_under_rcu_read: bool,
    runqueue_current_published_with_rcu: bool,
}

impl BootIdleSetupState {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            pi_lock: RawSpinLock::new(),
            cpu_ref: CpuRef::invalid(),
            uses_current_init_task: false,
            lazy_tlb_mm_ready: false,
            no_set_affinity: false,
            init_held_pi_lock: false,
            init_held_runqueue_lock: false,
            cpu_set_under_rcu_read: false,
            runqueue_current_published_with_rcu: false,
        }
    }

    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn pi_lock(&self) -> &RawSpinLock {
        &self.pi_lock
    }

    pub const fn task_id(&self) -> usize {
        0
    }

    pub fn cpu_id(&self) -> usize {
        self.cpu_ref.logical_id()
    }

    pub const fn init_held_pi_lock(&self) -> bool {
        self.init_held_pi_lock
    }

    pub const fn init_held_runqueue_lock(&self) -> bool {
        self.init_held_runqueue_lock
    }

    pub const fn cpu_set_under_rcu_read(&self) -> bool {
        self.cpu_set_under_rcu_read
    }

    pub const fn runqueue_current_published_with_rcu(&self) -> bool {
        self.runqueue_current_published_with_rcu
    }

    pub fn view(&self) -> Option<CpuIdleTaskView> {
        if self.lifecycle.state() != State::Ready
            || self.cpu_ref == CpuRef::invalid()
            || self.cpu_ref.logical_id() == usize::MAX
        {
            return None;
        }

        Some(CpuIdleTaskView {
            state: self.lifecycle.state(),
            task_id: BootTask::canonical_pid(),
            cpu_ref: self.cpu_ref,
            cpu_id: self.cpu_ref.logical_id(),
            uses_current_init_task: self.uses_current_init_task,
            lazy_tlb_mm_ready: self.lazy_tlb_mm_ready,
            no_set_affinity: self.no_set_affinity,
            switch_ctx_ra: BootTask::canonical_switch_context().ra(),
            switch_ctx_initialized: BootTask::canonical_switch_context().initialized(),
            core_saved_count: BootTask::canonical_task()
                .thread_context()
                .core_saved_count(),
            core_restored_count: BootTask::canonical_task()
                .thread_context()
                .core_restored_count(),
        })
    }

    pub fn is_boot_cpu_idle_task_view(
        &self,
        cpu_group: &CpuGroup,
        boot_runqueue: &BootRunQueue,
    ) -> bool {
        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return false;
        };
        self.lifecycle.state() == State::Ready
            && boot_runqueue.is_boot_cpu_runqueue_view(cpu_group)
            && self.cpu_ref == boot_cpu.cpu_ref()
            && self.cpu_ref == boot_runqueue.cpu_ref()
            && self.cpu_id() == boot_runqueue.cpu_id()
            && self.task_id() == boot_runqueue.idle_task_id()
            && boot_runqueue.curr_task_id() == self.task_id()
    }

    // Boot idle setup binds the specified task, runqueue, RCU and CPU-local guards.
    #[allow(clippy::too_many_arguments)]
    fn setup(
        &mut self,
        boot_task: &mut BootTask,
        init_mm: &InitMm,
        boot_runqueue: &mut BootRunQueue,
        boot_idle_rcu_read_side: &mut RcuReadSide,
        boot_cpu_ref: CpuRef,
        local_interrupt: &mut LocalInterruptControl,
        boot_idle_preemption: &mut PreemptionControl,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.pi_lock.state() != State::Base
            || boot_task.state() != State::OnCpu
            || init_mm.state() != State::Ready
            || boot_runqueue.state() != State::Ready
            || boot_runqueue.lock().state() != State::Ready
            || boot_idle_rcu_read_side.state() != State::Prepared
            || !boot_idle_rcu_read_side.incomplete_first_slice()
            || !boot_idle_rcu_read_side.full_semantics_deferred()
            || local_interrupt.state() != State::Ready
            || boot_idle_preemption.state() != State::Base
            || boot_runqueue.cpu_ref() != boot_cpu_ref
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.pi_lock
            .setup_with_checkpoint(Checkpoint::BootIdlePiLockReady)?;
        boot_idle_preemption.setup_disabled(&*boot_task)?;
        self.pi_lock
            .lock_irqsave(local_interrupt, boot_idle_preemption)?;
        let guarded_result = (|| {
            self.init_held_pi_lock = true;
            boot_runqueue.lock.acquire()?;
            let runqueue_guarded_result = (|| {
                self.init_held_runqueue_lock = true;
                boot_idle_rcu_read_side.read_lock()?;
                if boot_task
                    .bind_idle_metadata(boot_runqueue.cpu_id())
                    .is_err()
                {
                    let _ = boot_idle_rcu_read_side.read_unlock();
                    return failed_condition(
                        LifecycleEvent::Setup,
                        self.lifecycle.state(),
                        State::Base,
                        State::Ready,
                    );
                }
                boot_idle_rcu_read_side.read_unlock()?;
                self.cpu_set_under_rcu_read = true;
                self.cpu_ref = boot_runqueue.cpu_ref();
                self.uses_current_init_task = true;
                self.lazy_tlb_mm_ready = true;
                self.no_set_affinity = true;
                self.runqueue_current_published_with_rcu = true;
                self.lifecycle.transition(
                    LifecycleEvent::Setup,
                    State::Base,
                    State::Ready,
                    Checkpoint::BootIdleSetupReady,
                )
            })();
            let release_result = boot_runqueue.lock.release();
            runqueue_guarded_result.and(release_result)
        })();
        let unlock_result = self
            .pi_lock
            .unlock_irqrestore(local_interrupt, boot_idle_preemption);
        guarded_result.and(unlock_result)
    }
}
