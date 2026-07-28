mod entry;

use crate::{
    checkpoint::Checkpoint,
    objects::{
        boot_task::BootTask,
        cpu_control::LocalInterruptControl,
        cpu_group::{CpuGroup, CurrentCpu},
        current_task::CurrentTask,
        rest_init::{
            KernelInitFlow, KernelInitTask, KthreaddFlow, KthreaddReadyGate, KthreaddTask,
        },
        scheduler::Scheduler,
        state::{EventResult, LifecycleEvent, State, failed_condition},
        task::{Task, TaskRef},
        task_flow::{TaskFlow, TaskFlowRef},
        user_boot::{UserAppFlow, UserTaskSet},
    },
};

pub struct BootIdleFlow {
    flow: TaskFlow,
    first_schedule_committed: bool,
    idle_entry_prepared: bool,
    cpu_startup_entry_ready: bool,
    idle_loop_entered: bool,
    idle_cycle_committed: bool,
    idle_cycle_started: bool,
    need_resched_clear_before_wait: bool,
    observed_no_need_resched: bool,
    idle_polling_set: bool,
    idle_polling_rmb_before_sleep_check: bool,
    nohz_idle_entered: bool,
    nohz_run_idle_balance_done: bool,
    local_irq_disabled_for_sleep: bool,
    local_irq_save_count_for_sleep: usize,
    local_irq_restore_count_for_sleep: usize,
    arch_cpu_idle_enter_done: bool,
    arch_cpu_idle_exit_done: bool,
    rcu_nocb_deferred_wakeup_flushed: bool,
    cpu_offline_dead_path_not_taken: bool,
    poll_or_cpuidle_path_deferred: bool,
    idle_wait_committed: bool,
    idle_wait_path_deferred: bool,
    need_resched_set_for_schedule: bool,
    observed_need_resched: bool,
    idle_polling_cleared: bool,
    preempt_need_resched_set: bool,
    nohz_idle_exited: bool,
    polling_clear_mb_before_flush: bool,
    smp_call_function_queue_flushed: bool,
    idle_schedule_requested: bool,
    idle_schedule_returned: bool,
    need_resched_drained: bool,
    livepatch_state_update_deferred: bool,
    idle_loop_continues: bool,
    boot_init_handoff_complete: bool,
    boot_cpu_hotplug_online: bool,
    secondary_cpus_not_started: bool,
    kernel_init_task_switch_handoff_ready: bool,
}

/// Continues into the first BootIdleFlow leaf after BootTask is restored.
pub(super) fn preset_entry(ctx: &mut crate::context::Context) -> ! {
    entry::preset(ctx)
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn entry_is_online() -> bool {
    entry::is_online()
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
impl BootIdleFlow {
    pub const fn new() -> Self {
        Self {
            flow: TaskFlow::new_static(TaskFlowRef::BOOT_IDLE),
            first_schedule_committed: false,
            idle_entry_prepared: false,
            cpu_startup_entry_ready: false,
            idle_loop_entered: false,
            idle_cycle_committed: false,
            idle_cycle_started: false,
            need_resched_clear_before_wait: false,
            observed_no_need_resched: false,
            idle_polling_set: false,
            idle_polling_rmb_before_sleep_check: false,
            nohz_idle_entered: false,
            nohz_run_idle_balance_done: false,
            local_irq_disabled_for_sleep: false,
            local_irq_save_count_for_sleep: 0,
            local_irq_restore_count_for_sleep: 0,
            arch_cpu_idle_enter_done: false,
            arch_cpu_idle_exit_done: false,
            rcu_nocb_deferred_wakeup_flushed: false,
            cpu_offline_dead_path_not_taken: false,
            poll_or_cpuidle_path_deferred: false,
            idle_wait_committed: false,
            idle_wait_path_deferred: false,
            need_resched_set_for_schedule: false,
            observed_need_resched: false,
            idle_polling_cleared: false,
            preempt_need_resched_set: false,
            nohz_idle_exited: false,
            polling_clear_mb_before_flush: false,
            smp_call_function_queue_flushed: false,
            idle_schedule_requested: false,
            idle_schedule_returned: false,
            need_resched_drained: false,
            livepatch_state_update_deferred: false,
            idle_loop_continues: false,
            boot_init_handoff_complete: false,
            boot_cpu_hotplug_online: false,
            secondary_cpus_not_started: true,
            kernel_init_task_switch_handoff_ready: false,
        }
    }

    pub fn continue_active(&self, boot_task: &Task) -> EventResult {
        if self.flow.state() != State::Ready
            || !self.flow.active()
            || !boot_task.active_flow().same_identity(self.flow.flow_ref())
            || !crate::objects::task_flow::task_flow_execution_guard_satisfied(
                &self.flow, boot_task,
            )
        {
            return failed_condition(
                LifecycleEvent::Continue,
                self.flow.state(),
                State::Ready,
                State::Ready,
            );
        }
        Ok(())
    }

    pub const fn state(&self) -> State {
        self.flow.state()
    }

    pub const fn cpu_ref(&self) -> Option<crate::objects::cpu::CpuRef> {
        self.flow.cpu_ref()
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

    pub const fn idle_polling_rmb_before_sleep_check(&self) -> bool {
        self.idle_polling_rmb_before_sleep_check
    }

    pub const fn nohz_idle_entered(&self) -> bool {
        self.nohz_idle_entered
    }

    pub const fn nohz_run_idle_balance_done(&self) -> bool {
        self.nohz_run_idle_balance_done
    }

    pub const fn local_irq_disabled_for_sleep(&self) -> bool {
        self.local_irq_disabled_for_sleep
    }

    pub const fn local_irq_save_count_for_sleep(&self) -> usize {
        self.local_irq_save_count_for_sleep
    }

    pub const fn local_irq_restore_count_for_sleep(&self) -> usize {
        self.local_irq_restore_count_for_sleep
    }

    pub const fn arch_cpu_idle_enter_done(&self) -> bool {
        self.arch_cpu_idle_enter_done
    }

    pub const fn arch_cpu_idle_exit_done(&self) -> bool {
        self.arch_cpu_idle_exit_done
    }

    pub const fn rcu_nocb_deferred_wakeup_flushed(&self) -> bool {
        self.rcu_nocb_deferred_wakeup_flushed
    }

    pub const fn cpu_offline_dead_path_not_taken(&self) -> bool {
        self.cpu_offline_dead_path_not_taken
    }

    pub const fn poll_or_cpuidle_path_deferred(&self) -> bool {
        self.poll_or_cpuidle_path_deferred
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

    pub const fn preempt_need_resched_set(&self) -> bool {
        self.preempt_need_resched_set
    }

    pub const fn nohz_idle_exited(&self) -> bool {
        self.nohz_idle_exited
    }

    pub const fn polling_clear_mb_before_flush(&self) -> bool {
        self.polling_clear_mb_before_flush
    }

    pub const fn smp_call_function_queue_flushed(&self) -> bool {
        self.smp_call_function_queue_flushed
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

    pub const fn livepatch_state_update_deferred(&self) -> bool {
        self.livepatch_state_update_deferred
    }

    pub const fn idle_loop_continues(&self) -> bool {
        self.idle_loop_continues
    }

    pub const fn representative_need_resched_cycle_committed(&self) -> bool {
        self.idle_cycle_started
            && self.need_resched_clear_before_wait
            && self.observed_no_need_resched
            && self.idle_polling_set
            && self.idle_polling_rmb_before_sleep_check
            && self.nohz_idle_entered
            && self.nohz_run_idle_balance_done
            && self.local_irq_disabled_for_sleep
            && self.local_irq_save_count_for_sleep != 0
            && self.local_irq_restore_count_for_sleep != 0
            && self.arch_cpu_idle_enter_done
            && self.arch_cpu_idle_exit_done
            && self.rcu_nocb_deferred_wakeup_flushed
            && self.cpu_offline_dead_path_not_taken
            && self.poll_or_cpuidle_path_deferred
            && self.idle_wait_committed
            && self.idle_wait_path_deferred
            && self.need_resched_set_for_schedule
            && self.observed_need_resched
            && self.idle_polling_cleared
            && self.preempt_need_resched_set
            && self.nohz_idle_exited
            && self.polling_clear_mb_before_flush
            && self.smp_call_function_queue_flushed
            && self.idle_schedule_requested
            && self.idle_schedule_returned
            && self.need_resched_drained
            && self.livepatch_state_update_deferred
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

    pub const fn kernel_init_task_switch_handoff_ready(&self) -> bool {
        self.kernel_init_task_switch_handoff_ready
    }

    #[allow(clippy::too_many_arguments)]
    pub fn setup(
        &mut self,
        boot_task: &mut BootTask,
        boot_init_flow: &mut TaskFlow,
        scheduler: &Scheduler,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        kthreadd_ready_gate: &KthreaddReadyGate,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.flow.state() != State::Base
            || boot_task.state() != State::OnCpu
            || boot_task.task_ref() != TaskRef::BOOT
            || boot_task.pid() != 0
            || !boot_task
                .task()
                .active_flow()
                .same_identity(boot_init_flow.flow_ref())
            || !boot_init_flow.active()
            || boot_init_flow.state() != State::Ready
            || scheduler.state() != State::Online
            || scheduler.boot_cpu_owned_scheduler_view(cpu_group).is_none()
            || kernel_init_task.state() != State::Online
            || !kernel_init_task.waiting_for_kthreadd_done()
            || kthreadd_task.state() != State::Online
            || kthreadd_ready_gate.state() != State::Online
            || scheduler.schedule_passes() != 0
            || cpu_group.state() != State::Ready
            || cpu_group.boot_cpu_state() != State::Online
        {
            return self.failed_setup();
        }

        self.first_schedule_committed = false;
        self.idle_entry_prepared = false;
        self.cpu_startup_entry_ready = false;
        self.idle_loop_entered = false;
        self.idle_cycle_committed = false;
        self.idle_cycle_started = false;
        self.need_resched_clear_before_wait = false;
        self.observed_no_need_resched = false;
        self.idle_polling_set = false;
        self.idle_polling_rmb_before_sleep_check = false;
        self.nohz_idle_entered = false;
        self.nohz_run_idle_balance_done = false;
        self.local_irq_disabled_for_sleep = false;
        self.local_irq_save_count_for_sleep = 0;
        self.local_irq_restore_count_for_sleep = 0;
        self.arch_cpu_idle_enter_done = false;
        self.arch_cpu_idle_exit_done = false;
        self.rcu_nocb_deferred_wakeup_flushed = false;
        self.cpu_offline_dead_path_not_taken = false;
        self.poll_or_cpuidle_path_deferred = false;
        self.idle_wait_committed = false;
        self.idle_wait_path_deferred = false;
        self.need_resched_set_for_schedule = false;
        self.observed_need_resched = false;
        self.idle_polling_cleared = false;
        self.preempt_need_resched_set = false;
        self.nohz_idle_exited = false;
        self.polling_clear_mb_before_flush = false;
        self.smp_call_function_queue_flushed = false;
        self.idle_schedule_requested = false;
        self.idle_schedule_returned = false;
        self.need_resched_drained = false;
        self.livepatch_state_update_deferred = false;
        self.idle_loop_continues = false;
        self.boot_init_handoff_complete = false;
        self.boot_cpu_hotplug_online = false;
        self.secondary_cpus_not_started = true;
        self.kernel_init_task_switch_handoff_ready = true;
        self.flow
            .bind(boot_task.task_mut(), boot_init_flow.flow_ref())?;
        self.flow
            .setup_successor(boot_task.task(), Some(Checkpoint::BootIdleFlowReady))?;
        boot_task
            .task_mut()
            .commit_ready_successor_handoff(boot_init_flow, &mut self.flow)
    }

    #[cfg_attr(app_smoke, allow(dead_code))]
    pub const fn flow_ref(&self) -> TaskFlowRef {
        self.flow.flow_ref()
    }

    #[cfg_attr(app_smoke, allow(dead_code))]
    pub const fn owner(&self) -> TaskRef {
        self.flow.owner()
    }

    #[cfg_attr(app_smoke, allow(dead_code))]
    pub const fn active(&self) -> bool {
        self.flow.active()
    }

    pub(crate) fn core_mut(&mut self) -> &mut TaskFlow {
        &mut self.flow
    }

    pub(crate) const fn core(&self) -> &TaskFlow {
        &self.flow
    }

    pub fn prepare_idle_entry(
        &mut self,
        boot_task: &BootTask,
        scheduler: &Scheduler,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        if self.flow.state() != State::Ready
            || !crate::objects::task_flow::task_flow_execution_guard_satisfied(
                &self.flow,
                boot_task.task(),
            )
            || scheduler.state() != State::Online
            || scheduler.schedule_passes() == 0
            || scheduler.kernel_init_stack_switch_started_count() != 1
            || scheduler.kernel_init_stack_switch_returned_count() != 1
            || scheduler.boot_cpu_owned_scheduler_view(cpu_group).is_none()
            || cpu_group.state() != State::Ready
            || cpu_group.boot_cpu_state() != State::Online
        {
            return self.failed_ready_action();
        }

        self.first_schedule_committed = true;
        self.idle_entry_prepared = true;
        self.cpu_startup_entry_ready = true;
        self.boot_init_handoff_complete = true;
        self.boot_cpu_hotplug_online = true;
        self.need_resched_clear_before_wait = true;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_idle_loop(
        &mut self,
        scheduler: &mut Scheduler,
        current_task: CurrentTask,
        current_cpu: CurrentCpu,
        kernel_init_task: &mut KernelInitTask,
        kernel_init_flow: &mut KernelInitFlow,
        user_app_flow: &UserAppFlow,
        kthreadd_task: &mut KthreaddTask,
        kthreadd_flow: &mut KthreaddFlow,
        user_task_set: &mut UserTaskSet,
        boot_task: &BootTask,
        local_interrupt: &mut LocalInterruptControl,
    ) -> EventResult {
        if self.flow.state() != State::Ready
            || !self.idle_entry_prepared
            || scheduler.state() != State::Online
            || !crate::objects::task_flow::task_flow_execution_guard_satisfied(
                &self.flow,
                boot_task.task(),
            )
        {
            return self.failed_ready_action();
        }

        self.do_idle_cycle(
            scheduler,
            current_task,
            current_cpu,
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            user_task_set,
            boot_task,
            local_interrupt,
        )?;
        self.idle_loop_entered = true;
        self.idle_loop_continues = true;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn do_idle_cycle(
        &mut self,
        scheduler: &mut Scheduler,
        current_task: CurrentTask,
        current_cpu: CurrentCpu,
        kernel_init_task: &mut KernelInitTask,
        kernel_init_flow: &mut KernelInitFlow,
        user_app_flow: &UserAppFlow,
        kthreadd_task: &mut KthreaddTask,
        kthreadd_flow: &mut KthreaddFlow,
        user_task_set: &mut UserTaskSet,
        boot_task: &BootTask,
        local_interrupt: &mut LocalInterruptControl,
    ) -> EventResult {
        if self.flow.state() != State::Ready
            || !self.idle_entry_prepared
            || scheduler.state() != State::Online
            || !crate::objects::task_flow::task_flow_execution_guard_satisfied(
                &self.flow,
                boot_task.task(),
            )
        {
            return self.failed_ready_action();
        }

        self.nohz_run_idle_balance_done = true;
        self.wait_while_no_need_resched(boot_task, local_interrupt)?;
        self.observe_need_resched(boot_task)?;
        self.schedule_if_need_resched(
            scheduler,
            current_task,
            current_cpu,
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            user_task_set,
            boot_task,
            local_interrupt,
        )?;
        self.idle_cycle_committed = true;
        self.secondary_cpus_not_started = true;
        self.kernel_init_task_switch_handoff_ready = true;
        Ok(())
    }

    fn wait_while_no_need_resched(
        &mut self,
        boot_task: &BootTask,
        local_interrupt: &mut LocalInterruptControl,
    ) -> EventResult {
        if self.flow.state() != State::Ready
            || !self.idle_entry_prepared
            || local_interrupt.state() != State::Ready
            || !crate::objects::task_flow::task_flow_execution_guard_satisfied(
                &self.flow,
                boot_task.task(),
            )
        {
            return self.failed_ready_action();
        }

        self.idle_cycle_started = true;
        self.need_resched_clear_before_wait = true;
        self.observed_no_need_resched = true;
        self.idle_polling_set = true;
        self.idle_polling_rmb_before_sleep_check = true;
        self.nohz_idle_entered = true;
        let saves_before = local_interrupt.saved_and_disabled_count();
        local_interrupt.save_and_disable()?;
        let guarded_result: EventResult = {
            self.local_irq_disabled_for_sleep = local_interrupt.disabled()
                && local_interrupt.saved_and_disabled_count() == saves_before.wrapping_add(1);
            self.local_irq_save_count_for_sleep = local_interrupt.saved_and_disabled_count();
            self.cpu_offline_dead_path_not_taken = true;
            self.arch_cpu_idle_enter_done = true;
            self.rcu_nocb_deferred_wakeup_flushed = true;
            self.poll_or_cpuidle_path_deferred = true;
            self.arch_cpu_idle_exit_done = true;
            Ok(())
        };
        let restore_result = local_interrupt.restore();
        if guarded_result.is_ok() && restore_result.is_ok() {
            self.local_irq_restore_count_for_sleep = local_interrupt.restored_count();
        }
        guarded_result.and(restore_result)?;
        self.idle_wait_committed = true;
        self.idle_wait_path_deferred = true;
        Ok(())
    }

    fn observe_need_resched(&mut self, boot_task: &BootTask) -> EventResult {
        if self.flow.state() != State::Ready
            || !self.idle_entry_prepared
            || !self.idle_wait_committed
            || !self.need_resched_clear_before_wait
            || !crate::objects::task_flow::task_flow_execution_guard_satisfied(
                &self.flow,
                boot_task.task(),
            )
        {
            return self.failed_ready_action();
        }

        self.need_resched_set_for_schedule = true;
        self.observed_need_resched = true;
        self.idle_polling_cleared = true;
        self.preempt_need_resched_set = true;
        self.nohz_idle_exited = true;
        self.polling_clear_mb_before_flush = true;
        self.smp_call_function_queue_flushed = true;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn schedule_if_need_resched(
        &mut self,
        scheduler: &mut Scheduler,
        current_task: CurrentTask,
        current_cpu: CurrentCpu,
        kernel_init_task: &mut KernelInitTask,
        kernel_init_flow: &mut KernelInitFlow,
        user_app_flow: &UserAppFlow,
        kthreadd_task: &mut KthreaddTask,
        kthreadd_flow: &mut KthreaddFlow,
        user_task_set: &mut UserTaskSet,
        boot_task: &BootTask,
        local_interrupt: &mut LocalInterruptControl,
    ) -> EventResult {
        if self.flow.state() != State::Ready
            || !self.idle_entry_prepared
            || !self.need_resched_set_for_schedule
            || !self.observed_need_resched
            || scheduler.state() != State::Online
            || scheduler.schedule_passes() == 0
            || !current_task.task_ref().same_identity(TaskRef::BOOT)
            || !crate::objects::task_flow::task_flow_execution_guard_satisfied(
                &self.flow,
                boot_task.task(),
            )
        {
            return self.failed_ready_action();
        }

        self.idle_schedule_requested = true;
        scheduler.schedule_idle(
            current_task,
            current_cpu,
            kernel_init_task,
            kernel_init_flow,
            user_app_flow,
            kthreadd_task,
            kthreadd_flow,
            self,
            user_task_set,
            local_interrupt,
        )?;
        self.idle_schedule_returned = true;
        self.need_resched_drained = true;
        self.livepatch_state_update_deferred = true;
        self.idle_loop_continues = true;
        Ok(())
    }

    fn failed_setup(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.flow.state(),
            State::Base,
            State::Ready,
        )
    }

    fn failed_ready_action(&self) -> EventResult {
        failed_condition(
            LifecycleEvent::Setup,
            self.flow.state(),
            State::Ready,
            State::Ready,
        )
    }
}
