use crate::{
    checkpoint::Checkpoint,
    objects::{
        boot_task::BootTask,
        cpu_group::CpuGroup,
        rest_init::{KernelInitTask, KthreaddReadyGate, KthreaddTask},
        scheduler::Scheduler,
        state::{EventResult, LifecycleEvent, State, failed_condition},
        task::Task,
    },
};

impl super::BootInitFlow {
    pub fn prepare_idle_runtime(
        &mut self,
        boot_task: &BootTask,
        scheduler: &Scheduler,
        kernel_init_task: &KernelInitTask,
        kthreadd_task: &KthreaddTask,
        kthreadd_ready_gate: &KthreaddReadyGate,
        cpu_group: &CpuGroup,
    ) -> EventResult {
        self.idle.setup(
            &self.flow,
            boot_task,
            scheduler,
            kernel_init_task,
            kthreadd_task,
            kthreadd_ready_gate,
            cpu_group,
        )
    }

    pub fn enable(&mut self, owner: &Task, checkpoint: Checkpoint) -> EventResult {
        self.flow.enable(owner, Some(checkpoint))
    }
}

/// BootInitFlow.Enable drives only the reversible schedule-handoff leaf.
pub fn enable() -> ! {
    crate::phases::shutdown_on_error(
        super::require_guarded_state(LifecycleEvent::Enable, State::Ready, State::Online),
        "arceos_ex boot init enable start failed\n",
    );
    if !super::setup_leaves_online() || !super::rest_init::is_online() {
        crate::phases::shutdown_on_error(
            super::phase_failure(LifecycleEvent::Enable, State::Ready, State::Online),
            "arceos_ex boot init enable dependency failed\n",
        );
    }
    super::schedule_handoff::preset(crate::context::context())
}

/// Commits BootInitFlow.Online at the last reversible boundary.
pub fn enable_after_boot_init_schedule_handoff() -> ! {
    let dependencies_ready = super::boot_task_on_cpu_and_canonical()
        && super::rest_init::is_online()
        && super::schedule_handoff::is_online()
        && super::schedule_handoff::precommit_ready();
    let ctx = crate::context::context();
    let result = if dependencies_ready {
        let crate::context::Context {
            boot_init_flow,
            boot_task,
            cpu_group,
            kernel_init_task,
            kthreadd_task,
            kthreadd_ready_gate,
            ..
        } = ctx;
        if let Some(scheduler) = cpu_group.boot_scheduler() {
            boot_init_flow
                .prepare_idle_runtime(
                    boot_task,
                    scheduler,
                    kernel_init_task,
                    kthreadd_task,
                    kthreadd_ready_gate,
                    cpu_group,
                )
                .and_then(|()| {
                    boot_init_flow.enable(boot_task.task(), Checkpoint::BootInitFlowOnline)
                })
        } else {
            failed_condition(
                LifecycleEvent::Enable,
                State::Ready,
                State::Ready,
                State::Online,
            )
        }
    } else {
        failed_condition(
            LifecycleEvent::Enable,
            ctx.boot_init_flow.state(),
            State::Ready,
            State::Online,
        )
    };
    crate::phases::shutdown_on_error(result, "arceos_ex boot init enable failed\n");
    schedule()
}

/// Lowers BootInitFlow's internal idle Schedule signal after the Flow has
/// committed Online. Kernel.Enable remains the enclosing continuation but is
/// not the Scheduler signal sender. The call returns only after a later switch
/// restores the original BootTask.
fn schedule() -> ! {
    if !crate::systems::kernel::enable_in_progress() || !super::is_online() {
        crate::phases::shutdown_on_error(
            super::phase_failure(LifecycleEvent::Enable, State::Online, State::Online),
            "arceos_ex boot init schedule invariant failed\n",
        );
    }

    let ctx = crate::context::context();
    let schedule_result = ctx.schedule_current();
    crate::phases::shutdown_on_error(schedule_result, "arceos_ex first schedule failed\n");
    boot_task_restored()
}

/// Runs only if a later scheduler switch restores the original BootTask stack.
pub fn boot_task_restored() -> ! {
    super::idle::preset_entry(crate::context::context())
}

pub(super) fn rest_init_facts_stable(ctx: &crate::context::Context) -> bool {
    super::rest_init::facts_stable(ctx)
}
