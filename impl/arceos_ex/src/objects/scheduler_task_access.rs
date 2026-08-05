use super::trap_flow_type::{TrapFlowRef as RootTrapFlowRef, TrapFlowType};
use super::{
    boot_task::BootTask,
    cpu::CpuRef,
    rest_init::{KernelInitTask, KthreaddTask},
    scheduler::SchedulerTestTasks,
    state::{EventResult, State},
    task::{Task, TaskExecutionAuthority, TaskFlowEnterProof, TaskRef},
    task_flow::TaskFlowRef,
    user_boot::UserTaskSet,
};
use crate::arch::riscv64::task_switch::TaskSwitchContext;

/// Stable, generation-carrying result of the next-task preflight. Scheduler
/// stores this value across the physical stack switch and must dispatch the
/// exact Task/Flow pair recorded here.
#[derive(Clone, Copy, Eq, PartialEq)]
enum DispatchStackBinding {
    TaskOwned,
    SimulatedUserCarrier,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct NextDispatch {
    task_ref: TaskRef,
    flow_ref: TaskFlowRef,
    cpu_ref: CpuRef,
    context_epoch: u64,
    root_trap_flow_ref: RootTrapFlowRef,
    stack_base: usize,
    stack_top: usize,
    stack_binding: DispatchStackBinding,
}

impl NextDispatch {
    const fn new(
        task_ref: TaskRef,
        flow_ref: TaskFlowRef,
        cpu_ref: CpuRef,
        context_epoch: u64,
        root_trap_flow_ref: RootTrapFlowRef,
        stack_bounds: (usize, usize),
        stack_binding: DispatchStackBinding,
    ) -> Self {
        Self {
            task_ref,
            flow_ref,
            cpu_ref,
            context_epoch,
            root_trap_flow_ref,
            stack_base: stack_bounds.0,
            stack_top: stack_bounds.1,
            stack_binding,
        }
    }

    pub const fn task_ref(self) -> TaskRef {
        self.task_ref
    }

    pub const fn flow_ref(self) -> TaskFlowRef {
        self.flow_ref
    }

    pub const fn cpu_ref(self) -> CpuRef {
        self.cpu_ref
    }

    pub const fn context_epoch(self) -> u64 {
        self.context_epoch
    }

    pub const fn root_trap_flow_ref(self) -> RootTrapFlowRef {
        self.root_trap_flow_ref
    }

    fn with_simulated_user_carrier(mut self, stack_base: usize, stack_top: usize) -> Self {
        self.stack_base = stack_base;
        self.stack_top = stack_top;
        self.stack_binding = DispatchStackBinding::SimulatedUserCarrier;
        self
    }
}

/// Resolves stable scheduler references into task/flow carriers owned by the
/// surrounding Context. Scheduler itself never owns or names those carriers.
pub struct SchedulerTaskAccess<'a> {
    kernel_init_task: Option<&'a mut KernelInitTask>,
    kthreadd_task: Option<&'a mut KthreaddTask>,
    user_task_set: Option<&'a mut UserTaskSet>,
    test_tasks: Option<&'a mut SchedulerTestTasks>,
    owner_cpu: usize,
}

impl<'a> SchedulerTaskAccess<'a> {
    pub fn new(
        kernel_init_task: &'a mut KernelInitTask,
        kthreadd_task: &'a mut KthreaddTask,
        user_task_set: &'a mut UserTaskSet,
        test_tasks: &'a mut SchedulerTestTasks,
    ) -> Self {
        Self {
            kernel_init_task: Some(kernel_init_task),
            kthreadd_task: Some(kthreadd_task),
            user_task_set: Some(user_task_set),
            test_tasks: Some(test_tasks),
            owner_cpu: 0,
        }
    }

    pub fn new_secondary(owner_cpu: usize) -> Self {
        Self {
            kernel_init_task: None,
            kthreadd_task: None,
            user_task_set: None,
            test_tasks: None,
            owner_cpu,
        }
    }

    fn user_candidate(
        &self,
        task_ref: TaskRef,
    ) -> Option<super::current_task::CurrentTaskCandidate<'_>> {
        if let Some(user_task_set) = self.user_task_set.as_deref() {
            user_task_set
                .current_task_candidate_by_ref(task_ref)
                .or_else(|| super::user_boot::smp_task_candidate_by_ref(task_ref, self.owner_cpu))
        } else {
            super::user_boot::smp_task_candidate_by_ref(task_ref, self.owner_cpu)
        }
    }

    fn user_task_mut(&mut self, task_ref: TaskRef) -> Option<&mut Task> {
        if let Some(user_task_set) = self.user_task_set.as_deref_mut() {
            user_task_set
                .task_mut_by_ref(task_ref)
                .or_else(|| super::user_boot::smp_task_mut_by_ref_on_cpu(task_ref, self.owner_cpu))
        } else {
            super::user_boot::smp_task_mut_by_ref_on_cpu(task_ref, self.owner_cpu)
        }
    }

    pub fn effective_cpu_ref(&self, task_ref: TaskRef) -> Option<CpuRef> {
        match task_ref {
            TaskRef::BOOT => BootTask::canonical_task().flow_cpu_ref(),
            TaskRef::KERNEL_INIT => self.kernel_init_task.as_deref()?.task().flow_cpu_ref(),
            TaskRef::KTHREADD => self.kthreadd_task.as_deref()?.task().flow_cpu_ref(),
            TaskRef::SMOKE_SCHEDULER => {
                self.test_tasks.as_deref()?.smoke_scheduler_task().cpu_ref()
            }
            TaskRef::SMOKE_MUTEX => self.test_tasks.as_deref()?.smoke_mutex_task().cpu_ref(),
            TaskRef::SMOKE_RWSEM => self.test_tasks.as_deref()?.smoke_rwsem_task().cpu_ref(),
            TaskRef::SMOKE_RWLOCK => self.test_tasks.as_deref()?.smoke_rwlock_task().cpu_ref(),
            _ if task_ref.is_user() => self.user_candidate(task_ref)?.task.flow_cpu_ref(),
            _ if task_ref.is_ap_idle() => {
                super::smp_bringup::ap_current_task_candidate_by_ref(task_ref)?
                    .task
                    .flow_cpu_ref()
            }
            _ if task_ref.is_kernel() => super::kernel_task::task_by_ref(task_ref)?.flow_cpu_ref(),
            _ => None,
        }
    }

    /// Commit the selected user aggregate's address space before code on its
    /// restored kernel continuation can touch user memory.
    pub fn commit_dispatch_address_space(&self, task_ref: TaskRef) -> Result<bool, &'static str> {
        let satp = if task_ref.same_identity(TaskRef::KERNEL_INIT) {
            let Some(user_task_set) = self.user_task_set.as_deref() else {
                return Err("pid1-user-task-set-missing");
            };
            let Some(satp) = user_task_set.satp_token_for_task(task_ref) else {
                // The first BootTask -> KernelInitTask handoff precedes PID1's
                // address-space construction and therefore remains on the
                // current kernel address space.
                return Ok(false);
            };
            Some(satp)
        } else if task_ref.is_user() {
            if let Some(user_task_set) = self.user_task_set.as_deref() {
                Some(
                    user_task_set
                        .satp_token_for_task(task_ref)
                        .or_else(|| super::user_boot::smp_task_satp_token(task_ref, self.owner_cpu))
                        .ok_or("local-user-satp-token-missing")?,
                )
            } else {
                Some(
                    super::user_boot::smp_task_satp_token(task_ref, self.owner_cpu)
                        .ok_or("remote-user-satp-token-missing")?,
                )
            }
        } else {
            return Ok(false);
        };
        let satp = satp.ok_or("user-satp-token-missing")?;
        if crate::arch::riscv64::csr::read_satp() != satp {
            crate::arch::riscv64::csr::write_satp(satp);
            crate::arch::riscv64::csr::sfence_vma();
        }
        if crate::arch::riscv64::csr::read_satp() != satp {
            return Err("user-satp-readback-mismatch");
        }
        Ok(true)
    }

    /// Resolves the exact Schedule sender without copying Task or Flow facts.
    /// The caller supplies the CPU-local CurrentTask binding separately; this
    /// bridge proves that the signal source is that Task's unique fixed Flow
    /// on the Scheduler-owning CPU.
    pub fn schedule_sender_matches(
        &self,
        sender_flow_ref: TaskFlowRef,
        task_ref: TaskRef,
        cpu_ref: CpuRef,
    ) -> bool {
        let Some(candidate) = self.current_task_candidate(task_ref) else {
            return false;
        };
        candidate.task.task_ref().same_identity(task_ref)
            && candidate.task.state() == State::OnCpu
            && candidate.task.execution_authority() == TaskExecutionAuthority::Live
            && candidate.task.flow().same_identity(sender_flow_ref)
            && candidate.task.owns_flow(sender_flow_ref)
            && candidate.flow.flow_ref().same_identity(sender_flow_ref)
            && candidate.flow.declared()
            && candidate.flow.owner().same_identity(task_ref)
            && candidate.flow.cpu_ref() == Some(cpu_ref)
    }

    pub(crate) fn current_task_candidate(
        &self,
        task_ref: TaskRef,
    ) -> Option<super::current_task::CurrentTaskCandidate<'_>> {
        match task_ref {
            TaskRef::BOOT => Some(super::current_task::CurrentTaskCandidate {
                task: BootTask::canonical_task(),
                flow: BootTask::canonical_task().embedded_flow(),
            }),
            TaskRef::KERNEL_INIT => Some(super::current_task::CurrentTaskCandidate {
                task: self.kernel_init_task.as_deref()?.task(),
                flow: self.kernel_init_task.as_deref()?.task().embedded_flow(),
            }),
            TaskRef::KTHREADD => Some(super::current_task::CurrentTaskCandidate {
                task: self.kthreadd_task.as_deref()?.task(),
                flow: self.kthreadd_task.as_deref()?.task().embedded_flow(),
            }),
            TaskRef::SMOKE_SCHEDULER
            | TaskRef::SMOKE_MUTEX
            | TaskRef::SMOKE_RWSEM
            | TaskRef::SMOKE_RWLOCK => self
                .test_tasks
                .as_deref()?
                .current_task_candidate_by_ref(task_ref),
            _ if task_ref.is_user() => self.user_candidate(task_ref),
            _ if task_ref.is_ap_idle() => {
                super::smp_bringup::ap_current_task_candidate_by_ref(task_ref)
            }
            _ if task_ref.is_kernel() => super::kernel_task::task_candidate_by_ref(task_ref),
            _ => None,
        }
    }

    pub(crate) fn wake_for_inbound(&mut self, task_ref: TaskRef) -> Option<EventResult> {
        match task_ref {
            TaskRef::KERNEL_INIT => Some(
                self.kernel_init_task
                    .as_deref_mut()?
                    .task_mut()
                    .wake_for_scheduler_enqueue(),
            ),
            TaskRef::KTHREADD => Some(
                self.kthreadd_task
                    .as_deref_mut()?
                    .task_mut()
                    .wake_for_scheduler_enqueue(),
            ),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_scheduler_task_mut()
                    .task_mut()
                    .wake_for_scheduler_enqueue(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_mutex_task_mut()
                    .task_mut()
                    .wake_for_scheduler_enqueue(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwsem_task_mut()
                    .task_mut()
                    .wake_for_scheduler_enqueue(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwlock_task_mut()
                    .task_mut()
                    .wake_for_scheduler_enqueue(),
            ),
            _ if task_ref.is_user() => self
                .user_task_mut(task_ref)
                .map(Task::wake_for_scheduler_enqueue),
            _ if task_ref.is_ap_idle() => super::smp_bringup::ap_task_mut_by_ref(task_ref)
                .map(Task::wake_for_scheduler_enqueue),
            _ if task_ref.is_kernel() => Some(super::kernel_task::wake_for_enqueue(
                task_ref,
                self.owner_cpu,
            )),
            _ => None,
        }
    }

    pub fn prepare_prev_runnable(&mut self, task_ref: TaskRef) -> Option<bool> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::canonical_prepare_prev_runnable()),
            TaskRef::KERNEL_INIT => Some(
                self.kernel_init_task
                    .as_deref_mut()?
                    .task_mut()
                    .prepare_prev_runnable(),
            ),
            TaskRef::KTHREADD => Some(
                self.kthreadd_task
                    .as_deref_mut()?
                    .task_mut()
                    .prepare_prev_runnable(),
            ),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_scheduler_task_mut()
                    .task_mut()
                    .prepare_prev_runnable(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_mutex_task_mut()
                    .task_mut()
                    .prepare_prev_runnable(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwsem_task_mut()
                    .task_mut()
                    .prepare_prev_runnable(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwlock_task_mut()
                    .task_mut()
                    .prepare_prev_runnable(),
            ),
            _ if task_ref.is_user() => self
                .user_task_mut(task_ref)
                .map(Task::prepare_prev_runnable),
            _ if task_ref.is_ap_idle() => {
                Some(super::smp_bringup::ap_task_mut_by_ref(task_ref)?.prepare_prev_runnable())
            }
            _ if task_ref.is_kernel() => Some(
                super::kernel_task::task_mut_by_ref_on_cpu(task_ref, self.owner_cpu)?
                    .prepare_prev_runnable(),
            ),
            _ => None,
        }
    }

    pub fn deactivate(&mut self, task_ref: TaskRef) -> Option<EventResult> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::canonical_deactivate_from_scheduler()),
            TaskRef::KERNEL_INIT => Some(
                self.kernel_init_task
                    .as_deref_mut()?
                    .task_mut()
                    .deactivate_from_scheduler(),
            ),
            TaskRef::KTHREADD => Some(
                self.kthreadd_task
                    .as_deref_mut()?
                    .task_mut()
                    .deactivate_from_scheduler(),
            ),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_scheduler_task_mut()
                    .deactivate_from_scheduler(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_mutex_task_mut()
                    .deactivate_from_scheduler(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwsem_task_mut()
                    .deactivate_from_scheduler(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwlock_task_mut()
                    .deactivate_from_scheduler(),
            ),
            _ if task_ref.is_user() => {
                Some(self.user_task_mut(task_ref)?.deactivate_from_scheduler())
            }
            _ if task_ref.is_kernel() => Some(
                super::kernel_task::task_mut_by_ref_on_cpu(task_ref, self.owner_cpu)?
                    .deactivate_from_scheduler(),
            ),
            _ => None,
        }
    }

    pub fn switch_out_ready(&self, task_ref: TaskRef) -> bool {
        self.current_task_candidate(task_ref)
            .is_some_and(|candidate| {
                candidate.task.switch_out_ready() || candidate.task.terminal_switch_out_ready()
            })
    }

    pub fn switch_in_ready(&self, task_ref: TaskRef) -> bool {
        self.current_task_candidate(task_ref)
            .is_some_and(|candidate| candidate.task.switch_in_ready())
    }

    /// Resolves the one fixed Task/Flow pair before any context is saved.
    pub fn preflight_next_dispatch(
        &self,
        task_ref: TaskRef,
        cpu_ref: CpuRef,
    ) -> Option<NextDispatch> {
        let candidate = self.current_task_candidate(task_ref)?;
        let task = candidate.task;
        let flow = candidate.flow;
        let common = self.switch_in_ready(task_ref)
            && flow.flow_ref().is_valid()
            && task.owns_flow(flow.flow_ref())
            && flow.declared()
            && flow.owner().same_identity(task_ref);
        if !common {
            return None;
        }

        let flow_ref = task.flow();
        let context_epoch = task.context_epoch();
        let root_ref = task.root_trap_flow_ref();
        let cpu_matches =
            flow.cpu_ref() == Some(cpu_ref) || (task_ref.is_user() && flow.cpu_ref().is_none());
        let root_matches = !root_ref.is_valid()
            || TrapFlowType::active_leaf_preflight(
                root_ref,
                task_ref,
                flow_ref,
                cpu_ref,
                context_epoch,
            );
        (task.state() == State::Online
            && flow_ref.is_valid()
            && flow_ref.same_identity(flow.flow_ref())
            && task.breakpoint_matches(flow_ref)
            && flow.state() == State::Online
            && cpu_matches
            && root_matches)
            .then_some(NextDispatch::new(
                task_ref,
                flow_ref,
                cpu_ref,
                context_epoch,
                root_ref,
                (task.kernel_stack_base(), task.kernel_stack_top()),
                DispatchStackBinding::TaskOwned,
            ))
    }

    pub fn preflight_next_dispatch_on_user_carrier(
        &self,
        task_ref: TaskRef,
        cpu_ref: CpuRef,
    ) -> Option<NextDispatch> {
        if !task_ref.is_user() && !task_ref.same_identity(TaskRef::KERNEL_INIT) {
            return None;
        }
        let (stack_base, stack_top) = self.user_task_set.as_deref()?.carrier_stack_bounds()?;
        let live_sp = crate::arch::riscv64::csr::read_sp();
        if live_sp < stack_base || live_sp > stack_top {
            return None;
        }
        self.preflight_next_dispatch(task_ref, cpu_ref)
            .map(|dispatch| dispatch.with_simulated_user_carrier(stack_base, stack_top))
    }

    pub fn first_boot_handoff_preflight_ready(&self, dispatch: NextDispatch) -> bool {
        let Some(kernel_init_task) = self.kernel_init_task.as_deref() else {
            return false;
        };
        BootTask::canonical_task().switch_out_ready()
            && kernel_init_task.task().switch_in_ready()
            && dispatch.task_ref().same_identity(TaskRef::KERNEL_INIT)
            && dispatch
                .flow_ref()
                .same_identity(kernel_init_task.task().flow_ref())
    }

    pub fn task_on_cpu_identity_matches(&self, task_ref: TaskRef, identity: usize) -> bool {
        self.task_identity_ptr(task_ref) == Some(identity)
            && self
                .current_task_candidate(task_ref)
                .is_some_and(|candidate| candidate.task.state() == State::OnCpu)
    }

    pub fn suspend(&mut self, task_ref: TaskRef) -> Option<EventResult> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::suspend_canonical()),
            TaskRef::KERNEL_INIT => Some(self.kernel_init_task.as_deref_mut()?.suspend_from_cpu()),
            TaskRef::KTHREADD => Some(self.kthreadd_task.as_deref_mut()?.suspend_from_cpu()),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_scheduler_task_mut()
                    .suspend_from_cpu(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_mutex_task_mut()
                    .suspend_from_cpu(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwsem_task_mut()
                    .suspend_from_cpu(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwlock_task_mut()
                    .suspend_from_cpu(),
            ),
            _ if task_ref.is_user() => {
                if let Some(user_task_set) = self.user_task_set.as_deref_mut() {
                    Some(user_task_set.suspend_task(task_ref))
                } else {
                    Some(
                        super::user_boot::smp_task_mut_by_ref_on_cpu(task_ref, self.owner_cpu)?
                            .suspend_from_cpu(),
                    )
                }
            }
            _ if task_ref.is_ap_idle() => {
                Some(super::smp_bringup::ap_task_mut_by_ref(task_ref)?.suspend_from_cpu())
            }
            _ if task_ref.is_kernel() => Some(
                super::kernel_task::task_mut_by_ref_on_cpu(task_ref, self.owner_cpu)?
                    .suspend_from_cpu(),
            ),
            _ => None,
        }
    }

    pub fn save_core_context(&mut self, task_ref: TaskRef) -> Option<EventResult> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::save_core_context_canonical()),
            TaskRef::KERNEL_INIT => Some(
                self.kernel_init_task
                    .as_deref_mut()?
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            TaskRef::KTHREADD => Some(
                self.kthreadd_task
                    .as_deref_mut()?
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_scheduler_task_mut()
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_mutex_task_mut()
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwsem_task_mut()
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwlock_task_mut()
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            _ if task_ref.is_user() => Some(
                self.user_task_mut(task_ref)?
                    .save_core_context_for_suspend(),
            ),
            _ if task_ref.is_ap_idle() => Some(
                super::smp_bringup::ap_task_mut_by_ref(task_ref)?.save_core_context_for_suspend(),
            ),
            _ if task_ref.is_kernel() => Some(
                super::kernel_task::task_mut_by_ref_on_cpu(task_ref, self.owner_cpu)?
                    .save_core_context_for_suspend(),
            ),
            _ => None,
        }
    }

    pub fn suspend_after_core_context_save(&mut self, task_ref: TaskRef) -> Option<EventResult> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::suspend_after_core_context_save_canonical()),
            TaskRef::KERNEL_INIT => Some(
                self.kernel_init_task
                    .as_deref_mut()?
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            TaskRef::KTHREADD => Some(
                self.kthreadd_task
                    .as_deref_mut()?
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_scheduler_task_mut()
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_mutex_task_mut()
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwsem_task_mut()
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwlock_task_mut()
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            _ if task_ref.is_user() => Some(
                self.user_task_mut(task_ref)?
                    .suspend_after_core_context_save(),
            ),
            _ if task_ref.is_ap_idle() => Some(
                super::smp_bringup::ap_task_mut_by_ref(task_ref)?.suspend_after_core_context_save(),
            ),
            _ if task_ref.is_kernel() => Some(
                super::kernel_task::task_mut_by_ref_on_cpu(task_ref, self.owner_cpu)?
                    .suspend_after_core_context_save(),
            ),
            _ => None,
        }
    }

    fn task_mut_for_dispatch(&mut self, task_ref: TaskRef) -> Option<&mut Task> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::canonical_task_mut()),
            TaskRef::KERNEL_INIT => Some(self.kernel_init_task.as_deref_mut()?.task_mut()),
            TaskRef::KTHREADD => Some(self.kthreadd_task.as_deref_mut()?.task_mut()),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_scheduler_task_mut()
                    .task_mut(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_mutex_task_mut()
                    .task_mut(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwsem_task_mut()
                    .task_mut(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwlock_task_mut()
                    .task_mut(),
            ),
            _ if task_ref.is_user() => self.user_task_mut(task_ref),
            _ if task_ref.is_ap_idle() => super::smp_bringup::ap_task_mut_by_ref(task_ref),
            _ if task_ref.is_kernel() => {
                super::kernel_task::task_mut_by_ref_on_cpu(task_ref, self.owner_cpu)
            }
            _ => None,
        }
    }

    pub fn accept_task_dispatch(
        &mut self,
        dispatch: NextDispatch,
    ) -> Option<Result<TaskFlowEnterProof, super::state::EventError>> {
        let task = self.task_mut_for_dispatch(dispatch.task_ref())?;
        Some(task.dispatch_on_cpu(
            dispatch.flow_ref(),
            dispatch.context_epoch(),
            dispatch.cpu_ref(),
        ))
    }

    pub fn enter_task_flow(
        &mut self,
        dispatch: NextDispatch,
        proof: TaskFlowEnterProof,
    ) -> Option<EventResult> {
        // Resolve both bindings again after Dispatch. Together with the TP
        // check at finish_task_switch, these are the actual selected aggregate
        // and its embedded effective Flow, not copies from NextDispatch.
        let (current_task_ref, effective_flow_ref, current_stack_matches) = {
            let candidate = self.current_task_candidate(dispatch.task_ref())?;
            let stack_binding_matches = match dispatch.stack_binding {
                DispatchStackBinding::TaskOwned => {
                    dispatch.stack_base == candidate.task.kernel_stack_base()
                        && dispatch.stack_top == candidate.task.kernel_stack_top()
                        && candidate.task.current_stack_matches()
                }
                DispatchStackBinding::SimulatedUserCarrier => {
                    (dispatch.task_ref().is_user()
                        || dispatch.task_ref().same_identity(TaskRef::KERNEL_INIT))
                        && self.user_task_set.as_deref().is_some_and(|user_task_set| {
                            user_task_set
                                .carrier_stack_matches(dispatch.stack_base, dispatch.stack_top)
                        })
                }
            };
            let live_sp = crate::arch::riscv64::csr::read_sp();
            (
                candidate.task.task_ref(),
                candidate.flow.flow_ref(),
                stack_binding_matches
                    && live_sp >= dispatch.stack_base
                    && live_sp <= dispatch.stack_top,
            )
        };
        let task = self.task_mut_for_dispatch(dispatch.task_ref())?;
        if !current_stack_matches {
            let (live_sp, stack_base, stack_top) = task.current_stack_observation();
            trace_contextual_enter_stack_mismatch(
                dispatch.task_ref(),
                live_sp,
                stack_base,
                stack_top,
            );
        }
        let result = task.enter_flow_contextual(
            proof,
            current_task_ref,
            current_stack_matches,
            effective_flow_ref,
            dispatch.root_trap_flow_ref(),
        );
        if result.is_ok() && dispatch.root_trap_flow_ref().is_valid() {
            crate::objects::trap_type::record_leaf_switch_resume(dispatch.cpu_ref().logical_id());
        }
        Some(result)
    }

    pub fn task_identity_ptr(&self, task_ref: TaskRef) -> Option<usize> {
        let candidate = self.current_task_candidate(task_ref)?;
        Some(candidate.task as *const Task as usize)
    }

    /// Resolves stable Task identity metadata independently of class-queue
    /// membership. A blocked prev has already left its queue before SwitchTo
    /// preflight but still owns the context that must be saved.
    pub fn task_id(&self, task_ref: TaskRef) -> Option<usize> {
        let candidate = self.current_task_candidate(task_ref)?;
        Some(candidate.task.pid())
    }

    pub fn switch_context_mut_ptr(&mut self, task_ref: TaskRef) -> Option<*mut TaskSwitchContext> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::canonical_switch_context_mut()),
            TaskRef::KERNEL_INIT => {
                Some(self.kernel_init_task.as_deref_mut()?.switch_context_mut())
            }
            TaskRef::KTHREADD => Some(self.kthreadd_task.as_deref_mut()?.switch_context_mut()),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_scheduler_task_mut()
                    .switch_context_mut(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_mutex_task_mut()
                    .switch_context_mut(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwsem_task_mut()
                    .switch_context_mut(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .as_deref_mut()?
                    .smoke_rwlock_task_mut()
                    .switch_context_mut(),
            ),
            _ if task_ref.is_user() => self
                .user_task_mut(task_ref)
                .map(|task| task.switch_context_mut() as *mut TaskSwitchContext),
            _ if task_ref.is_ap_idle() => super::smp_bringup::ap_task_mut_by_ref(task_ref)
                .map(|task| task.switch_context_mut() as *mut TaskSwitchContext),
            _ if task_ref.is_kernel() => {
                super::kernel_task::task_mut_by_ref_on_cpu(task_ref, self.owner_cpu)
                    .map(|task| task.switch_context_mut() as *mut TaskSwitchContext)
            }
            _ => None,
        }
    }

    pub fn switch_context_ptr(&self, task_ref: TaskRef) -> Option<*const TaskSwitchContext> {
        let candidate = self.current_task_candidate(task_ref)?;
        Some(candidate.task.switch_context())
    }
}

fn trace_contextual_enter_stack_mismatch(
    task_ref: TaskRef,
    live_sp: usize,
    stack_base: usize,
    stack_top: usize,
) {
    use crate::arch::riscv64::sbi;

    sbi::putstr("failure_context TaskFlow.Enter task=");
    sbi::putstr(task_ref.name());
    sbi::putstr(" live_sp=0x");
    print_hex(live_sp);
    sbi::putstr(" stack_base=0x");
    print_hex(stack_base);
    sbi::putstr(" stack_top=0x");
    print_hex(stack_top);
    #[cfg(app_user_boot)]
    {
        sbi::putstr(" user_trap_base=0x");
        print_hex(super::user_boot::user_kernel_trap_stack_base());
        sbi::putstr(" user_trap_top=0x");
        print_hex(super::user_boot::user_kernel_trap_stack_top());
    }
    sbi::putchar(b'\n');
}

fn print_hex(value: usize) {
    let mut shift = usize::BITS as usize;
    while shift != 0 {
        shift -= 4;
        let nibble = ((value >> shift) & 0xf) as u8;
        let byte = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + (nibble - 10)
        };
        crate::arch::riscv64::sbi::putchar(byte);
    }
}
