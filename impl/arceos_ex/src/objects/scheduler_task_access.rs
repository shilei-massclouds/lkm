use super::{
    boot_task::BootTask,
    cpu::CpuRef,
    rest_init::{KernelInitFlow, KernelInitTask, KthreaddFlow, KthreaddTask},
    scheduler::SchedulerTestTasks,
    state::{EventResult, State},
    task::{Task, TaskExecutionAuthority, TaskRef},
    task_flow::TaskFlowRef,
    user_boot::UserTaskSet,
};
use crate::arch::riscv64::task_switch::TaskSwitchContext;

/// Stable, generation-carrying result of the next-task preflight. Scheduler
/// stores this value across the physical stack switch and must dispatch the
/// exact Task/Flow pair recorded here.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct NextDispatch {
    task_ref: TaskRef,
    flow_ref: TaskFlowRef,
}

impl NextDispatch {
    const fn new(task_ref: TaskRef, flow_ref: TaskFlowRef) -> Self {
        Self { task_ref, flow_ref }
    }

    pub const fn task_ref(self) -> TaskRef {
        self.task_ref
    }

    pub const fn flow_ref(self) -> TaskFlowRef {
        self.flow_ref
    }
}

/// Resolves stable scheduler references into task/flow carriers owned by the
/// surrounding Context. Scheduler itself never owns or names those carriers.
pub struct SchedulerTaskAccess<'a> {
    kernel_init_task: &'a mut KernelInitTask,
    kernel_init_flow: &'a mut KernelInitFlow,
    kthreadd_task: &'a mut KthreaddTask,
    kthreadd_flow: &'a mut KthreaddFlow,
    boot_flow: &'a super::task_flow::TaskFlow,
    user_task_set: &'a mut UserTaskSet,
    test_tasks: &'a mut SchedulerTestTasks,
}

impl<'a> SchedulerTaskAccess<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kernel_init_task: &'a mut KernelInitTask,
        kernel_init_flow: &'a mut KernelInitFlow,
        kthreadd_task: &'a mut KthreaddTask,
        kthreadd_flow: &'a mut KthreaddFlow,
        boot_flow: &'a super::task_flow::TaskFlow,
        user_task_set: &'a mut UserTaskSet,
        test_tasks: &'a mut SchedulerTestTasks,
    ) -> Self {
        Self {
            kernel_init_task,
            kernel_init_flow,
            kthreadd_task,
            kthreadd_flow,
            boot_flow,
            user_task_set,
            test_tasks,
        }
    }

    pub fn effective_cpu_ref(&self, task_ref: TaskRef) -> Option<CpuRef> {
        match task_ref {
            TaskRef::BOOT => self.boot_flow.cpu_ref(),
            TaskRef::KERNEL_INIT => self.kernel_init_flow.cpu_ref(),
            TaskRef::KTHREADD => self.kthreadd_flow.cpu_ref(),
            TaskRef::SMOKE_SCHEDULER => self.test_tasks.smoke_scheduler_task().cpu_ref(),
            TaskRef::SMOKE_MUTEX => self.test_tasks.smoke_mutex_task().cpu_ref(),
            TaskRef::SMOKE_RWSEM => self.test_tasks.smoke_rwsem_task().cpu_ref(),
            TaskRef::SMOKE_RWLOCK => self.test_tasks.smoke_rwlock_task().cpu_ref(),
            _ if task_ref.is_user() => self.user_task_set.cpu_ref_for_task(task_ref),
            _ => None,
        }
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

    fn current_task_candidate(
        &self,
        task_ref: TaskRef,
    ) -> Option<super::current_task::CurrentTaskCandidate<'_>> {
        match task_ref {
            TaskRef::BOOT => Some(super::current_task::CurrentTaskCandidate {
                task: BootTask::canonical_task(),
                flow: self.boot_flow,
            }),
            TaskRef::KERNEL_INIT => Some(super::current_task::CurrentTaskCandidate {
                task: self.kernel_init_task.task(),
                flow: self.kernel_init_flow.core(),
            }),
            TaskRef::KTHREADD => Some(super::current_task::CurrentTaskCandidate {
                task: self.kthreadd_task.task(),
                flow: self.kthreadd_flow.core(),
            }),
            TaskRef::SMOKE_SCHEDULER
            | TaskRef::SMOKE_MUTEX
            | TaskRef::SMOKE_RWSEM
            | TaskRef::SMOKE_RWLOCK => self.test_tasks.current_task_candidate_by_ref(task_ref),
            _ if task_ref.is_user() => self.user_task_set.current_task_candidate_by_ref(task_ref),
            _ => None,
        }
    }

    pub fn prepare_prev_runnable(&mut self, task_ref: TaskRef) -> Option<bool> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::canonical_prepare_prev_runnable()),
            TaskRef::KERNEL_INIT => Some(self.kernel_init_task.task_mut().prepare_prev_runnable()),
            TaskRef::KTHREADD => Some(self.kthreadd_task.task_mut().prepare_prev_runnable()),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .smoke_scheduler_task_mut()
                    .task_mut()
                    .prepare_prev_runnable(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .smoke_mutex_task_mut()
                    .task_mut()
                    .prepare_prev_runnable(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .smoke_rwsem_task_mut()
                    .task_mut()
                    .prepare_prev_runnable(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .smoke_rwlock_task_mut()
                    .task_mut()
                    .prepare_prev_runnable(),
            ),
            _ if task_ref.is_user() => self
                .user_task_set
                .task_mut_by_ref(task_ref)
                .map(Task::prepare_prev_runnable),
            _ => None,
        }
    }

    pub fn deactivate(&mut self, task_ref: TaskRef) -> Option<EventResult> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::canonical_deactivate_from_scheduler()),
            TaskRef::KERNEL_INIT => {
                Some(self.kernel_init_task.task_mut().deactivate_from_scheduler())
            }
            TaskRef::KTHREADD => Some(self.kthreadd_task.task_mut().deactivate_from_scheduler()),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .smoke_scheduler_task_mut()
                    .deactivate_from_scheduler(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .smoke_mutex_task_mut()
                    .deactivate_from_scheduler(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .smoke_rwsem_task_mut()
                    .deactivate_from_scheduler(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .smoke_rwlock_task_mut()
                    .deactivate_from_scheduler(),
            ),
            _ if task_ref.is_user() => Some(
                self.user_task_set
                    .task_mut_by_ref(task_ref)?
                    .deactivate_from_scheduler(),
            ),
            _ => None,
        }
    }

    pub fn switch_out_ready(&self, task_ref: TaskRef) -> bool {
        match task_ref {
            TaskRef::BOOT => BootTask::canonical_task().switch_out_ready(),
            TaskRef::KERNEL_INIT => {
                self.kernel_init_task.task().switch_out_ready()
                    || self.kernel_init_task.task().terminal_switch_out_ready()
            }
            TaskRef::KTHREADD => {
                self.kthreadd_task.task().switch_out_ready()
                    || self.kthreadd_task.task().terminal_switch_out_ready()
            }
            TaskRef::SMOKE_SCHEDULER => self
                .test_tasks
                .smoke_scheduler_task()
                .task()
                .switch_out_ready(),
            TaskRef::SMOKE_MUTEX => self.test_tasks.smoke_mutex_task().task().switch_out_ready(),
            TaskRef::SMOKE_RWSEM => self.test_tasks.smoke_rwsem_task().task().switch_out_ready(),
            TaskRef::SMOKE_RWLOCK => self
                .test_tasks
                .smoke_rwlock_task()
                .task()
                .switch_out_ready(),
            _ if task_ref.is_user() => self.user_task_set.task_switch_out_ready(task_ref),
            _ => false,
        }
    }

    pub fn switch_in_ready(&self, task_ref: TaskRef) -> bool {
        match task_ref {
            TaskRef::BOOT => BootTask::canonical_task().switch_in_ready(),
            TaskRef::KERNEL_INIT => self.kernel_init_task.task().switch_in_ready(),
            TaskRef::KTHREADD => self.kthreadd_task.task().switch_in_ready(),
            TaskRef::SMOKE_SCHEDULER => self
                .test_tasks
                .smoke_scheduler_task()
                .task()
                .switch_in_ready(),
            TaskRef::SMOKE_MUTEX => self.test_tasks.smoke_mutex_task().task().switch_in_ready(),
            TaskRef::SMOKE_RWSEM => self.test_tasks.smoke_rwsem_task().task().switch_in_ready(),
            TaskRef::SMOKE_RWLOCK => self.test_tasks.smoke_rwlock_task().task().switch_in_ready(),
            _ if task_ref.is_user() => self.user_task_set.task_switch_in_ready(task_ref),
            _ => false,
        }
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
        let cpu_matches =
            flow.cpu_ref() == Some(cpu_ref) || (task_ref.is_user() && flow.cpu_ref().is_none());
        (task.state() == State::Online
            && flow_ref.is_valid()
            && flow_ref.same_identity(flow.flow_ref())
            && task.breakpoint_matches(flow_ref)
            && flow.state() == State::Online
            && cpu_matches)
            .then_some(NextDispatch::new(task_ref, flow_ref))
    }

    pub fn first_boot_handoff_preflight_ready(&self, dispatch: NextDispatch) -> bool {
        BootTask::canonical_task().switch_out_ready()
            && self.kernel_init_task.task().switch_in_ready()
            && dispatch.task_ref().same_identity(TaskRef::KERNEL_INIT)
            && dispatch
                .flow_ref()
                .same_identity(self.kernel_init_flow.flow_ref())
    }

    pub fn task_on_cpu_identity_matches(&self, task_ref: TaskRef, identity: usize) -> bool {
        self.task_identity_ptr(task_ref) == Some(identity)
            && match task_ref {
                TaskRef::BOOT => BootTask::canonical_task().state() == State::OnCpu,
                TaskRef::KERNEL_INIT => self.kernel_init_task.state() == State::OnCpu,
                TaskRef::KTHREADD => self.kthreadd_task.state() == State::OnCpu,
                TaskRef::SMOKE_SCHEDULER => {
                    self.test_tasks.smoke_scheduler_task().state() == State::OnCpu
                }
                TaskRef::SMOKE_MUTEX => self.test_tasks.smoke_mutex_task().state() == State::OnCpu,
                TaskRef::SMOKE_RWSEM => self.test_tasks.smoke_rwsem_task().state() == State::OnCpu,
                TaskRef::SMOKE_RWLOCK => {
                    self.test_tasks.smoke_rwlock_task().state() == State::OnCpu
                }
                _ if task_ref.is_user() => self
                    .user_task_set
                    .task_state(task_ref)
                    .is_some_and(|state| state == State::OnCpu),
                _ => false,
            }
    }

    pub fn suspend(&mut self, task_ref: TaskRef) -> Option<EventResult> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::suspend_canonical()),
            TaskRef::KERNEL_INIT => Some(self.kernel_init_task.suspend_from_cpu()),
            TaskRef::KTHREADD => Some(self.kthreadd_task.suspend_from_cpu()),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .smoke_scheduler_task_mut()
                    .suspend_from_cpu(),
            ),
            TaskRef::SMOKE_MUTEX => Some(self.test_tasks.smoke_mutex_task_mut().suspend_from_cpu()),
            TaskRef::SMOKE_RWSEM => Some(self.test_tasks.smoke_rwsem_task_mut().suspend_from_cpu()),
            TaskRef::SMOKE_RWLOCK => {
                Some(self.test_tasks.smoke_rwlock_task_mut().suspend_from_cpu())
            }
            _ if task_ref.is_user() => Some(self.user_task_set.suspend_task(task_ref)),
            _ => None,
        }
    }

    pub fn save_core_context(&mut self, task_ref: TaskRef) -> Option<EventResult> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::save_core_context_canonical()),
            TaskRef::KERNEL_INIT => Some(
                self.kernel_init_task
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            TaskRef::KTHREADD => Some(
                self.kthreadd_task
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .smoke_scheduler_task_mut()
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .smoke_mutex_task_mut()
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .smoke_rwsem_task_mut()
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .smoke_rwlock_task_mut()
                    .task_mut()
                    .save_core_context_for_suspend(),
            ),
            _ if task_ref.is_user() => Some(
                self.user_task_set
                    .task_mut_by_ref(task_ref)?
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
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            TaskRef::KTHREADD => Some(
                self.kthreadd_task
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .smoke_scheduler_task_mut()
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .smoke_mutex_task_mut()
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .smoke_rwsem_task_mut()
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .smoke_rwlock_task_mut()
                    .task_mut()
                    .suspend_after_core_context_save(),
            ),
            _ if task_ref.is_user() => Some(
                self.user_task_set
                    .task_mut_by_ref(task_ref)?
                    .suspend_after_core_context_save(),
            ),
            _ => None,
        }
    }

    pub fn accept_task_continue(&mut self, dispatch: NextDispatch) -> Option<EventResult> {
        let task_ref = dispatch.task_ref();
        match task_ref {
            TaskRef::BOOT => Some(BootTask::continue_canonical()),
            TaskRef::KERNEL_INIT => Some(self.kernel_init_task.continue_on_cpu()),
            TaskRef::KTHREADD => Some(self.kthreadd_task.continue_on_cpu()),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .smoke_scheduler_task_mut()
                    .accept_task_continue(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .smoke_mutex_task_mut()
                    .accept_task_continue(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .smoke_rwsem_task_mut()
                    .accept_task_continue(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .smoke_rwlock_task_mut()
                    .accept_task_continue(),
            ),
            _ if task_ref.is_user() => Some(self.user_task_set.accept_task_continue(task_ref)),
            _ => None,
        }
    }

    pub fn emit_flow_continue(&self, dispatch: NextDispatch) -> Option<EventResult> {
        let task_ref = dispatch.task_ref();
        let flow_ref = dispatch.flow_ref();
        match task_ref {
            TaskRef::BOOT if flow_ref.same_identity(self.boot_flow.flow_ref()) => Some(
                (BootTask::canonical_task().flow().same_identity(flow_ref))
                    .then_some(())
                    .ok_or_else(|| {
                        super::state::EventError::failed(
                            super::state::EventErrorCode::ConditionFailed,
                            super::state::LifecycleEvent::Continue,
                            self.boot_flow.state(),
                            State::Online,
                            State::Online,
                        )
                    }),
            ),
            TaskRef::KERNEL_INIT if flow_ref.same_identity(self.kernel_init_flow.flow_ref()) => {
                Some(self.kernel_init_flow.continue_flow(self.kernel_init_task))
            }
            TaskRef::KTHREADD if flow_ref.same_identity(self.kthreadd_flow.flow_ref()) => {
                Some(self.kthreadd_flow.continue_flow(self.kthreadd_task))
            }
            TaskRef::SMOKE_SCHEDULER
                if flow_ref.same_identity(self.test_tasks.smoke_scheduler_task().flow_ref()) =>
            {
                Some(self.test_tasks.smoke_scheduler_task().continue_flow())
            }
            TaskRef::SMOKE_MUTEX
                if flow_ref.same_identity(self.test_tasks.smoke_mutex_task().flow_ref()) =>
            {
                Some(self.test_tasks.smoke_mutex_task().continue_flow())
            }
            TaskRef::SMOKE_RWSEM
                if flow_ref.same_identity(self.test_tasks.smoke_rwsem_task().flow_ref()) =>
            {
                Some(self.test_tasks.smoke_rwsem_task().continue_flow())
            }
            TaskRef::SMOKE_RWLOCK
                if flow_ref.same_identity(self.test_tasks.smoke_rwlock_task().flow_ref()) =>
            {
                Some(self.test_tasks.smoke_rwlock_task().continue_flow())
            }
            _ if task_ref.is_user() => Some(self.user_task_set.continue_flow(task_ref)),
            _ => None,
        }
    }

    pub fn task_identity_ptr(&self, task_ref: TaskRef) -> Option<usize> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::canonical_task() as *const Task as usize),
            TaskRef::KERNEL_INIT => Some(self.kernel_init_task.task() as *const Task as usize),
            TaskRef::KTHREADD => Some(self.kthreadd_task.task() as *const Task as usize),
            TaskRef::SMOKE_SCHEDULER => Some(self.test_tasks.smoke_scheduler_task().task_ptr()),
            TaskRef::SMOKE_MUTEX => Some(self.test_tasks.smoke_mutex_task().task_ptr()),
            TaskRef::SMOKE_RWSEM => Some(self.test_tasks.smoke_rwsem_task().task_ptr()),
            TaskRef::SMOKE_RWLOCK => Some(self.test_tasks.smoke_rwlock_task().task_ptr()),
            _ if task_ref.is_user() => self.user_task_set.task_identity_ptr(task_ref),
            _ => None,
        }
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
            TaskRef::KERNEL_INIT => Some(self.kernel_init_task.switch_context_mut()),
            TaskRef::KTHREADD => Some(self.kthreadd_task.switch_context_mut()),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .smoke_scheduler_task_mut()
                    .switch_context_mut(),
            ),
            TaskRef::SMOKE_MUTEX => {
                Some(self.test_tasks.smoke_mutex_task_mut().switch_context_mut())
            }
            TaskRef::SMOKE_RWSEM => {
                Some(self.test_tasks.smoke_rwsem_task_mut().switch_context_mut())
            }
            TaskRef::SMOKE_RWLOCK => {
                Some(self.test_tasks.smoke_rwlock_task_mut().switch_context_mut())
            }
            _ => None,
        }
    }

    pub fn switch_context_ptr(&self, task_ref: TaskRef) -> Option<*const TaskSwitchContext> {
        match task_ref {
            TaskRef::BOOT => Some(BootTask::canonical_switch_context()),
            TaskRef::KERNEL_INIT => Some(self.kernel_init_task.switch_context()),
            TaskRef::KTHREADD => Some(self.kthreadd_task.switch_context()),
            TaskRef::SMOKE_SCHEDULER => {
                Some(self.test_tasks.smoke_scheduler_task().switch_context())
            }
            TaskRef::SMOKE_MUTEX => Some(self.test_tasks.smoke_mutex_task().switch_context()),
            TaskRef::SMOKE_RWSEM => Some(self.test_tasks.smoke_rwsem_task().switch_context()),
            TaskRef::SMOKE_RWLOCK => Some(self.test_tasks.smoke_rwlock_task().switch_context()),
            _ => None,
        }
    }
}
