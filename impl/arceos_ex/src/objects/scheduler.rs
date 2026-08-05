use super::{
    boot_task::BootTask,
    cpu::CpuRef,
    cpu_control::{PreemptionControl, RawSpinLock, RcuReadSide},
    cpu_group::{CpuGroup, PossibleCpuInventory},
    current_task::CurrentTask,
    default_sched_root_domain::DefaultSchedRootDomain,
    init_mm::InitMm,
    interrupt_type::InterruptType,
    per_cpu_storage::PerCpuStorage,
    scheduler_task_access::{NextDispatch, SchedulerTaskAccess},
    state::{
        EventError, EventErrorCode, EventResult, FailureDiagnostic, Lifecycle, LifecycleEvent,
        State, failed_condition,
    },
    static_branch::StaticBranch,
    task::{KERNEL_TASK_SLOT_COUNT, Task, TaskEntry, TaskKind, TaskRef, USER_TASK_SLOT_COUNT},
    task_flow::TaskFlowRef,
    trap_type::TrapType,
    user_boot::USER_CHILD_PID,
};
use crate::arch::riscv64::task_switch::{self, TaskSwitchContext};
use crate::checkpoint::Checkpoint;

const SMOKE_SCHEDULER_TASK_ID: usize = 1001;
const SMOKE_MUTEX_TASK_ID: usize = 1002;
const SMOKE_RWSEM_TASK_ID: usize = 1003;
const SMOKE_RWLOCK_TASK_ID: usize = 1004;
const SMOKE_SCHEDULER_STACK_WORDS: usize = 512;
const BIT_WAIT_TABLE_SIZE: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrevDisposition {
    Runnable,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedClassRef {
    Stop,
    Deadline,
    Realtime,
    Fair,
    Idle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PickNextProtocol {
    Combined,
    Fallback,
}

#[derive(Clone, Copy)]
struct SwitchPreflight {
    prev_ref: TaskRef,
    next_ref: TaskRef,
    prev_context: *mut TaskSwitchContext,
    next_context: *const TaskSwitchContext,
    prev_task_id: usize,
    next_task_id: usize,
    next_task_identity: usize,
    trap_entry_context: usize,
    first_boot_handoff: bool,
}

// PID 1 and kthreadd can share the fair queue with every dynamic user and
// kernel Task when fixed placement targets a single CPU.
const SCHED_CLASS_QUEUE_CAPACITY: usize = USER_TASK_SLOT_COUNT + KERNEL_TASK_SLOT_COUNT + 2;

#[derive(Clone, Copy)]
struct SchedClassQueue {
    task_refs: [TaskRef; SCHED_CLASS_QUEUE_CAPACITY],
    task_ids: [usize; SCHED_CLASS_QUEUE_CAPACITY],
    len: usize,
}

impl SchedClassQueue {
    const fn new() -> Self {
        Self {
            task_refs: [TaskRef::NONE; SCHED_CLASS_QUEUE_CAPACITY],
            task_ids: [usize::MAX; SCHED_CLASS_QUEUE_CAPACITY],
            len: 0,
        }
    }

    const fn len(&self) -> usize {
        self.len
    }

    const fn contains_ref(&self, task_ref: TaskRef) -> bool {
        let mut index = 0usize;
        while index < self.len {
            if self.task_refs[index].same_identity(task_ref) {
                return true;
            }
            index += 1;
        }
        false
    }

    const fn contains_id(&self, task_id: usize) -> bool {
        let mut index = 0usize;
        while index < self.len {
            if self.task_ids[index] == task_id {
                return true;
            }
            index += 1;
        }
        false
    }

    const fn id_for_ref(&self, task_ref: TaskRef) -> Option<usize> {
        let mut index = 0usize;
        while index < self.len {
            if self.task_refs[index].same_identity(task_ref) {
                return Some(self.task_ids[index]);
            }
            index += 1;
        }
        None
    }

    fn enqueue(&mut self, task_ref: TaskRef, task_id: usize) -> bool {
        if self.len == SCHED_CLASS_QUEUE_CAPACITY
            || !task_ref.is_valid()
            || self.contains_ref(task_ref)
            || self.contains_id(task_id)
        {
            return false;
        }
        self.task_refs[self.len] = task_ref;
        self.task_ids[self.len] = task_id;
        self.len += 1;
        true
    }

    fn dequeue_ref(&mut self, task_ref: TaskRef) -> bool {
        let mut index = 0usize;
        while index < self.len && !self.task_refs[index].same_identity(task_ref) {
            index += 1;
        }
        if index == self.len {
            return false;
        }
        while index + 1 < self.len {
            self.task_refs[index] = self.task_refs[index + 1];
            self.task_ids[index] = self.task_ids[index + 1];
            index += 1;
        }
        self.len -= 1;
        self.task_refs[self.len] = TaskRef::NONE;
        self.task_ids[self.len] = usize::MAX;
        true
    }

    fn replace_ref(&mut self, previous: TaskRef, next: TaskRef, next_id: usize) -> bool {
        if !next.is_valid() || self.contains_ref(next) {
            return false;
        }
        let mut index = 0usize;
        while index < self.len && !self.task_refs[index].same_identity(previous) {
            index += 1;
        }
        if index == self.len {
            return false;
        }
        self.task_refs[index] = next;
        self.task_ids[index] = next_id;
        true
    }

    const fn pick_after(&self, prev_ref: TaskRef) -> TaskRef {
        if self.len == 0 {
            return TaskRef::NONE;
        }
        let mut index = 0usize;
        while index < self.len {
            if self.task_refs[index].same_identity(prev_ref) {
                return self.task_refs[(index + 1) % self.len];
            }
            index += 1;
        }
        self.task_refs[0]
    }
}

// Scheduler diagnostic views and local-subject helpers are consumed by smoke/KUnit cases.
#[cfg_attr(not(app_smoke), allow(dead_code))]
pub struct Scheduler {
    lifecycle: Lifecycle,
    boot_idle_rcu_read_side: RcuReadSide,
    runqueue_ready: bool,
    lock: RawSpinLock,
    cpu_ref: CpuRef,
    cpu_hartid: usize,
    curr: TaskRef,
    idle: TaskRef,
    stop: TaskRef,
    curr_task_id: usize,
    idle_task_id: usize,
    class_queues_ready: bool,
    stop_queue: SchedClassQueue,
    deadline_queue: SchedClassQueue,
    realtime_queue: SchedClassQueue,
    fair_queue: SchedClassQueue,
    idle_queue: SchedClassQueue,
    attached_to_root_domain: bool,
    root_attach_held_runqueue_lock: bool,
    balance_push_enabled: bool,
    enqueued_task_id: usize,
    boot_init_preemption: PreemptionControl,
    boot_idle_setup_state: BootIdleSetupState,
    boot_idle_preemption: PreemptionControl,
    scheduler_running: bool,
    task_stack_switching_online: bool,
    selected_runqueue_task_id: usize,
    schedule_passes: usize,
    current_runqueue_resolve_passes: usize,
    pick_next_task_passes: usize,
    combined_pick_passes: usize,
    fallback_pick_passes: usize,
    pick_task_passes: usize,
    put_prev_task_passes: usize,
    set_next_task_passes: usize,
    class_protocol_sequence: usize,
    prepare_prev_sequence: usize,
    pick_task_sequence: usize,
    put_prev_task_sequence: usize,
    set_next_task_sequence: usize,
    last_picked_class: Option<SchedClassRef>,
    prepare_prev_passes: usize,
    prepare_prev_blocked_passes: usize,
    last_prev_disposition: PrevDisposition,
    switch_to_passes: usize,
    identity_switch_passes: usize,
    task_dispatch_signal_passes: usize,
    flow_enter_signal_passes: usize,
    switch_preflight_passes: usize,
    switch_protocol_sequence: usize,
    save_core_context_sequence: usize,
    suspend_task_sequence: usize,
    restore_core_context_sequence: usize,
    finish_task_switch_sequence: usize,
    task_dispatch_sequence: usize,
    flow_signal_sequence: usize,
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
    switch_stack_mismatch_count: usize,
    switch_stack_mismatch_prev_ref: TaskRef,
    switch_stack_mismatch_live_sp: usize,
    switch_stack_mismatch_task_base: usize,
    switch_stack_mismatch_task_top: usize,
    switch_stack_mismatch_entry_base: usize,
    switch_stack_mismatch_entry_top: usize,
    physical_switch_pending: bool,
    schedule_guard_pending: bool,
    switch_prev_committed_online: bool,
    next_dispatch: Option<NextDispatch>,
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
}

/// Test-only task stacks are process-wide fixtures, not per-CPU scheduler
/// state.  Keeping them outside `Scheduler` prevents every possible CPU from
/// carrying another copy of the four smoke stacks.
#[cfg_attr(not(app_smoke), allow(dead_code))]
pub struct SchedulerTestStacks {
    scheduler: [usize; SMOKE_SCHEDULER_STACK_WORDS],
    mutex: [usize; SMOKE_SCHEDULER_STACK_WORDS],
    rwsem: [usize; SMOKE_SCHEDULER_STACK_WORDS],
    rwlock: [usize; SMOKE_SCHEDULER_STACK_WORDS],
}

/// Process-wide smoke tasks. They exercise the scheduler but are not part of
/// any CPU-local Scheduler object.
#[cfg_attr(not(app_smoke), allow(dead_code))]
pub struct SchedulerTestTasks {
    smoke_scheduler_task: SmokeSchedulerTask,
    smoke_mutex_task: SmokeSchedulerTask,
    smoke_rwsem_task: SmokeSchedulerTask,
    smoke_rwlock_task: SmokeSchedulerTask,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl SchedulerTestTasks {
    pub const fn new() -> Self {
        Self {
            smoke_scheduler_task: SmokeSchedulerTask::new(SMOKE_SCHEDULER_TASK_ID),
            smoke_mutex_task: SmokeSchedulerTask::new(SMOKE_MUTEX_TASK_ID),
            smoke_rwsem_task: SmokeSchedulerTask::new(SMOKE_RWSEM_TASK_ID),
            smoke_rwlock_task: SmokeSchedulerTask::new(SMOKE_RWLOCK_TASK_ID),
        }
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
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl SchedulerTestStacks {
    pub const fn new() -> Self {
        Self {
            scheduler: [0; SMOKE_SCHEDULER_STACK_WORDS],
            mutex: [0; SMOKE_SCHEDULER_STACK_WORDS],
            rwsem: [0; SMOKE_SCHEDULER_STACK_WORDS],
            rwlock: [0; SMOKE_SCHEDULER_STACK_WORDS],
        }
    }
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl Scheduler {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            boot_idle_rcu_read_side: RcuReadSide::new(),
            runqueue_ready: false,
            lock: RawSpinLock::new(),
            cpu_ref: CpuRef::invalid(),
            cpu_hartid: usize::MAX,
            curr: TaskRef::NONE,
            idle: TaskRef::NONE,
            stop: TaskRef::NONE,
            curr_task_id: usize::MAX,
            idle_task_id: usize::MAX,
            class_queues_ready: false,
            stop_queue: SchedClassQueue::new(),
            deadline_queue: SchedClassQueue::new(),
            realtime_queue: SchedClassQueue::new(),
            fair_queue: SchedClassQueue::new(),
            idle_queue: SchedClassQueue::new(),
            attached_to_root_domain: false,
            root_attach_held_runqueue_lock: false,
            balance_push_enabled: true,
            enqueued_task_id: usize::MAX,
            boot_init_preemption: PreemptionControl::new(),
            boot_idle_setup_state: BootIdleSetupState::new(),
            boot_idle_preemption: PreemptionControl::new(),
            scheduler_running: false,
            task_stack_switching_online: false,
            selected_runqueue_task_id: usize::MAX,
            schedule_passes: 0,
            current_runqueue_resolve_passes: 0,
            pick_next_task_passes: 0,
            combined_pick_passes: 0,
            fallback_pick_passes: 0,
            pick_task_passes: 0,
            put_prev_task_passes: 0,
            set_next_task_passes: 0,
            class_protocol_sequence: 0,
            prepare_prev_sequence: 0,
            pick_task_sequence: 0,
            put_prev_task_sequence: 0,
            set_next_task_sequence: 0,
            last_picked_class: None,
            prepare_prev_passes: 0,
            prepare_prev_blocked_passes: 0,
            last_prev_disposition: PrevDisposition::Runnable,
            switch_to_passes: 0,
            identity_switch_passes: 0,
            task_dispatch_signal_passes: 0,
            flow_enter_signal_passes: 0,
            switch_preflight_passes: 0,
            switch_protocol_sequence: 0,
            save_core_context_sequence: 0,
            suspend_task_sequence: 0,
            restore_core_context_sequence: 0,
            finish_task_switch_sequence: 0,
            task_dispatch_sequence: 0,
            flow_signal_sequence: 0,
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
            switch_stack_mismatch_count: 0,
            switch_stack_mismatch_prev_ref: TaskRef::NONE,
            switch_stack_mismatch_live_sp: 0,
            switch_stack_mismatch_task_base: 0,
            switch_stack_mismatch_task_top: 0,
            switch_stack_mismatch_entry_base: 0,
            switch_stack_mismatch_entry_top: 0,
            physical_switch_pending: false,
            schedule_guard_pending: false,
            switch_prev_committed_online: false,
            next_dispatch: None,
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

    pub const fn boot_idle_rcu_read_side(&self) -> &RcuReadSide {
        &self.boot_idle_rcu_read_side
    }

    pub fn boot_idle_rcu_read_side_mut(&mut self) -> &mut RcuReadSide {
        &mut self.boot_idle_rcu_read_side
    }

    pub const fn runqueue_state(&self) -> State {
        if self.runqueue_ready {
            State::Ready
        } else {
            State::Base
        }
    }

    pub const fn boot_runqueue_lock(&self) -> &RawSpinLock {
        &self.lock
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

    pub fn boot_cpu_owned_scheduler_view(
        &self,
        cpu_group: &CpuGroup,
    ) -> Option<CpuOwnedSchedulerView> {
        let boot_cpu = cpu_group.boot_cpu()?;
        let runqueue = self.runqueue_view(true)?;
        let idle_task = self.boot_idle_setup_state.view()?;
        if cpu_group.state() != State::Ready
            || !boot_cpu.cpu_ref().is_boot_cpu()
            || !self.boot_idle_setup_state.is_boot_cpu_idle_task_view(
                cpu_group,
                self.runqueue_ready,
                self.cpu_ref,
                self.cpu_hartid,
                self.idle_task_id,
            )
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
            runqueue_current_task_ref: self.curr_ref(),
            runqueue_current_task_id: self.curr_task_id(),
            runqueue_idle_task_id: self.idle_task_id(),
            runqueue_task_count: self.task_count(),
            runqueue_kernel_init_task_enqueued: self
                .contains_task(crate::objects::rest_init::KERNEL_INIT_PID),
            runqueue_kthreadd_task_enqueued: self
                .contains_task(crate::objects::rest_init::KTHREADD_PID),
            runqueue_user_child_task_enqueued: self.contains_task(USER_CHILD_PID),
            runqueue_smoke_scheduler_task_enqueued: self.contains_task(SMOKE_SCHEDULER_TASK_ID),
            runqueue_smoke_mutex_task_enqueued: self.contains_task(SMOKE_MUTEX_TASK_ID),
            runqueue_smoke_rwsem_task_enqueued: self.contains_task(SMOKE_RWSEM_TASK_ID),
            runqueue_smoke_rwlock_task_enqueued: self.contains_task(SMOKE_RWLOCK_TASK_ID),
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

    pub const fn combined_pick_passes(&self) -> usize {
        self.combined_pick_passes
    }

    pub const fn fallback_pick_passes(&self) -> usize {
        self.fallback_pick_passes
    }

    pub const fn pick_task_passes(&self) -> usize {
        self.pick_task_passes
    }

    pub const fn put_prev_task_passes(&self) -> usize {
        self.put_prev_task_passes
    }

    pub const fn set_next_task_passes(&self) -> usize {
        self.set_next_task_passes
    }

    pub const fn prepare_prev_sequence(&self) -> usize {
        self.prepare_prev_sequence
    }

    pub const fn pick_task_sequence(&self) -> usize {
        self.pick_task_sequence
    }

    pub const fn put_prev_task_sequence(&self) -> usize {
        self.put_prev_task_sequence
    }

    pub const fn set_next_task_sequence(&self) -> usize {
        self.set_next_task_sequence
    }

    pub const fn last_picked_class(&self) -> Option<SchedClassRef> {
        self.last_picked_class
    }

    pub const fn prepare_prev_passes(&self) -> usize {
        self.prepare_prev_passes
    }

    pub const fn prepare_prev_blocked_passes(&self) -> usize {
        self.prepare_prev_blocked_passes
    }

    pub const fn last_prev_disposition(&self) -> PrevDisposition {
        self.last_prev_disposition
    }

    pub const fn switch_to_passes(&self) -> usize {
        self.switch_to_passes
    }

    pub const fn identity_switch_passes(&self) -> usize {
        self.identity_switch_passes
    }

    pub const fn task_dispatch_signal_passes(&self) -> usize {
        self.task_dispatch_signal_passes
    }

    pub const fn flow_enter_signal_passes(&self) -> usize {
        self.flow_enter_signal_passes
    }

    pub const fn switch_preflight_passes(&self) -> usize {
        self.switch_preflight_passes
    }

    pub const fn save_core_context_sequence(&self) -> usize {
        self.save_core_context_sequence
    }

    pub const fn suspend_task_sequence(&self) -> usize {
        self.suspend_task_sequence
    }

    pub const fn restore_core_context_sequence(&self) -> usize {
        self.restore_core_context_sequence
    }

    pub const fn finish_task_switch_sequence(&self) -> usize {
        self.finish_task_switch_sequence
    }

    pub const fn task_dispatch_sequence(&self) -> usize {
        self.task_dispatch_sequence
    }

    pub const fn flow_signal_sequence(&self) -> usize {
        self.flow_signal_sequence
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

    pub fn preset(
        &mut self,
        per_cpu_storage: &PerCpuStorage,
        static_branch: &StaticBranch,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || per_cpu_storage.state() != State::Ready
            || static_branch.state() != State::Ready
        {
            return self.failed_preset();
        }

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
        inventory: PossibleCpuInventory,
        per_cpu_storage: &PerCpuStorage,
        root_domain: &DefaultSchedRootDomain,
        boot_task: &mut BootTask,
        init_mm: &InitMm,
        local_interrupt: &mut InterruptType,
    ) -> EventResult {
        let Some(boot_cpu_ref) = inventory.cpu_ref(0) else {
            return self.failed_setup();
        };
        let Some(boot_cpu_hartid) = inventory.hartid(0) else {
            return self.failed_setup();
        };
        if self.lifecycle.state() != State::Prepared
            || root_domain.state() != State::Ready
            || self.boot_idle_rcu_read_side.state() != State::Prepared
            || inventory.count() == 0
            || per_cpu_storage.state() != State::Ready
            || boot_task.state() != State::OnCpu
            || init_mm.state() != State::Ready
            || local_interrupt.state() != State::Ready
        {
            return self.failed_setup();
        }

        self.setup_runqueue(
            boot_cpu_ref,
            boot_cpu_hartid,
            per_cpu_storage,
            root_domain,
            &*boot_task,
            local_interrupt,
        )?;
        self.boot_idle_setup_state.setup(
            boot_task,
            init_mm,
            &mut self.lock,
            self.runqueue_ready,
            self.cpu_ref,
            &mut self.boot_idle_rcu_read_side,
            boot_cpu_ref,
            local_interrupt,
            &mut self.boot_idle_preemption,
        )?;
        if !self.setup_facts_hold(inventory, root_domain) {
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

    /// Prepares the CPU-local scheduler owned by a possible secondary CPU.
    /// It remains Ready until that CPU's online acknowledgement enables it.
    pub fn setup_secondary(
        &mut self,
        cpu_ref: CpuRef,
        cpu_hartid: usize,
        root_domain: &DefaultSchedRootDomain,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || cpu_ref.is_boot_cpu()
            || !root_domain.covers_cpu_ref(cpu_ref)
        {
            return self.failed_preset();
        }
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::SchedulerPrepared,
        )?;
        self.setup_secondary_runqueue(cpu_ref, cpu_hartid, root_domain)?;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::SchedulerReady,
        )
    }

    pub fn enable_secondary(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready || self.cpu_ref().is_boot_cpu() {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }
        self.boot_idle_preemption.adopt_secondary_ready()?;
        // The HSM architectural entry is the one allowed non-contextual AP
        // entry. Every scheduler switch from this point uses saved contexts.
        self.task_stack_switching_online = true;
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
        sender_flow_ref: TaskFlowRef,
        current_task_ref: TaskRef,
        current_cpu_ref: CpuRef,
        task_access: &mut SchedulerTaskAccess<'_>,
        local_interrupt: &mut InterruptType,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online {
            return Err(self.failed_schedule_entry("scheduler-online"));
        }
        if !self.scheduler_running {
            return Err(self.failed_schedule_entry("scheduler-running"));
        }
        if !self.idle_binding_valid() {
            return Err(self.failed_schedule_entry("idle-task-binding"));
        }
        if local_interrupt.local_state() != State::Ready {
            return Err(self.failed_schedule_entry("local-interrupt-ready"));
        }
        if !current_task_ref.is_scheduler_ref() {
            return Err(self.failed_schedule_entry("current-task-ref-resolves"));
        }
        if current_cpu_ref != self.cpu_ref() {
            return Err(self.failed_schedule_entry("sender-cpu-matches-scheduler"));
        }
        if !self.curr_ref().same_identity(current_task_ref) {
            return Err(self.failed_schedule_entry("scheduler-curr-matches-sender-task"));
        }
        if !task_access.schedule_sender_matches(sender_flow_ref, current_task_ref, self.cpu_ref()) {
            return Err(self.failed_schedule_entry("active-flow-is-exact-sender"));
        }
        if self.schedule_guard_pending {
            return Err(self.failed_schedule_entry("no-pending-schedule-guard"));
        }

        let prev_ref = current_task_ref;
        let mut next_ref = TaskRef::NONE;
        let mut switch_preflight = None;
        let stack_mismatch_count_before = self.switch_stack_mismatch_count;

        self.boot_idle_preemption
            .disable()
            .map_err(|error| schedule_stage(error, "PreemptDisable"))?;
        self.schedule_preemption_disable_count =
            self.schedule_preemption_disable_count.wrapping_add(1);
        local_interrupt
            .save_and_disable()
            .map_err(|error| schedule_stage(error, "LocalIrqSaveDisable"))?;
        self.schedule_guard_pending = true;
        let guarded_result = (|| {
            self.scheduler_rcu_context_switch_count =
                self.scheduler_rcu_context_switch_count.wrapping_add(1);
            let current_scheduler_ref = self
                .resolve_current_scheduler_ref(current_cpu_ref, task_access, prev_ref)
                .map_err(|error| schedule_stage(error, "ResolveCurrentRunqueue"))?;
            self.lock
                .lock_irqsave(local_interrupt, &mut self.boot_idle_preemption)
                .map_err(|error| schedule_stage(error, "RunqueueLock"))?;
            let runqueue_result = (|| {
                self.scheduler_rq_lock_mb_after_spinlock_count = self
                    .scheduler_rq_lock_mb_after_spinlock_count
                    .wrapping_add(1);
                self.scheduler_rq_clock_update_count =
                    self.scheduler_rq_clock_update_count.wrapping_add(1);
                let disposition = self
                    .prepare_prev(prev_ref, task_access)
                    .map_err(|error| schedule_stage(error, "PreparePrev"))?;
                next_ref = self
                    .pick_next_task(current_scheduler_ref, prev_ref, disposition)
                    .map_err(|error| schedule_stage(error, "PickNext"))?;
                self.scheduler_need_resched_clear_count =
                    self.scheduler_need_resched_clear_count.wrapping_add(1);
                self.scheduler_rq_curr_publish_rcu_count =
                    self.scheduler_rq_curr_publish_rcu_count.wrapping_add(1);
                self.scheduler_trace_sched_switch_count =
                    self.scheduler_trace_sched_switch_count.wrapping_add(1);
                if next_ref != prev_ref {
                    switch_preflight =
                        Some(self.switch_to(prev_ref, next_ref, current_task_ref, task_access)?);
                } else {
                    self.identity_switch_passes = self.identity_switch_passes.wrapping_add(1);
                    if !self.cpu_ref().is_boot_cpu() {
                        crate::objects::kernel_task::record_identity_schedule(self.cpu_id());
                    }
                }
                self.schedule_passes = self.schedule_passes.wrapping_add(1);
                crate::checkpoint::checkpoint(Checkpoint::SchedulerSchedule);
                Ok(())
            })();
            let unlock_result = self
                .lock
                .unlock_irqrestore(local_interrupt, &mut self.boot_idle_preemption)
                .map_err(|error| schedule_stage(error, "RunqueueUnlock"));
            if runqueue_result.is_ok() && unlock_result.is_ok() {
                self.scheduler_finish_released_rq_lock_count =
                    self.scheduler_finish_released_rq_lock_count.wrapping_add(1);
            }
            runqueue_result.and(unlock_result)
        })();
        if let Err(error) = guarded_result {
            let finish_result = self
                .complete_schedule_guard(local_interrupt)
                .map_err(|finish| schedule_stage(finish, "RejectedGuardRestore"));
            if self.switch_stack_mismatch_count != stack_mismatch_count_before {
                self.trace_switch_stack_mismatch();
            }
            return Err::<(), EventError>(error).and(finish_result);
        }
        self.schedule_exit_prev_ref = prev_ref;
        self.schedule_exit_next_ref = next_ref;
        self.schedule_exit_saved_interrupt_count = local_interrupt.saved_and_disabled_count();
        if let Some(preflight) = switch_preflight {
            if let Err(error) = self.save_and_suspend_task(prev_ref, task_access) {
                let error = schedule_stage(error, "SaveSuspendPrev");
                let finish_result = self
                    .complete_schedule_guard(local_interrupt)
                    .map_err(|finish| schedule_stage(finish, "SaveSuspendGuardRestore"));
                return Err::<(), EventError>(error).and(finish_result);
            }
            let resumed = match self.cooperative_context_switch(preflight) {
                Ok(resumed) => resumed,
                Err(error) => {
                    let error = schedule_stage(error, "ArchitectureContextSwitch");
                    let finish_result = self
                        .complete_schedule_guard(local_interrupt)
                        .map_err(|finish| schedule_stage(finish, "ContextSwitchGuardRestore"));
                    return Err::<(), EventError>(error).and(finish_result);
                }
            };
            if !resumed {
                let error = self
                    .failed_switch_to()
                    .map_err(|error| schedule_stage(error, "ContextSwitchDidNotResume"));
                let finish_result = self
                    .complete_schedule_guard(local_interrupt)
                    .map_err(|finish| schedule_stage(finish, "NoResumeGuardRestore"));
                return error.and(finish_result);
            }
            let dispatch_result = self
                .dispatch_task_after_switch(prev_ref, task_access)
                .map_err(|error| schedule_stage(error, "DispatchRestoredPrev"));
            let finish_result = self
                .complete_schedule_guard(local_interrupt)
                .map_err(|finish| schedule_stage(finish, "DispatchGuardRestore"));
            dispatch_result.and(finish_result)?;
        } else {
            self.complete_schedule_guard(local_interrupt)
                .map_err(|error| schedule_stage(error, "IdentityGuardRestore"))?;
        }
        Ok(())
    }

    pub(crate) fn complete_schedule_guard(
        &mut self,
        local_interrupt: &mut InterruptType,
    ) -> EventResult {
        if !self.schedule_guard_pending {
            return Err(self.failed_schedule_entry("schedule-guard-pending"));
        }
        let restore_result = local_interrupt.restore();
        let enable_result = self.boot_idle_preemption.enable_no_resched();
        if restore_result.is_ok() && enable_result.is_ok() {
            self.schedule_guard_pending = false;
            self.schedule_preemption_enable_no_resched_count = self
                .schedule_preemption_enable_no_resched_count
                .wrapping_add(1);
            self.scheduler_finish_preempt_count_restore_count = self
                .scheduler_finish_preempt_count_restore_count
                .wrapping_add(1);
            self.schedule_exit_restored_interrupt_count = local_interrupt.restored_count();
        }
        restore_result.and(enable_result)
    }

    pub(crate) const fn schedule_guard_pending(&self) -> bool {
        self.schedule_guard_pending
    }

    pub fn schedule_idle(
        &mut self,
        sender_flow_ref: TaskFlowRef,
        current_task_ref: TaskRef,
        current_cpu_ref: CpuRef,
        task_access: &mut SchedulerTaskAccess<'_>,
        local_interrupt: &mut InterruptType,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || !current_task_ref.same_identity(self.idle_ref())
        {
            return Err(self.failed_schedule_condition());
        }

        self.schedule(
            sender_flow_ref,
            current_task_ref,
            current_cpu_ref,
            task_access,
            local_interrupt,
        )?;
        self.idle_schedule_passes = self.idle_schedule_passes.wrapping_add(1);
        self.idle_schedule_returned_passes = self.idle_schedule_returned_passes.wrapping_add(1);
        if task_access
            .task_on_cpu_identity_matches(self.idle_ref(), crate::arch::riscv64::csr::read_tp())
        {
            self.idle_schedule_identity_passes = self.idle_schedule_identity_passes.wrapping_add(1);
        }
        Ok(())
    }

    fn resolve_current_scheduler_ref(
        &mut self,
        current_cpu_ref: CpuRef,
        task_access: &SchedulerTaskAccess<'_>,
        current_task_ref: TaskRef,
    ) -> Result<CpuRef, EventError> {
        let Some(cpu_ref) = task_access.effective_cpu_ref(current_task_ref) else {
            return Err(self.failed_schedule_condition());
        };
        if current_cpu_ref != cpu_ref {
            return Err(self.failed_schedule_condition());
        }
        if cpu_ref != self.cpu_ref() {
            return Err(self.failed_schedule_condition());
        }

        self.current_runqueue_resolve_passes = self.current_runqueue_resolve_passes.wrapping_add(1);
        Ok(cpu_ref)
    }

    fn prepare_prev(
        &mut self,
        prev_ref: TaskRef,
        task_access: &mut SchedulerTaskAccess<'_>,
    ) -> Result<PrevDisposition, EventError> {
        crate::checkpoint::checkpoint(Checkpoint::SchedulerPreparePrevEntry);
        self.pick_task_sequence = 0;
        self.put_prev_task_sequence = 0;
        self.set_next_task_sequence = 0;
        let runnable = task_access
            .prepare_prev_runnable(prev_ref)
            .ok_or_else(|| self.failed_schedule_condition())?;

        let disposition = if runnable {
            PrevDisposition::Runnable
        } else {
            self.deactivate_task(prev_ref, task_access)?;
            PrevDisposition::Blocked
        };
        self.prepare_prev_passes = self.prepare_prev_passes.wrapping_add(1);
        if disposition == PrevDisposition::Blocked {
            self.prepare_prev_blocked_passes = self.prepare_prev_blocked_passes.wrapping_add(1);
        }
        self.last_prev_disposition = disposition;
        self.class_protocol_sequence = self.class_protocol_sequence.wrapping_add(1);
        self.prepare_prev_sequence = self.class_protocol_sequence;
        crate::checkpoint::checkpoint(Checkpoint::SchedulerPreparePrevExit);
        Ok(disposition)
    }

    fn deactivate_task(
        &mut self,
        task_ref: TaskRef,
        task_access: &mut SchedulerTaskAccess<'_>,
    ) -> EventResult {
        let scheduler_ref = self.cpu_ref();
        if self.contains_task_ref(task_ref) {
            self.dequeue_task_ref(scheduler_ref, task_ref)?;
        } else if task_ref != TaskRef::BOOT {
            return self.failed_switch_to();
        }
        task_access
            .deactivate(task_ref)
            .ok_or_else(|| self.failed_schedule_condition())??;
        crate::checkpoint::checkpoint(Checkpoint::SchedulerDeactivateTask);
        Ok(())
    }

    fn pick_next_task(
        &mut self,
        current_scheduler_ref: CpuRef,
        prev_ref: TaskRef,
        disposition: PrevDisposition,
    ) -> Result<TaskRef, EventError> {
        if current_scheduler_ref != self.cpu_ref()
            || !prev_ref.is_scheduler_ref()
            || !self.idle_binding_valid()
        {
            return Err(self.failed_schedule_condition());
        }

        self.class_protocol_sequence = self.class_protocol_sequence.wrapping_add(1);
        self.pick_task_sequence = self.class_protocol_sequence;
        self.pick_task_passes = self.pick_task_passes.wrapping_add(1);
        let candidate = self
            .pick_task(current_scheduler_ref, prev_ref, disposition)
            .map_err(|_| self.failed_schedule_condition())?;
        let protocol = if self.task_class(prev_ref) == Some(SchedClassRef::Fair)
            && self.task_class(candidate) == Some(SchedClassRef::Fair)
        {
            PickNextProtocol::Combined
        } else {
            PickNextProtocol::Fallback
        };
        let outcome = self
            .pick_next_task_with_protocol(current_scheduler_ref, prev_ref, disposition, protocol)
            .map_err(|_| self.failed_schedule_condition())?;
        let next_ref = outcome.next_ref;
        match outcome.protocol {
            PickNextProtocol::Combined => {
                self.combined_pick_passes = self.combined_pick_passes.wrapping_add(1);
            }
            PickNextProtocol::Fallback => {
                self.fallback_pick_passes = self.fallback_pick_passes.wrapping_add(1);
            }
        }
        if outcome.put_prev_done {
            self.class_protocol_sequence = self.class_protocol_sequence.wrapping_add(1);
            self.put_prev_task_sequence = self.class_protocol_sequence;
            self.put_prev_task_passes = self.put_prev_task_passes.wrapping_add(1);
        }
        if outcome.set_next_done {
            self.class_protocol_sequence = self.class_protocol_sequence.wrapping_add(1);
            self.set_next_task_sequence = self.class_protocol_sequence;
            self.set_next_task_passes = self.set_next_task_passes.wrapping_add(1);
        }
        self.last_picked_class = self.task_class(next_ref);

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

    fn switch_to(
        &mut self,
        prev_ref: TaskRef,
        next_ref: TaskRef,
        current_task_ref: TaskRef,
        task_access: &mut SchedulerTaskAccess<'_>,
    ) -> Result<SwitchPreflight, EventError> {
        if !prev_ref.is_scheduler_ref() {
            return Err(self.failed_switch_preflight("prev-ref-resolves"));
        }
        if !next_ref.is_scheduler_ref() {
            return Err(self.failed_switch_preflight("next-ref-resolves"));
        }
        if prev_ref.same_identity(next_ref) {
            return Err(self.failed_switch_preflight("prev-next-do-not-alias"));
        }
        if !self.curr_ref().same_identity(prev_ref) {
            return Err(self.failed_switch_preflight("scheduler-curr-matches-prev"));
        }
        if !self.idle_binding_valid() {
            return Err(self.failed_switch_preflight("idle-binding-is-current"));
        }
        if current_task_ref != prev_ref {
            return Err(self.failed_switch_preflight("current-task-binding-matches-prev"));
        }
        if self.physical_switch_pending {
            return Err(self.failed_switch_preflight("no-switch-already-pending"));
        }

        let Some(prev_context) = task_access.switch_context_mut_ptr(prev_ref) else {
            return Err(self.failed_switch_preflight("prev-context-resolves"));
        };
        let Some(next_context) = task_access.switch_context_ptr(next_ref) else {
            return Err(self.failed_switch_preflight("next-context-resolves"));
        };
        let Some(prev_task_identity) = task_access.task_identity_ptr(prev_ref) else {
            return Err(self.failed_switch_preflight("prev-task-identity-resolves"));
        };
        let Some(next_task_identity) = task_access.task_identity_ptr(next_ref) else {
            return Err(self.failed_switch_preflight("next-task-identity-resolves"));
        };
        let Some(prev_task_id) = task_access.task_id(prev_ref) else {
            return Err(self.failed_switch_preflight("prev-task-id-resolves"));
        };
        let Some(next_task_id) = task_access.task_id(next_ref) else {
            return Err(self.failed_switch_preflight("next-task-id-resolves"));
        };
        if self.task_id_for_ref(next_ref) != Some(next_task_id) {
            return Err(self.failed_switch_preflight("next-queue-task-id-matches"));
        }
        if self
            .task_id_for_ref(prev_ref)
            .is_some_and(|queued_id| queued_id != prev_task_id)
        {
            return Err(self.failed_switch_preflight("prev-queued-task-id-matches"));
        }
        let trap_entry_context = TrapType::installed_entry_context_address(self.cpu_id());
        let first_boot_handoff = !self.task_stack_switching_online;
        if !task_access.switch_out_ready(prev_ref) {
            return Err(self.failed_switch_preflight("prev-switch-out-ready"));
        }
        let Some(next_dispatch) = task_access.preflight_next_dispatch(next_ref, self.cpu_ref())
        else {
            return Err(self.failed_switch_preflight("next-dispatch-preflight"));
        };
        if prev_context.cast_const() == next_context {
            return Err(self.failed_switch_preflight("context-storage-does-not-alias"));
        }
        if prev_task_identity == next_task_identity {
            return Err(self.failed_switch_preflight("task-storage-does-not-alias"));
        }
        if crate::arch::riscv64::csr::read_tp() != prev_task_identity {
            return Err(self.failed_switch_preflight("tp-binding-matches-prev"));
        }
        let live_sp = crate::arch::riscv64::csr::read_sp();
        if unsafe { !(*prev_context).physical_save_ready(live_sp) } {
            self.switch_stack_mismatch_count = self.switch_stack_mismatch_count.wrapping_add(1);
            self.switch_stack_mismatch_prev_ref = prev_ref;
            self.switch_stack_mismatch_live_sp = live_sp;
            self.switch_stack_mismatch_task_base = unsafe { (*prev_context).kernel_stack_base() };
            self.switch_stack_mismatch_task_top = unsafe { (*prev_context).kernel_stack_top() };
            if trap_entry_context != 0 {
                let entry_context =
                    unsafe { &*(trap_entry_context as *const super::trap_type::TrapEntryContext) };
                self.switch_stack_mismatch_entry_base = entry_context.kernel_stack_base();
                self.switch_stack_mismatch_entry_top = entry_context.kernel_stack_top();
            } else {
                self.switch_stack_mismatch_entry_base = 0;
                self.switch_stack_mismatch_entry_top = 0;
            }
            return Err(self.failed_switch_preflight("prev-context-physical-save-ready"));
        }
        if unsafe { !(*next_context).physical_switch_ready() } {
            return Err(self.failed_switch_preflight("next-context-physical-switch-ready"));
        }
        if trap_entry_context == 0 {
            return Err(self.failed_switch_preflight("trap-entry-context-installed"));
        }
        if first_boot_handoff && prev_ref != TaskRef::BOOT {
            return Err(self.failed_switch_preflight("first-handoff-prev-is-boot"));
        }
        if first_boot_handoff && next_ref != TaskRef::KERNEL_INIT {
            return Err(self.failed_switch_preflight("first-handoff-next-is-kernel-init"));
        }
        if first_boot_handoff && !task_access.first_boot_handoff_preflight_ready(next_dispatch) {
            return Err(self.failed_switch_preflight("first-handoff-task-contracts-ready"));
        }
        if first_boot_handoff && self.kernel_init_stack_switch_started_count != 0 {
            return Err(self.failed_switch_preflight("first-handoff-not-started"));
        }

        self.switch_protocol_sequence = 0;
        self.switch_preflight_passes = self.switch_preflight_passes.wrapping_add(1);
        self.switch_to_entry_prev_ref = prev_ref;
        self.switch_to_entry_next_ref = next_ref;
        self.switch_to_entry_current_ref = current_task_ref;
        self.switch_to_entry_count = self.switch_to_entry_count.wrapping_add(1);
        self.scheduler_prepare_task_switch_count =
            self.scheduler_prepare_task_switch_count.wrapping_add(1);
        self.physical_switch_pending = true;
        self.switch_prev_committed_online = false;
        self.next_dispatch = Some(next_dispatch);
        crate::checkpoint::checkpoint(Checkpoint::SchedulerSwitchToEntry);
        trace_switch_to(
            self.cpu_id(),
            prev_ref,
            prev_task_id,
            next_ref,
            next_task_id,
            current_task_ref,
        );
        self.switch_to_passes = self.switch_to_passes.wrapping_add(1);
        Ok(SwitchPreflight {
            prev_ref,
            next_ref,
            prev_context,
            next_context,
            prev_task_id,
            next_task_id,
            next_task_identity,
            trap_entry_context,
            first_boot_handoff,
        })
    }

    pub(crate) fn prepare_simulated_task_switch(
        &mut self,
        prev_ref: TaskRef,
        next_ref: TaskRef,
        current_task: CurrentTask,
        task_access: &SchedulerTaskAccess<'_>,
    ) -> EventResult {
        if prev_ref == next_ref {
            self.identity_switch_passes = self.identity_switch_passes.wrapping_add(1);
            return Ok(());
        }
        let Some(next_dispatch) =
            task_access.preflight_next_dispatch_on_user_carrier(next_ref, self.cpu_ref())
        else {
            return Err(self.failed_switch_preflight("simulated-next-dispatch-preflight"));
        };
        if current_task.task_ref() != prev_ref {
            return Err(self.failed_switch_preflight("simulated-current-task-matches-prev"));
        }
        if !task_access.switch_out_ready(prev_ref) {
            return Err(self.failed_switch_preflight("simulated-prev-switch-out-ready"));
        }
        self.switch_to_entry_prev_ref = prev_ref;
        self.switch_to_entry_next_ref = next_ref;
        self.switch_to_entry_current_ref = current_task.task_ref();
        self.switch_to_entry_count = self.switch_to_entry_count.wrapping_add(1);
        self.scheduler_prepare_task_switch_count =
            self.scheduler_prepare_task_switch_count.wrapping_add(1);
        self.physical_switch_pending = false;
        self.switch_prev_committed_online = false;
        self.next_dispatch = Some(next_dispatch);
        if !self.publish_current(next_ref) {
            return Err(self.failed_switch_commit("simulated-publish-current"));
        }
        crate::checkpoint::checkpoint(Checkpoint::SchedulerSwitchToEntry);
        Ok(())
    }

    pub(crate) fn suspend_task(
        &mut self,
        task_ref: TaskRef,
        task_access: &mut SchedulerTaskAccess<'_>,
    ) -> EventResult {
        task_access
            .suspend(task_ref)
            .ok_or_else(|| self.failed_schedule_condition())?
    }

    fn save_and_suspend_task(
        &mut self,
        task_ref: TaskRef,
        task_access: &mut SchedulerTaskAccess<'_>,
    ) -> EventResult {
        task_access
            .save_core_context(task_ref)
            .ok_or_else(|| self.failed_schedule_condition())??;
        self.switch_protocol_sequence = self.switch_protocol_sequence.wrapping_add(1);
        self.save_core_context_sequence = self.switch_protocol_sequence;

        task_access
            .suspend_after_core_context_save(task_ref)
            .ok_or_else(|| self.failed_schedule_condition())??;
        self.switch_protocol_sequence = self.switch_protocol_sequence.wrapping_add(1);
        self.suspend_task_sequence = self.switch_protocol_sequence;
        self.switch_prev_committed_online = true;
        Ok(())
    }

    /// Dispatch the preflighted fixed Task/Flow pair only after the selected
    /// task's stack has completed finish_task_switch.
    pub fn dispatch_task_after_switch(
        &mut self,
        task_ref: TaskRef,
        task_access: &mut SchedulerTaskAccess<'_>,
    ) -> EventResult {
        let prev_ref = self.switch_to_entry_prev_ref;
        if prev_ref == task_ref || self.switch_to_entry_next_ref != task_ref {
            return self.failed_switch_to();
        }

        let Some(expected_task_identity) = task_access.task_identity_ptr(task_ref) else {
            return self.failed_switch_to();
        };
        if crate::arch::riscv64::csr::read_tp() != expected_task_identity {
            return self.failed_switch_to();
        }
        // The architecture switch has committed tp/current-stack authority.
        // Publish the matching rq->curr again from the selected stack: the
        // The yielded Rust frame still holds the pre-switch mutable borrow, so
        // its earlier store is not the cross-stack observation boundary.
        if !self.publish_current(task_ref) {
            return self.failed_switch_to();
        }
        let user_address_space = task_access
            .commit_dispatch_address_space(task_ref)
            .map_err(|reason| self.failed_switch_commit(reason))?;
        if user_address_space
            && crate::objects::irq_time::begin_scheduler_slice(self.cpu_id()).is_none()
        {
            return Err(self.failed_switch_commit("user-scheduler-slice-begin"));
        }
        self.task_switch_tp_identity_passes = self.task_switch_tp_identity_passes.wrapping_add(1);
        self.physical_switch_pending = false;

        if !self.switch_prev_committed_online {
            self.suspend_task(prev_ref, task_access)?;
            self.switch_protocol_sequence = self.switch_protocol_sequence.wrapping_add(1);
            self.save_core_context_sequence = self.switch_protocol_sequence;
            self.switch_protocol_sequence = self.switch_protocol_sequence.wrapping_add(1);
            self.suspend_task_sequence = self.switch_protocol_sequence;
        }
        self.switch_prev_committed_online = false;

        // This code executes on the selected next Task's stack, after the
        // architecture switch replaced TP/current-stack authority. Only this
        // boundary can grant wait/reap permission for a terminal user Task.
        if prev_ref.is_user()
            && crate::objects::user_process_registry::global_registry().slot_state(prev_ref)
                == Some(crate::objects::user_process_registry::UserProcessSlotState::Zombie)
            && !crate::objects::user_process_registry::global_registry()
                .publish_scheduler_quiesced(prev_ref, self.cpu_ref())
        {
            return self.failed_switch_to();
        }

        self.switch_protocol_sequence = self.switch_protocol_sequence.wrapping_add(1);
        self.restore_core_context_sequence = self.switch_protocol_sequence;

        self.switch_to_exit_prev_ref = prev_ref;
        self.switch_to_exit_next_ref = task_ref;
        self.switch_to_exit_current_ref = task_ref;
        self.switch_to_exit_count = self.switch_to_exit_count.wrapping_add(1);
        self.scheduler_finish_task_switch_count =
            self.scheduler_finish_task_switch_count.wrapping_add(1);
        self.switch_protocol_sequence = self.switch_protocol_sequence.wrapping_add(1);
        self.finish_task_switch_sequence = self.switch_protocol_sequence;
        crate::checkpoint::checkpoint(Checkpoint::SchedulerSwitchToExit);

        let dispatch = self
            .next_dispatch
            .take()
            .filter(|dispatch| dispatch.task_ref().same_identity(task_ref))
            .ok_or_else(|| self.failed_schedule_condition())?;
        let enter_proof = task_access
            .accept_task_dispatch(dispatch)
            .ok_or_else(|| self.failed_schedule_condition())??;
        self.task_dispatch_signal_passes = self.task_dispatch_signal_passes.wrapping_add(1);
        self.switch_protocol_sequence = self.switch_protocol_sequence.wrapping_add(1);
        self.task_dispatch_sequence = self.switch_protocol_sequence;

        task_access
            .enter_task_flow(dispatch, enter_proof)
            .ok_or_else(|| self.failed_schedule_condition())??;
        self.flow_enter_signal_passes = self.flow_enter_signal_passes.wrapping_add(1);
        self.switch_protocol_sequence = self.switch_protocol_sequence.wrapping_add(1);
        self.flow_signal_sequence = self.switch_protocol_sequence;
        if task_ref.is_ap_idle() {
            crate::objects::kernel_task::record_idle_restore(self.cpu_id());
        }

        Ok(())
    }

    pub(crate) fn record_schedule_exit(&mut self, current_task: CurrentTask) -> EventResult {
        let task_ref = current_task.task_ref();
        let prev_ref = self.switch_to_entry_prev_ref;
        if prev_ref == task_ref
            || self.switch_to_entry_next_ref != task_ref
            || self.switch_to_exit_next_ref != task_ref
        {
            return self.failed_switch_to();
        }
        self.schedule_exit_current_ref = task_ref;
        self.schedule_exit_count = self.schedule_exit_count.wrapping_add(1);
        crate::checkpoint::checkpoint(Checkpoint::SchedulerScheduleExit);
        Ok(())
    }

    fn cooperative_context_switch(
        &mut self,
        preflight: SwitchPreflight,
    ) -> Result<bool, EventError> {
        let SwitchPreflight {
            prev_ref,
            next_ref,
            prev_context,
            next_context,
            prev_task_id,
            next_task_id,
            next_task_identity,
            trap_entry_context,
            first_boot_handoff,
        } = preflight;
        if first_boot_handoff {
            self.task_stack_switching_online = true;
            self.kernel_init_stack_switch_started_count =
                self.kernel_init_stack_switch_started_count.wrapping_add(1);
            crate::arch::riscv64::sbi::putstr("-> switch BootTask -> KernelInitTask\n");
        }
        if !self.cpu_ref().is_boot_cpu() {
            crate::objects::kernel_task::record_nonidentity_switch(self.cpu_id());
        }

        self.curr = next_ref;
        self.curr_task_id = next_task_id;
        let sentinel_mask = unsafe {
            task_switch::switch(
                &mut *prev_context,
                &*next_context,
                next_task_identity,
                trap_entry_context,
            )
        };
        self.curr = prev_ref;
        self.curr_task_id = prev_task_id;
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

    pub fn setup_smoke_scheduler_task(
        &mut self,
        entry: extern "C" fn() -> !,
        stacks: &mut SchedulerTestStacks,
        test_tasks: &mut SchedulerTestTasks,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || !self.runqueue_ready
            || !test_tasks
                .smoke_scheduler_task
                .setup(entry, self.cpu_id(), &mut stacks.scheduler)
        {
            return Err(self.failed_schedule_condition());
        }
        Ok(())
    }

    pub fn enqueue_smoke_scheduler_task(
        &mut self,
        test_tasks: &mut SchedulerTestTasks,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || test_tasks.smoke_scheduler_task.state() != State::Ready
            || test_tasks.smoke_scheduler_task.enqueued()
        {
            return Err(self.failed_schedule_condition());
        }

        self.enqueue_task_ref(self.cpu_ref(), TaskRef::SMOKE_SCHEDULER)?;
        test_tasks.smoke_scheduler_task.mark_enqueued()
    }

    pub fn setup_smoke_mutex_task(
        &mut self,
        entry: extern "C" fn() -> !,
        stacks: &mut SchedulerTestStacks,
        test_tasks: &mut SchedulerTestTasks,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || !self.runqueue_ready
            || !test_tasks
                .smoke_mutex_task
                .setup(entry, self.cpu_id(), &mut stacks.mutex)
        {
            return Err(self.failed_schedule_condition());
        }
        Ok(())
    }

    pub fn enqueue_smoke_mutex_task(&mut self, test_tasks: &mut SchedulerTestTasks) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || test_tasks.smoke_mutex_task.state() != State::Ready
            || test_tasks.smoke_mutex_task.enqueued()
        {
            return Err(self.failed_schedule_condition());
        }

        self.enqueue_task_ref(self.cpu_ref(), TaskRef::SMOKE_MUTEX)?;
        test_tasks.smoke_mutex_task.mark_enqueued()
    }

    pub fn setup_smoke_rwsem_task(
        &mut self,
        entry: extern "C" fn() -> !,
        stacks: &mut SchedulerTestStacks,
        test_tasks: &mut SchedulerTestTasks,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || !self.runqueue_ready
            || !test_tasks
                .smoke_rwsem_task
                .setup(entry, self.cpu_id(), &mut stacks.rwsem)
        {
            return Err(self.failed_schedule_condition());
        }
        Ok(())
    }

    pub fn enqueue_smoke_rwsem_task(&mut self, test_tasks: &mut SchedulerTestTasks) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || test_tasks.smoke_rwsem_task.state() != State::Ready
            || test_tasks.smoke_rwsem_task.enqueued()
        {
            return Err(self.failed_schedule_condition());
        }

        self.enqueue_task_ref(self.cpu_ref(), TaskRef::SMOKE_RWSEM)?;
        test_tasks.smoke_rwsem_task.mark_enqueued()
    }

    pub fn setup_smoke_rwlock_task(
        &mut self,
        entry: extern "C" fn() -> !,
        stacks: &mut SchedulerTestStacks,
        test_tasks: &mut SchedulerTestTasks,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || !self.runqueue_ready
            || !test_tasks
                .smoke_rwlock_task
                .setup(entry, self.cpu_id(), &mut stacks.rwlock)
        {
            return Err(self.failed_schedule_condition());
        }
        Ok(())
    }

    pub fn enqueue_smoke_rwlock_task(
        &mut self,
        test_tasks: &mut SchedulerTestTasks,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || test_tasks.smoke_rwlock_task.state() != State::Ready
            || test_tasks.smoke_rwlock_task.enqueued()
        {
            return Err(self.failed_schedule_condition());
        }

        self.enqueue_task_ref(self.cpu_ref(), TaskRef::SMOKE_RWLOCK)?;
        test_tasks.smoke_rwlock_task.mark_enqueued()
    }

    fn failed_switch_to(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Online,
            State::Online,
        )
    }

    fn failed_switch_preflight(&self, first_failed: &'static str) -> EventError {
        self.failed_schedule_condition()
            .with_diagnostic(FailureDiagnostic::new(
                "Scheduler",
                "SwitchTo.Preflight",
                "TaskPair",
                "all prerequisites before first mutation",
                first_failed,
            ))
    }

    fn trace_switch_stack_mismatch(&self) {
        crate::arch::riscv64::sbi::putstr("scheduler switch stack mismatch prev=");
        print_scheduler_hex(self.switch_stack_mismatch_prev_ref.slot());
        crate::arch::riscv64::sbi::putstr(":");
        print_scheduler_hex(self.switch_stack_mismatch_prev_ref.generation() as usize);
        crate::arch::riscv64::sbi::putstr(" live_sp=");
        print_scheduler_hex(self.switch_stack_mismatch_live_sp);
        crate::arch::riscv64::sbi::putstr(" task_stack=[");
        print_scheduler_hex(self.switch_stack_mismatch_task_base);
        crate::arch::riscv64::sbi::putstr(",");
        print_scheduler_hex(self.switch_stack_mismatch_task_top);
        crate::arch::riscv64::sbi::putstr(") entry_stack=[");
        print_scheduler_hex(self.switch_stack_mismatch_entry_base);
        crate::arch::riscv64::sbi::putstr(",");
        print_scheduler_hex(self.switch_stack_mismatch_entry_top);
        crate::arch::riscv64::sbi::putstr(") count=");
        print_scheduler_hex(self.switch_stack_mismatch_count);
        crate::arch::riscv64::sbi::putchar(b'\n');
    }

    fn failed_switch_commit(&self, first_failed: &'static str) -> EventError {
        self.failed_schedule_condition()
            .with_diagnostic(FailureDiagnostic::new(
                "Scheduler",
                "SwitchTo.Commit",
                "TaskPair",
                "selected Task and scheduler bindings",
                first_failed,
            ))
    }

    fn failed_schedule_entry(&self, first_failed: &'static str) -> EventError {
        self.failed_schedule_condition()
            .with_diagnostic(FailureDiagnostic::new(
                "Scheduler",
                "Schedule.Entry",
                "Sender",
                "active current TaskFlow on owning CPU",
                first_failed,
            ))
    }

    pub fn select_scheduler_for_task(&mut self, task_id: usize) -> Result<CpuRef, EventError> {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || !self.runqueue_ready
            || task_id == usize::MAX
        {
            return Err(self.failed_enable_condition());
        }

        let Some(scheduler_ref) = self.resolve_selected_scheduler_ref_for_cpu(self.cpu_id()) else {
            return Err(self.failed_enable_condition());
        };
        self.selected_runqueue_task_id = task_id;
        Ok(scheduler_ref)
    }

    pub fn enqueue_task_on_scheduler(
        &mut self,
        task_id: usize,
        task_ref: TaskRef,
        scheduler_ref: CpuRef,
    ) -> EventResult {
        if self.selected_runqueue_task_id != task_id || scheduler_ref != self.cpu_ref() {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.enqueue_task_with_id(scheduler_ref, task_ref, task_id)
    }

    // Used by the user-boot syscall continuation configuration.
    #[allow(dead_code)]
    pub fn dequeue_user_child_from_runqueue(&mut self, task_ref: TaskRef) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.scheduler_running
            || !self.runqueue_ready
        {
            return Err(self.failed_enable_condition());
        }

        let Some(scheduler_ref) = self.resolve_selected_scheduler_ref_for_cpu(self.cpu_id()) else {
            return Err(self.failed_enable_condition());
        };
        self.dequeue_task_ref(scheduler_ref, task_ref)
    }

    pub fn replace_user_task_on_runqueue(
        &mut self,
        previous: TaskRef,
        next: TaskRef,
        next_pid: usize,
    ) -> EventResult {
        let Some(scheduler_ref) = self.resolve_selected_scheduler_ref_for_cpu(self.cpu_id()) else {
            return Err(self.failed_enable_condition());
        };
        self.replace_user_task_ref(scheduler_ref, previous, next, next_pid)
    }

    pub fn can_replace_user_task_on_runqueue(&self, previous: TaskRef) -> bool {
        self.resolve_selected_scheduler_ref_for_cpu(self.cpu_id())
            .is_some()
            && previous.is_user()
            && self.contains_task_ref(previous)
    }

    fn resolve_selected_scheduler_ref_for_cpu(&self, cpu_id: usize) -> Option<CpuRef> {
        if self.runqueue_ready && self.cpu_id() == cpu_id && self.attached_to_root_domain() {
            Some(self.cpu_ref())
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

    fn failed_setup_error(&self) -> EventError {
        EventError::failed(
            EventErrorCode::ConditionFailed,
            LifecycleEvent::Setup,
            self.lifecycle.state(),
            State::Prepared,
            State::Ready,
        )
    }

    fn setup_facts_hold(
        &self,
        inventory: PossibleCpuInventory,
        root_domain: &DefaultSchedRootDomain,
    ) -> bool {
        self.runqueue_ready
            && self.lock().state() == State::Ready
            && !self.lock().locked()
            && self.lock().acquired_count() != 0
            && self.lock().released_count() != 0
            && self.root_attach_held_runqueue_lock()
            && self.lock().irqsave_entered_count() != 0
            && self.lock().irqrestore_exited_count() != 0
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
            && self.cpu_ref().is_boot_cpu()
            && inventory.cpu_ref(0) == Some(self.cpu_ref())
            && root_domain.covered_cpu_count() == inventory.count()
            && root_domain.covers_cpu_ref(self.cpu_ref())
            && self.curr_task_id() == self.boot_idle_setup_state.task_id()
            && self.idle_task_id() == self.boot_idle_setup_state.task_id()
    }
}

fn trace_switch_to(
    logical_id: usize,
    prev_ref: TaskRef,
    prev_task_id: usize,
    next_ref: TaskRef,
    next_task_id: usize,
    current_after: TaskRef,
) {
    #[cfg(checkpoint_handler_announce)]
    crate::arch::riscv64::sbi::write_record(format_args!(
        "checkpoint: Scheduler.SwitchTo prev={} next={} current_after={}\n",
        prev_ref.name(),
        next_ref.name(),
        current_after.name()
    ));

    #[cfg(not(checkpoint_handler_announce))]
    {
        let _ = current_after;
    }

    #[cfg(checkpoint_handler_user_scheduler_trace)]
    crate::arch::riscv64::sbi::write_record(format_args!(
        "user scheduler switch cpu={} prev_slot={} prev_generation={} prev_pid={} next_slot={} next_generation={} next_pid={}\n",
        logical_id,
        prev_ref.slot(),
        prev_ref.generation(),
        prev_task_id,
        next_ref.slot(),
        next_ref.generation(),
        next_task_id,
    ));

    #[cfg(not(checkpoint_handler_user_scheduler_trace))]
    {
        let _ = (logical_id, prev_ref, prev_task_id, next_ref, next_task_id);
    }
}

fn print_scheduler_hex(value: usize) {
    crate::arch::riscv64::sbi::putstr("0x");
    let mut shift = usize::BITS as usize;
    while shift != 0 {
        shift -= 4;
        let digit = ((value >> shift) & 0xf) as u8;
        crate::arch::riscv64::sbi::putchar(if digit < 10 {
            b'0' + digit
        } else {
            b'a' + digit - 10
        });
    }
}

fn schedule_stage(error: EventError, first_failed: &'static str) -> EventError {
    error.with_diagnostic_if_absent(FailureDiagnostic::new(
        "Scheduler",
        "Schedule.Runtime",
        "CpuRunqueue",
        "entry, runqueue protocol, context switch, and guard restoration",
        first_failed,
    ))
}

#[derive(Clone, Copy)]
pub struct CpuRunQueueView {
    state: State,
    cpu_ref: CpuRef,
    cpu_hartid: usize,
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
    runqueue_current_task_ref: TaskRef,
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

    pub const fn runqueue_current_task_ref(self) -> TaskRef {
        self.runqueue_current_task_ref
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
    if task_ref.is_kernel() {
        return crate::objects::kernel_task::task_by_ref(task_ref).map(Task::pid);
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

const fn default_sched_class(task_ref: TaskRef) -> SchedClassRef {
    if task_ref.is_ap_idle() || task_ref.same_identity(TaskRef::BOOT) {
        SchedClassRef::Idle
    } else {
        SchedClassRef::Fair
    }
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub struct SmokeSchedulerTask {
    task: Task,
    enqueued: bool,
    entry_ran: bool,
    yielded_back: bool,
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl SmokeSchedulerTask {
    const fn new(task_id: usize) -> Self {
        Self {
            task: Task::with_ref(smoke_task_ref(task_id)),
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
        self.task.embedded_flow().cpu_id()
    }

    pub const fn cpu_ref(&self) -> Option<CpuRef> {
        self.task.flow_cpu_ref()
    }

    pub const fn enqueued(&self) -> bool {
        self.enqueued
    }

    pub const fn scheduler_sleep_declared(&self) -> bool {
        self.task.scheduler_sleep_declared()
    }

    pub const fn pending_wake_signal(&self) -> bool {
        self.task.pending_wake_signal()
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

    pub(crate) fn task_ptr(&self) -> usize {
        &self.task as *const Task as usize
    }

    pub(crate) fn task_mut(&mut self) -> &mut Task {
        &mut self.task
    }

    fn current_task_candidate(&self) -> super::current_task::CurrentTaskCandidate<'_> {
        super::current_task::CurrentTaskCandidate {
            task: &self.task,
            flow: self.task.embedded_flow(),
        }
    }

    pub const fn flow_ref(&self) -> TaskFlowRef {
        self.task.flow_ref()
    }

    pub fn unified_carrier_ready(&self) -> bool {
        self.task
            .embedded_flow()
            .owner()
            .same_identity(self.task.task_ref())
            && self
                .task
                .flow()
                .same_identity(self.task.embedded_flow().flow_ref())
            && ((self.task.state() == State::Ready && self.task.flow_state() == State::Online)
                || (self.task.state() == State::Online && self.task.flow_state() == State::Online)
                || (self.task.state() == State::OnCpu
                    && self.task.flow_state() == State::Online
                    && self
                        .task
                        .flow()
                        .same_identity(self.task.embedded_flow().flow_ref())))
    }

    fn setup(
        &mut self,
        entry: extern "C" fn() -> !,
        cpu_id: usize,
        stack: &mut [usize; SMOKE_SCHEDULER_STACK_WORDS],
    ) -> bool {
        if self.task.state() != State::Base || cpu_id == usize::MAX {
            return false;
        }

        let stack_top = stack.as_ptr() as usize + core::mem::size_of_val(stack);
        if self
            .task
            .set_identity_metadata(
                self.task_id_from_ref(),
                TaskEntry::SmokeScheduler,
                TaskKind::TestOnly,
            )
            .is_err()
            || self.task.adopt_preset().is_err()
            || self.task.declare_and_bind_embedded_flow().is_err()
        {
            return false;
        }
        self.task
            .init_switch_context(entry, stack.as_ptr() as usize, stack_top);
        if self.task.adopt_setup().is_err() {
            return false;
        }
        if !self.task.bind_flow_cpu_ref(CpuRef::new(cpu_id)) {
            return false;
        }
        if self.task.publish_embedded_flow().is_err() {
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

    pub(crate) fn deactivate_from_scheduler(&mut self) -> EventResult {
        self.task.deactivate_from_scheduler()?;
        self.mark_dequeued();
        Ok(())
    }

    pub fn mark_entry_ran(&mut self) -> EventResult {
        if self.task.state() != State::OnCpu
            || self.task.flow_state() != State::Online
            || !self.enqueued
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

    pub(crate) fn switch_context_mut(&mut self) -> &mut TaskSwitchContext {
        self.task.switch_context_mut()
    }

    pub(crate) fn suspend_from_cpu(&mut self) -> EventResult {
        self.task.suspend_from_cpu()
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

pub struct BitWaitQueueTable {
    lifecycle: Lifecycle,
    bucket_count: usize,
    bucket_waitqueues_ready: bool,
    bucket_locks_ready: bool,
    bucket_lists_empty: bool,
}

impl BitWaitQueueTable {
    pub(crate) const fn new() -> Self {
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

    pub(crate) fn preset(&mut self) -> EventResult {
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

#[derive(Clone, Copy)]
struct ClassPickOutcome {
    next_ref: TaskRef,
    protocol: PickNextProtocol,
    put_prev_done: bool,
    set_next_done: bool,
}

impl Scheduler {
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

    pub const fn curr_ref(&self) -> TaskRef {
        self.curr
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub const fn idle_ref(&self) -> TaskRef {
        self.idle
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub const fn stop_ref(&self) -> TaskRef {
        self.stop
    }

    pub const fn idle_task_id(&self) -> usize {
        self.idle_task_id
    }

    const fn idle_binding_valid(&self) -> bool {
        if self.cpu_ref.is_boot_cpu() {
            self.idle.same_identity(TaskRef::BOOT)
                && self.idle_task_id == self.boot_idle_setup_state.task_id()
        } else {
            self.idle.same_identity(TaskRef::ap_idle(self.cpu_id())) && self.idle_task_id == 0
        }
    }

    pub const fn class_queues_ready(&self) -> bool {
        self.class_queues_ready
    }

    pub const fn attached_to_root_domain(&self) -> bool {
        self.attached_to_root_domain
    }

    pub const fn root_attach_held_runqueue_lock(&self) -> bool {
        self.root_attach_held_runqueue_lock
    }

    pub const fn balance_push_enabled(&self) -> bool {
        self.balance_push_enabled
    }

    fn runqueue_view(&self, boot_backed: bool) -> Option<CpuRunQueueView> {
        if !self.runqueue_ready {
            return None;
        }
        Some(CpuRunQueueView {
            state: State::Ready,
            cpu_ref: self.cpu_ref,
            cpu_hartid: self.cpu_hartid,
            boot_backed,
        })
    }

    pub const fn contains_task(&self, task_id: usize) -> bool {
        self.stop_queue.contains_id(task_id)
            || self.deadline_queue.contains_id(task_id)
            || self.realtime_queue.contains_id(task_id)
            || self.fair_queue.contains_id(task_id)
    }

    pub const fn task_count(&self) -> usize {
        self.stop_queue.len()
            + self.deadline_queue.len()
            + self.realtime_queue.len()
            + self.fair_queue.len()
    }

    /// Return whether a scheduling decision can select a non-idle Task other
    /// than `current`. The owner CPU calls this only at an IRQ-masked return
    /// boundary after committing all acquire-visible inbox records.
    pub const fn has_runnable_competitor(&self, current: TaskRef) -> bool {
        let next = self.pick_highest_class_task(current);
        next.is_valid() && !next.same_identity(current) && !next.same_identity(self.idle_ref())
    }

    pub const fn contains_task_ref(&self, task_ref: TaskRef) -> bool {
        self.stop_queue.contains_ref(task_ref)
            || self.deadline_queue.contains_ref(task_ref)
            || self.realtime_queue.contains_ref(task_ref)
            || self.fair_queue.contains_ref(task_ref)
            || self.idle_queue.contains_ref(task_ref)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub const fn first_runnable_task_ref(&self) -> TaskRef {
        let stop = self.stop_queue.pick_after(TaskRef::NONE);
        if stop.is_valid() {
            stop
        } else {
            let deadline = self.deadline_queue.pick_after(TaskRef::NONE);
            if deadline.is_valid() {
                deadline
            } else {
                let realtime = self.realtime_queue.pick_after(TaskRef::NONE);
                if realtime.is_valid() {
                    realtime
                } else {
                    self.pick_fair_task(TaskRef::BOOT)
                }
            }
        }
    }

    pub const fn task_class(&self, task_ref: TaskRef) -> Option<SchedClassRef> {
        if self.stop_queue.contains_ref(task_ref) {
            Some(SchedClassRef::Stop)
        } else if self.deadline_queue.contains_ref(task_ref) {
            Some(SchedClassRef::Deadline)
        } else if self.realtime_queue.contains_ref(task_ref) {
            Some(SchedClassRef::Realtime)
        } else if self.fair_queue.contains_ref(task_ref) {
            Some(SchedClassRef::Fair)
        } else if self.idle_queue.contains_ref(task_ref) {
            Some(SchedClassRef::Idle)
        } else {
            None
        }
    }

    const fn task_id_for_ref(&self, task_ref: TaskRef) -> Option<usize> {
        if task_ref.same_identity(self.idle) {
            return Some(self.idle_task_id);
        }
        if let Some(task_id) = self.stop_queue.id_for_ref(task_ref) {
            return Some(task_id);
        }
        if let Some(task_id) = self.deadline_queue.id_for_ref(task_ref) {
            return Some(task_id);
        }
        if let Some(task_id) = self.realtime_queue.id_for_ref(task_ref) {
            return Some(task_id);
        }
        self.fair_queue.id_for_ref(task_ref)
    }

    fn publish_current(&mut self, task_ref: TaskRef) -> bool {
        let Some(task_id) = self.task_id_for_ref(task_ref) else {
            return false;
        };
        self.curr = task_ref;
        self.curr_task_id = task_id;
        true
    }

    fn class_queue_mut(&mut self, class: SchedClassRef) -> &mut SchedClassQueue {
        match class {
            SchedClassRef::Stop => &mut self.stop_queue,
            SchedClassRef::Deadline => &mut self.deadline_queue,
            SchedClassRef::Realtime => &mut self.realtime_queue,
            SchedClassRef::Fair => &mut self.fair_queue,
            SchedClassRef::Idle => &mut self.idle_queue,
        }
    }

    const fn pick_highest_class_task(&self, prev_ref: TaskRef) -> TaskRef {
        let stop = self.stop_queue.pick_after(prev_ref);
        if stop.is_valid() {
            return stop;
        }
        let deadline = self.deadline_queue.pick_after(prev_ref);
        if deadline.is_valid() {
            return deadline;
        }
        let realtime = self.realtime_queue.pick_after(prev_ref);
        if realtime.is_valid() {
            return realtime;
        }
        let fair = self.pick_fair_task(prev_ref);
        if fair.is_valid() {
            return fair;
        }
        self.idle_queue.pick_after(prev_ref)
    }

    const fn pick_fair_task(&self, prev_ref: TaskRef) -> TaskRef {
        self.fair_queue.pick_after(prev_ref)
    }

    // Initializes the rq fields owned directly by this Scheduler.
    #[allow(clippy::too_many_arguments)]
    fn setup_runqueue(
        &mut self,
        boot_cpu_ref: CpuRef,
        boot_cpu_hartid: usize,
        _per_cpu_storage: &PerCpuStorage,
        root_domain: &DefaultSchedRootDomain,
        boot_task: &BootTask,
        local_interrupt: &mut InterruptType,
    ) -> EventResult {
        if self.runqueue_ready
            || self.lock.state() != State::Base
            || root_domain.state() != State::Ready
            || boot_task.state() != State::OnCpu
            || local_interrupt.local_state() != State::Ready
            || self.boot_init_preemption.state() != State::Base
        {
            return self.failed_setup();
        }

        if !boot_cpu_ref.is_boot_cpu() || !root_domain.covers_cpu_ref(boot_cpu_ref) {
            return self.failed_setup();
        }

        self.lock
            .setup_with_checkpoint(Checkpoint::BootRunQueueLockReady)?;
        self.boot_init_preemption
            .setup_disabled_with_checkpoint(boot_task, Checkpoint::BootInitPreemptionReady)?;
        self.cpu_ref = boot_cpu_ref;
        self.cpu_hartid = boot_cpu_hartid;
        self.curr = TaskRef::BOOT;
        self.idle = TaskRef::BOOT;
        self.curr_task_id = 0;
        self.idle_task_id = 0;
        self.class_queues_ready = true;
        if !self.idle_queue.enqueue(TaskRef::BOOT, 0) {
            return self.failed_setup();
        }
        self.lock
            .lock_irqsave(local_interrupt, &mut self.boot_init_preemption)?;
        let attach_result: EventResult = {
            self.attached_to_root_domain = true;
            self.root_attach_held_runqueue_lock = true;
            Ok(())
        };
        let unlock_result = self
            .lock
            .unlock_irqrestore(local_interrupt, &mut self.boot_init_preemption);
        attach_result.and(unlock_result)?;
        self.attached_to_root_domain = true;
        self.balance_push_enabled = false;
        self.runqueue_ready = true;
        crate::checkpoint::checkpoint(Checkpoint::BootRunQueueReady);
        Ok(())
    }

    fn setup_secondary_runqueue(
        &mut self,
        cpu_ref: CpuRef,
        cpu_hartid: usize,
        root_domain: &DefaultSchedRootDomain,
    ) -> EventResult {
        if self.runqueue_ready
            || self.lock.state() != State::Base
            || cpu_ref.is_boot_cpu()
            || !root_domain.covers_cpu_ref(cpu_ref)
        {
            return self.failed_setup();
        }
        self.lock
            .setup_with_checkpoint(Checkpoint::BootRunQueueLockReady)?;
        self.cpu_ref = cpu_ref;
        self.cpu_hartid = cpu_hartid;
        let idle_ref = TaskRef::ap_idle(cpu_ref.logical_id());
        self.curr = idle_ref;
        self.idle = idle_ref;
        self.curr_task_id = 0;
        self.idle_task_id = 0;
        self.class_queues_ready = true;
        if !self.idle_queue.enqueue(idle_ref, 0) {
            return self.failed_setup();
        }
        self.attached_to_root_domain = true;
        self.root_attach_held_runqueue_lock = true;
        self.balance_push_enabled = false;
        self.runqueue_ready = true;
        crate::checkpoint::checkpoint(Checkpoint::BootRunQueueReady);
        Ok(())
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn setup_runqueue_for_local_subject(
        &mut self,
        cpu_group: &CpuGroup,
        per_cpu_storage: &PerCpuStorage,
        root_domain: &DefaultSchedRootDomain,
    ) -> EventResult {
        if self.runqueue_ready
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
        self.curr = TaskRef::BOOT;
        self.idle = TaskRef::BOOT;
        self.curr_task_id = 0;
        self.idle_task_id = 0;
        self.class_queues_ready = true;
        if !self.idle_queue.enqueue(TaskRef::BOOT, 0) {
            return self.failed_setup();
        }
        self.attached_to_root_domain = true;
        self.root_attach_held_runqueue_lock = true;
        self.balance_push_enabled = false;
        self.runqueue_ready = true;
        crate::checkpoint::checkpoint(Checkpoint::BootRunQueueReady);
        Ok(())
    }

    pub fn enqueue_task_ref(&mut self, scheduler_ref: CpuRef, task_ref: TaskRef) -> EventResult {
        if scheduler_ref != self.cpu_ref() {
            return self.failed_setup();
        }

        let task_id = if task_ref.is_user() {
            USER_CHILD_PID
        } else {
            task_id_for_current_task_ref(task_ref).ok_or_else(|| self.failed_setup_error())?
        };
        self.enqueue_task_with_class_and_id(task_ref, default_sched_class(task_ref), task_id)
    }

    pub fn enqueue_task_with_id(
        &mut self,
        scheduler_ref: CpuRef,
        task_ref: TaskRef,
        task_id: usize,
    ) -> EventResult {
        if scheduler_ref != self.cpu_ref()
            || !task_ref.is_scheduler_ref()
            || task_ref.same_identity(TaskRef::BOOT)
            || task_id == usize::MAX
        {
            return self.failed_setup();
        }
        if (task_ref.is_user() && task_id < USER_CHILD_PID)
            || (!task_ref.is_user() && task_id_for_current_task_ref(task_ref) != Some(task_id))
        {
            return self.failed_setup();
        }
        self.enqueue_task_with_class_and_id(task_ref, default_sched_class(task_ref), task_id)
    }

    pub fn commit_inbound_task(
        &mut self,
        task_ref: TaskRef,
        kind: super::kernel_task::InboundKind,
        task_id: usize,
        task_access: &mut SchedulerTaskAccess<'_>,
        local_interrupt: &mut InterruptType,
    ) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !task_ref.is_scheduler_ref()
            || task_ref.same_identity(TaskRef::BOOT)
            || (!task_ref.is_user() && task_id_for_current_task_ref(task_ref) != Some(task_id))
            || (task_ref.is_user()
                && super::user_process_registry::global_registry().pid(task_ref) != Some(task_id))
            || self.contains_task(task_id)
            || self.contains_task_ref(task_ref)
            || self.fair_queue.len() == SCHED_CLASS_QUEUE_CAPACITY
        {
            return self.failed_setup();
        }
        let task = task_access
            .current_task_candidate(task_ref)
            .map(|candidate| candidate.task);
        let task_preflight = task.is_some_and(|task| {
            task.flow_cpu_ref() == Some(self.cpu_ref())
                && match kind {
                    super::kernel_task::InboundKind::Activate => {
                        (task_ref.is_kernel() && task.state() == State::Ready)
                            || (task_ref.is_user() && task.state() == State::Online)
                    }
                    super::kernel_task::InboundKind::Wake => {
                        task.state() == State::Online
                            && task.scheduler_sleep_declared()
                            && !task.runqueue_published()
                    }
                }
        });
        if !task_preflight {
            return self.failed_setup();
        }

        self.boot_idle_preemption.disable()?;
        local_interrupt.save_and_disable()?;
        self.lock
            .lock_irqsave(local_interrupt, &mut self.boot_idle_preemption)?;
        let commit = (|| {
            match kind {
                super::kernel_task::InboundKind::Activate => {
                    if task_ref.is_kernel() {
                        super::kernel_task::activate_for_enqueue(task_ref, self.cpu_id())?;
                    }
                }
                super::kernel_task::InboundKind::Wake => {
                    task_access
                        .wake_for_inbound(task_ref)
                        .ok_or_else(|| self.failed_setup_error())??;
                }
            }
            self.enqueue_task_with_class_and_id(task_ref, SchedClassRef::Fair, task_id)
        })();
        let unlock = self
            .lock
            .unlock_irqrestore(local_interrupt, &mut self.boot_idle_preemption);
        let restore = local_interrupt.restore();
        let enable = self.boot_idle_preemption.enable_no_resched();
        commit.and(unlock).and(restore).and(enable)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn enqueue_task_in_class(
        &mut self,
        scheduler_ref: CpuRef,
        task_ref: TaskRef,
        class: SchedClassRef,
    ) -> EventResult {
        if scheduler_ref != self.cpu_ref() || task_ref.is_user() {
            return self.failed_setup();
        }
        let task_id =
            task_id_for_current_task_ref(task_ref).ok_or_else(|| self.failed_setup_error())?;
        self.enqueue_task_with_class_and_id(task_ref, class, task_id)
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn pick_next_task_for_local_subject(
        &self,
        scheduler_ref: CpuRef,
        prev_ref: TaskRef,
        disposition: PrevDisposition,
    ) -> Result<TaskRef, EventError> {
        let next_ref = self.pick_task(scheduler_ref, prev_ref, disposition)?;
        let protocol = if self.task_class(prev_ref) == Some(SchedClassRef::Fair)
            && self.task_class(next_ref) == Some(SchedClassRef::Fair)
        {
            PickNextProtocol::Combined
        } else {
            PickNextProtocol::Fallback
        };
        Ok(self
            .pick_next_task_with_protocol(scheduler_ref, prev_ref, disposition, protocol)?
            .next_ref)
    }

    fn pick_next_task_with_protocol(
        &self,
        scheduler_ref: CpuRef,
        prev_ref: TaskRef,
        disposition: PrevDisposition,
        protocol: PickNextProtocol,
    ) -> Result<ClassPickOutcome, EventError> {
        let next_ref = self.pick_task(scheduler_ref, prev_ref, disposition)?;
        match protocol {
            PickNextProtocol::Combined => Ok(ClassPickOutcome {
                next_ref,
                protocol,
                put_prev_done: false,
                set_next_done: false,
            }),
            PickNextProtocol::Fallback => {
                self.put_prev_task(prev_ref, next_ref, disposition)?;
                self.set_next_task(next_ref)?;
                Ok(ClassPickOutcome {
                    next_ref,
                    protocol,
                    put_prev_done: true,
                    set_next_done: true,
                })
            }
        }
    }

    #[cfg_attr(not(app_smoke), allow(dead_code))]
    pub fn pick_next_task_for_protocol(
        &self,
        scheduler_ref: CpuRef,
        prev_ref: TaskRef,
        disposition: PrevDisposition,
        protocol: PickNextProtocol,
    ) -> Result<TaskRef, EventError> {
        Ok(self
            .pick_next_task_with_protocol(scheduler_ref, prev_ref, disposition, protocol)?
            .next_ref)
    }

    fn pick_task(
        &self,
        scheduler_ref: CpuRef,
        prev_ref: TaskRef,
        disposition: PrevDisposition,
    ) -> Result<TaskRef, EventError> {
        if scheduler_ref != self.cpu_ref() || !self.runqueue_ready || !prev_ref.is_scheduler_ref() {
            return Err(self.failed_setup_error());
        }

        if disposition == PrevDisposition::Blocked && self.contains_task_ref(prev_ref) {
            return Err(self.failed_setup_error());
        }

        let next_ref = self.pick_highest_class_task(prev_ref);
        if matches!(next_ref, TaskRef::NONE) {
            return Err(self.failed_setup_error());
        }
        Ok(next_ref)
    }

    fn put_prev_task(
        &self,
        prev_ref: TaskRef,
        _next_ref: TaskRef,
        disposition: PrevDisposition,
    ) -> EventResult {
        if disposition == PrevDisposition::Blocked {
            if self.contains_task_ref(prev_ref) {
                return self.failed_setup();
            }
            return Ok(());
        }
        if !self.contains_task_ref(prev_ref) {
            return self.failed_setup();
        }
        Ok(())
    }

    fn set_next_task(&self, next_ref: TaskRef) -> EventResult {
        if self.task_class(next_ref).is_none() {
            return self.failed_setup();
        }
        Ok(())
    }

    fn enqueue_task_with_class_and_id(
        &mut self,
        task_ref: TaskRef,
        class: SchedClassRef,
        task_id: usize,
    ) -> EventResult {
        if !self.runqueue_ready
            || task_id == usize::MAX
            || self.contains_task(task_id)
            || self.contains_task_ref(task_ref)
            || class == SchedClassRef::Idle
        {
            return self.failed_setup();
        }

        if !self.class_queue_mut(class).enqueue(task_ref, task_id) {
            return self.failed_setup();
        }
        if class == SchedClassRef::Stop {
            self.stop = task_ref;
        }
        self.enqueued_task_id = task_id;
        Ok(())
    }

    pub fn dequeue_task_ref(&mut self, scheduler_ref: CpuRef, task_ref: TaskRef) -> EventResult {
        if scheduler_ref != self.cpu_ref() {
            return self.failed_setup();
        }

        let Some(class) = self.task_class(task_ref) else {
            return self.failed_setup();
        };
        if class == SchedClassRef::Idle || !self.class_queue_mut(class).dequeue_ref(task_ref) {
            return self.failed_setup();
        }
        if class == SchedClassRef::Stop && self.stop.same_identity(task_ref) {
            self.stop = TaskRef::NONE;
        }
        self.enqueued_task_id = usize::MAX;
        Ok(())
    }

    pub fn replace_user_task_ref(
        &mut self,
        scheduler_ref: CpuRef,
        previous: TaskRef,
        next: TaskRef,
        next_pid: usize,
    ) -> EventResult {
        if scheduler_ref != self.cpu_ref()
            || !previous.is_user()
            || !next.is_user()
            || next_pid < USER_CHILD_PID
            || self.task_class(previous) != Some(SchedClassRef::Fair)
        {
            return self.failed_setup();
        }
        if !self.fair_queue.replace_ref(previous, next, next_pid) {
            return self.failed_setup();
        }
        self.enqueued_task_id = next_pid;
        Ok(())
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
        runqueue_ready: bool,
        runqueue_cpu_ref: CpuRef,
        runqueue_cpu_hartid: usize,
        runqueue_idle_task_id: usize,
    ) -> bool {
        let Some(boot_cpu) = cpu_group.boot_cpu() else {
            return false;
        };
        self.lifecycle.state() == State::Ready
            && runqueue_ready
            && runqueue_cpu_ref == boot_cpu.cpu_ref()
            && runqueue_cpu_hartid == boot_cpu.hartid()
            && runqueue_cpu_ref.is_boot_cpu()
            && self.cpu_ref == boot_cpu.cpu_ref()
            && self.cpu_ref == runqueue_cpu_ref
            && self.cpu_id() == runqueue_cpu_ref.logical_id()
            && self.task_id() == runqueue_idle_task_id
    }

    // Boot idle setup binds the specified task, runqueue, RCU and CPU-local guards.
    #[allow(clippy::too_many_arguments)]
    fn setup(
        &mut self,
        boot_task: &mut BootTask,
        init_mm: &InitMm,
        runqueue_lock: &mut RawSpinLock,
        runqueue_ready: bool,
        runqueue_cpu_ref: CpuRef,
        boot_idle_rcu_read_side: &mut RcuReadSide,
        boot_cpu_ref: CpuRef,
        local_interrupt: &mut InterruptType,
        boot_idle_preemption: &mut PreemptionControl,
    ) -> EventResult {
        if self.lifecycle.state() != State::Base
            || self.pi_lock.state() != State::Base
            || boot_task.state() != State::OnCpu
            || init_mm.state() != State::Ready
            || !runqueue_ready
            || runqueue_lock.state() != State::Ready
            || boot_idle_rcu_read_side.state() != State::Prepared
            || !boot_idle_rcu_read_side.incomplete_first_slice()
            || !boot_idle_rcu_read_side.full_semantics_deferred()
            || local_interrupt.local_state() != State::Ready
            || boot_idle_preemption.state() != State::Base
            || runqueue_cpu_ref != boot_cpu_ref
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
            runqueue_lock.acquire()?;
            let runqueue_guarded_result = (|| {
                self.init_held_runqueue_lock = true;
                boot_idle_rcu_read_side.read_lock()?;
                if boot_task
                    .bind_idle_metadata(runqueue_cpu_ref.logical_id())
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
                self.cpu_ref = runqueue_cpu_ref;
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
            let release_result = runqueue_lock.release();
            runqueue_guarded_result.and(release_result)
        })();
        let unlock_result = self
            .pi_lock
            .unlock_irqrestore(local_interrupt, boot_idle_preemption);
        guarded_result.and(unlock_result)
    }
}
