use super::{
    boot_task::BootTask,
    cpu::CpuRef,
    rest_init::{KernelInitFlow, KernelInitTask, KthreaddFlow, KthreaddTask},
    scheduler::SchedulerTestTasks,
    state::{EventResult, State},
    task::{Task, TaskExecutionAuthority, TaskRef},
    task_flow::TaskFlowRef,
    user_boot::{UserAppFlow, UserTaskSet},
};
use crate::{arch::riscv64::task_switch::TaskSwitchContext, flows::boot_idle_flow::BootIdleFlow};

/// Resolves stable scheduler references into task/flow carriers owned by the
/// surrounding Context. Scheduler itself never owns or names those carriers.
pub struct SchedulerTaskAccess<'a> {
    kernel_init_task: &'a mut KernelInitTask,
    kernel_init_flow: &'a mut KernelInitFlow,
    user_app_flow: &'a UserAppFlow,
    kthreadd_task: &'a mut KthreaddTask,
    kthreadd_flow: &'a mut KthreaddFlow,
    boot_idle_flow: &'a BootIdleFlow,
    user_task_set: &'a mut UserTaskSet,
    test_tasks: &'a mut SchedulerTestTasks,
}

impl<'a> SchedulerTaskAccess<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kernel_init_task: &'a mut KernelInitTask,
        kernel_init_flow: &'a mut KernelInitFlow,
        user_app_flow: &'a UserAppFlow,
        kthreadd_task: &'a mut KthreaddTask,
        kthreadd_flow: &'a mut KthreaddFlow,
        boot_idle_flow: &'a BootIdleFlow,
        user_task_set: &'a mut UserTaskSet,
        test_tasks: &'a mut SchedulerTestTasks,
    ) -> Self {
        Self {
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            boot_idle_flow,
            user_task_set,
            test_tasks,
        }
    }

    pub fn effective_cpu_ref(&self, task_ref: TaskRef) -> Option<CpuRef> {
        match task_ref {
            TaskRef::BOOT => self.boot_idle_flow.cpu_ref(),
            TaskRef::KERNEL_INIT => self
                .user_app_flow
                .cpu_ref()
                .or_else(|| self.kernel_init_flow.cpu_ref()),
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
    /// bridge proves that the signal source is that Task's unique active Flow
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
            && candidate.task.active_flow().same_identity(sender_flow_ref)
            && candidate.task.owns_flow(sender_flow_ref)
            && candidate.flow.flow_ref().same_identity(sender_flow_ref)
            && candidate.flow.declared()
            && candidate.flow.active()
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
                flow: self.boot_idle_flow.core(),
            }),
            TaskRef::KERNEL_INIT => {
                let task = self.kernel_init_task.task();
                let active_flow = task.active_flow();
                let flow = if !active_flow.is_valid()
                    || active_flow.same_identity(self.kernel_init_flow.flow_ref())
                {
                    self.kernel_init_flow.core()
                } else if active_flow.same_identity(self.user_app_flow.flow_ref()) {
                    self.user_app_flow.current_core()
                } else {
                    return None;
                };
                Some(super::current_task::CurrentTaskCandidate { task, flow })
            }
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

    /// Preflights both the Task.Continue acceptance and the exactly-one Flow
    /// Startup/Continue delivery that must follow it on the selected stack.
    pub fn switch_signal_capacity_ready(&self, task_ref: TaskRef, cpu_ref: CpuRef) -> bool {
        let Some(candidate) = self.current_task_candidate(task_ref) else {
            return false;
        };
        let task = candidate.task;
        let flow = candidate.flow;
        let initial = !task.active_flow().is_valid();
        let expected_flow = if initial {
            task.initial_flow()
        } else {
            task.active_flow()
        };
        let cpu_matches = flow.cpu_ref() == Some(cpu_ref)
            || (initial && task_ref.is_user() && flow.cpu_ref().is_none());

        self.switch_in_ready(task_ref)
            && expected_flow.is_valid()
            && expected_flow.same_identity(flow.flow_ref())
            && task.owns_flow(expected_flow)
            && flow.declared()
            && flow.owner().same_identity(task_ref)
            && cpu_matches
            && if initial {
                flow.state() == State::Base && !flow.active()
            } else {
                flow.active()
                    && (flow.state() == State::Online
                        || (task_ref == TaskRef::BOOT && flow.state() == State::Ready))
            }
    }

    pub fn initial_flow_will_start(&self, task_ref: TaskRef) -> Option<bool> {
        let candidate = self.current_task_candidate(task_ref)?;
        Some(!candidate.task.active_flow().is_valid())
    }

    pub fn first_boot_handoff_preflight_ready(&self) -> bool {
        BootTask::canonical_task().switch_out_ready()
            && self.kernel_init_task.task().switch_in_ready()
            && self.switch_signal_capacity_ready(TaskRef::KERNEL_INIT, CpuRef::new(0))
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
            TaskRef::KERNEL_INIT => {
                Some(if self.kernel_init_task.task().active_flow().is_valid() {
                    self.kernel_init_task.suspend_from_cpu()
                } else {
                    self.kernel_init_task.task_mut().disable()
                })
            }
            TaskRef::KTHREADD => Some(if self.kthreadd_task.task().active_flow().is_valid() {
                self.kthreadd_task.suspend_from_cpu()
            } else {
                self.kthreadd_task.task_mut().disable()
            }),
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

    pub fn accept_task_continue(&mut self, task_ref: TaskRef) -> Option<EventResult> {
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

    pub fn continue_flow_after_task(
        &mut self,
        task_ref: TaskRef,
        cpu_ref: CpuRef,
    ) -> Option<EventResult> {
        match task_ref {
            TaskRef::BOOT => Some(
                self.boot_idle_flow
                    .continue_active(BootTask::canonical_task()),
            ),
            TaskRef::KERNEL_INIT => Some(
                if self.kernel_init_flow.state() == State::Base
                    && !self.kernel_init_task.task().active_flow().is_valid()
                {
                    self.kernel_init_flow.start_initial(self.kernel_init_task)
                } else if self
                    .kernel_init_task
                    .task()
                    .active_flow()
                    .same_identity(self.kernel_init_flow.flow_ref())
                {
                    self.kernel_init_flow.continue_active(self.kernel_init_task)
                } else {
                    self.user_app_flow.continue_active(self.kernel_init_task)
                },
            ),
            TaskRef::KTHREADD => Some(
                if self.kthreadd_flow.state() == State::Base
                    && !self.kthreadd_task.task().active_flow().is_valid()
                {
                    self.kthreadd_flow.start_initial(self.kthreadd_task)
                } else {
                    self.kthreadd_flow.continue_active(self.kthreadd_task)
                },
            ),
            TaskRef::SMOKE_SCHEDULER => Some(
                self.test_tasks
                    .smoke_scheduler_task_mut()
                    .continue_flow_after_task(),
            ),
            TaskRef::SMOKE_MUTEX => Some(
                self.test_tasks
                    .smoke_mutex_task_mut()
                    .continue_flow_after_task(),
            ),
            TaskRef::SMOKE_RWSEM => Some(
                self.test_tasks
                    .smoke_rwsem_task_mut()
                    .continue_flow_after_task(),
            ),
            TaskRef::SMOKE_RWLOCK => Some(
                self.test_tasks
                    .smoke_rwlock_task_mut()
                    .continue_flow_after_task(),
            ),
            _ if task_ref.is_user() => {
                Some(self.user_task_set.continue_task_flow(task_ref, cpu_ref))
            }
            _ => None,
        }
    }

    /// Delivers the identity-schedule return to the already-active Flow. This
    /// deliberately does not invoke Task.Continue or change Task lifecycle.
    pub fn continue_identity_flow(
        &self,
        sender_flow_ref: TaskFlowRef,
        task_ref: TaskRef,
        cpu_ref: CpuRef,
    ) -> Option<EventResult> {
        if !self.schedule_sender_matches(sender_flow_ref, task_ref, cpu_ref) {
            return None;
        }
        match task_ref {
            TaskRef::BOOT => Some(
                self.boot_idle_flow
                    .continue_active(BootTask::canonical_task()),
            ),
            TaskRef::KERNEL_INIT => Some(
                if self
                    .kernel_init_task
                    .task()
                    .active_flow()
                    .same_identity(self.kernel_init_flow.flow_ref())
                {
                    self.kernel_init_flow.continue_active(self.kernel_init_task)
                } else {
                    self.user_app_flow.continue_active(self.kernel_init_task)
                },
            ),
            TaskRef::KTHREADD => Some(self.kthreadd_flow.continue_active(self.kthreadd_task)),
            TaskRef::SMOKE_SCHEDULER => {
                Some(self.test_tasks.smoke_scheduler_task().continue_active())
            }
            TaskRef::SMOKE_MUTEX => Some(self.test_tasks.smoke_mutex_task().continue_active()),
            TaskRef::SMOKE_RWSEM => Some(self.test_tasks.smoke_rwsem_task().continue_active()),
            TaskRef::SMOKE_RWLOCK => Some(self.test_tasks.smoke_rwlock_task().continue_active()),
            _ if task_ref.is_user() => Some(self.user_task_set.continue_active_flow(task_ref)),
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
