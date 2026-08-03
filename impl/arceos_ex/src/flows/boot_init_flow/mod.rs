mod enable;
mod idle;
mod idle_entry;
mod preset;
mod rest_init;
mod schedule_handoff;
mod setup;
pub(crate) use idle::IdleRuntime;

#[allow(unused_imports)]
pub use enable::{boot_task_restored, enable, enable_after_boot_init_schedule_handoff};
pub use preset::adopt_head_preset_entry;
#[allow(unused_imports)]
pub use setup::{
    setup_after_boot_init_rest_init, setup_after_core_prepare, setup_after_irq_open_prepare,
    setup_after_irq_time_init, setup_after_local_irq_enable, setup_after_mm_core_init,
    setup_after_process_prepare, setup_after_sched_init, start_kernel,
};

#[cfg(app_smoke)]
pub(crate) use preset::{
    boot_task_entry_bind_count, boot_task_entry_bind_diagnostic,
    boot_task_entry_preemption_initialized,
};

use crate::objects::{
    state::{EventResult, LifecycleEvent, State, failed_condition},
    task::TaskRef,
};

pub fn is_online() -> bool {
    crate::objects::boot_task::BootTask::canonical_task().flow_state() == State::Online
        && setup_leaves_online()
        && rest_init::is_online()
        && schedule_handoff::is_online()
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn rest_init_is_online() -> bool {
    rest_init::is_online()
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn schedule_handoff_is_online() -> bool {
    schedule_handoff::is_online()
}

#[cfg_attr(not(app_smoke), allow(dead_code))]
pub(crate) fn idle_entry_is_online() -> bool {
    idle::entry_is_online()
}

/// Reports the post-handoff boundary observed while KernelInitTask owns the CPU.
pub fn dispatch_ready() -> bool {
    let ctx = crate::context::context_ref();
    is_online()
        && ctx.scheduler().schedule_passes() != 0
        && ctx.scheduler().current_runqueue_resolve_passes() != 0
        && ctx.scheduler().pick_next_task_passes() != 0
        && ctx.scheduler().switch_to_passes() != 0
        && ctx.scheduler().scheduler_finish_task_switch_count() != 0
        && ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref.same_identity(ctx.kernel_init_task.task_ref()))
        && ctx.scheduler().kernel_init_stack_switch_started_count() == 1
}

pub fn is_prepared() -> bool {
    crate::objects::boot_task::BootTask::canonical_task().flow_state() == State::Prepared
        && boot_task_on_cpu_and_canonical()
}

pub(super) fn setup_leaves_online() -> bool {
    crate::phases::boot::core_prepare::is_online()
        && crate::phases::boot::mm_core_init::is_online()
        && crate::phases::boot::sched_init::is_online()
        && crate::phases::interrupt::irq_time_init::is_online()
        && crate::phases::interrupt::local_irq_enable::is_online()
        && crate::phases::interrupt::irq_open_prepare::is_online()
        && crate::phases::interrupt::process_prepare::is_online()
}

pub(super) fn boot_task_on_cpu_and_canonical() -> bool {
    let ctx = crate::context::context_ref();
    ctx.boot_task.state() == State::OnCpu
        && ctx.boot_task.task_ref() == TaskRef::BOOT
        && ctx.boot_task.task().task_ref() == TaskRef::BOOT
        && ctx.boot_task.pid() == 0
}

pub(super) fn require_guarded_state(
    event: LifecycleEvent,
    expected: State,
    target: State,
) -> EventResult {
    let ctx = crate::context::context_ref();
    let actual = ctx.boot_task.task().flow_state();
    if actual != expected
        || !crate::objects::task_flow::task_flow_execution_guard_satisfied(
            ctx.boot_task.task().embedded_flow(),
            ctx.boot_task.task(),
        )
    {
        return failed_condition(event, actual, expected, target);
    }
    Ok(())
}

pub(super) fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::context::context_ref().boot_task.task().flow_state(),
        expected,
        target,
    )
}
