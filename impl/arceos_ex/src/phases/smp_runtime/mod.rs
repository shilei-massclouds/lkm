//! KernelInitFlow direct execution-phase namespace.

pub mod ap_entry_prelude;
pub mod ap_online_idle;
pub mod ap_smp_callin;
pub mod finalize;
pub mod initcall;
pub mod pre_smp_init;
pub mod rootfs;
pub mod runtime_core;
pub mod smp_bringup;

use crate::{
    checkpoint::Checkpoint,
    objects::state::{EventResult, LifecycleEvent, State, failed_condition},
};

/// Entered only from `kernel_init_entry()` after its actual stack check.
pub fn start_kernel_init_flow() -> ! {
    let ctx = crate::context::context_ref();
    if ctx.kernel_init_flow.state() != State::Base
        || !ctx.kernel_init_flow.initial_start_accepted()
        || !mainline_ready(ctx)
    {
        crate::phases::shutdown_on_error(
            flow_failure(LifecycleEvent::Preset, State::Base, State::Prepared),
            "arceos_ex kernel init flow preset start failed\n",
        );
    }
    pre_smp_init::preset(crate::context::context())
}

pub fn preset_after_pre_smp_init() -> ! {
    require_flow_continuation(
        pre_smp_init::is_online(),
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        "arceos_ex kernel init flow after pre-smp init failed\n",
    );
    smp_bringup::preset(crate::context::context())
}

pub fn preset_after_smp_bringup() -> ! {
    let ctx = crate::context::context();
    let result = if pre_smp_init::is_online() && smp_bringup::is_online() && mainline_ready(ctx) {
        ctx.kernel_init_flow
            .commit_preset_after_children(&ctx.kernel_init_task)
    } else {
        flow_failure(LifecycleEvent::Preset, State::Base, State::Prepared)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex kernel init flow preset failed\n");
    runtime_core::preset(ctx)
}

pub fn setup_after_runtime_core() -> ! {
    require_flow_continuation(
        runtime_core::is_online(),
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        "arceos_ex kernel init flow after runtime core failed\n",
    );
    initcall::preset(crate::context::context())
}

pub fn setup_after_initcall() -> ! {
    require_flow_continuation(
        initcall::is_online(),
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        "arceos_ex kernel init flow after initcall failed\n",
    );
    rootfs::preset(crate::context::context())
}

pub fn setup_after_rootfs() -> ! {
    require_flow_continuation(
        rootfs::is_online(),
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        "arceos_ex kernel init flow after rootfs failed\n",
    );
    finalize::preset(crate::context::context())
}

pub fn setup_after_finalize() -> ! {
    require_flow_continuation(
        finalize::is_online(),
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        "arceos_ex kernel init flow after finalize failed\n",
    );
    crate::phases::payload::prepare::preset()
}

pub fn setup_after_payload_prepare() -> ! {
    let ctx = crate::context::context();
    let result = if setup_children_online() && mainline_ready(ctx) {
        ctx.kernel_init_flow
            .commit_setup_after_children(&ctx.kernel_init_task)
    } else {
        flow_failure(LifecycleEvent::Setup, State::Prepared, State::Ready)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex kernel init flow setup failed\n");
    crate::phases::payload::handoff_prepare::preset()
}

pub fn enable_after_payload_handoff_prepare() -> ! {
    let ctx = crate::context::context();
    let result = if crate::phases::payload::handoff_prepare::is_online() && mainline_ready(ctx) {
        ctx.kernel_init_flow
            .commit_enable_after_children(&mut ctx.kernel_init_task)
    } else {
        flow_failure(LifecycleEvent::Enable, State::Ready, State::Online)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex kernel init flow enable failed\n");
    commit_payload_handoff()
}

fn setup_children_online() -> bool {
    runtime_core::is_online()
        && initcall::is_online()
        && rootfs::is_online()
        && finalize::is_online()
        && crate::phases::payload::prepare::is_online()
}

fn require_flow_continuation(
    child_online: bool,
    event: LifecycleEvent,
    expected: State,
    target: State,
    message: &'static str,
) {
    let ctx = crate::context::context_ref();
    let result = if child_online && ctx.kernel_init_flow.state() == expected && mainline_ready(ctx)
    {
        Ok(())
    } else {
        flow_failure(event, expected, target)
    };
    crate::phases::shutdown_on_error(result, message)
}

fn mainline_ready(ctx: &crate::context::Context) -> bool {
    crate::systems::kernel::is_online()
        && crate::flows::boot_init_flow::is_online()
        && ctx.kernel_init_task.state() == State::OnCpu
        && ctx.boot_cpu_current_task.current_is_kernel_init()
        && ctx.boot_cpu_current_task.current() == ctx.kernel_init_task.task_ref()
        && ctx.scheduler.kernel_init_stack_switch_started_count() == 1
        && ctx.kernel_init_task.entry_started_count() == 1
        && ctx.kernel_init_task.entry_stack_verified()
        && ctx.kernel_init_task.current_stack_pointer_in_range()
}

fn commit_payload_handoff() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        ctx.kernel_init_flow
            .require_payload_handoff_action(&ctx.kernel_init_task),
        "arceos_ex kernel init payload handoff guard failed\n",
    );
    crate::phases::shutdown_on_error(
        crate::apps::commit_selected_payload(ctx),
        "arceos_ex selected payload handoff commit failed\n",
    );
    ctx.kernel_init_flow.mark_payload_handoff_committed();
    crate::checkpoint::checkpoint(Checkpoint::KernelInitFlowPayloadHandoffCommitted);
    crate::checkpoint::dispatch_after_trace(Checkpoint::KernelInitFlowPayloadHandoffCommitted, ctx);
    crate::apps::enter_selected_payload(ctx)
}

fn flow_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::context::context_ref().kernel_init_flow.state(),
        expected,
        target,
    )
}
