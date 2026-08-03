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
    if ctx.kernel_init_task.flow_state() != State::Online || !mainline_ready(ctx) {
        crate::phases::shutdown_on_error(
            flow_failure(LifecycleEvent::Dispatch, State::Online, State::Online),
            "arceos_ex kernel init flow preset start failed\n",
        );
    }
    pre_smp_init::preset(crate::context::context())
}

pub fn preset_after_pre_smp_init() -> ! {
    require_flow_continuation(
        pre_smp_init::is_online(),
        LifecycleEvent::Preset,
        State::Online,
        State::Online,
        "arceos_ex kernel init flow after pre-smp init failed\n",
    );
    smp_bringup::preset(crate::context::context())
}

pub fn preset_after_smp_bringup() -> ! {
    let ctx = crate::context::context_ref();
    let result = if pre_smp_init::is_online() && smp_bringup::is_online() && mainline_ready(ctx) {
        Ok(())
    } else {
        flow_failure(LifecycleEvent::Dispatch, State::Online, State::Online)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex kernel init flow preset failed\n");
    runtime_core::preset(crate::context::context())
}

pub fn setup_after_runtime_core() -> ! {
    require_flow_continuation(
        runtime_core::is_online(),
        LifecycleEvent::Setup,
        State::Online,
        State::Online,
        "arceos_ex kernel init flow after runtime core failed\n",
    );
    initcall::preset(crate::context::context())
}

pub fn setup_after_initcall() -> ! {
    require_flow_continuation(
        initcall::is_online(),
        LifecycleEvent::Setup,
        State::Online,
        State::Online,
        "arceos_ex kernel init flow after initcall failed\n",
    );
    rootfs::preset(crate::context::context())
}

pub fn setup_after_rootfs() -> ! {
    require_flow_continuation(
        rootfs::is_online(),
        LifecycleEvent::Setup,
        State::Online,
        State::Online,
        "arceos_ex kernel init flow after rootfs failed\n",
    );
    finalize::preset(crate::context::context())
}

pub fn setup_after_finalize() -> ! {
    require_flow_continuation(
        finalize::is_online(),
        LifecycleEvent::Setup,
        State::Online,
        State::Online,
        "arceos_ex kernel init flow after finalize failed\n",
    );
    crate::phases::payload::prepare::preset()
}

pub fn setup_after_payload_prepare() -> ! {
    let ctx = crate::context::context_ref();
    let result = if setup_children_online() && mainline_ready(ctx) {
        Ok(())
    } else {
        flow_failure(LifecycleEvent::Dispatch, State::Online, State::Online)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex kernel init flow setup failed\n");
    crate::phases::payload::handoff_prepare::preset()
}

pub fn enable_after_payload_handoff_prepare() -> ! {
    let ctx = crate::context::context_ref();
    let result = if crate::phases::payload::handoff_prepare::is_online() && mainline_ready(ctx) {
        Ok(())
    } else {
        flow_failure(LifecycleEvent::Dispatch, State::Online, State::Online)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex kernel init flow enable failed\n");
    crate::phases::shutdown_on_error(
        crate::systems::kernel::commit_online_after_application_environment_ready(),
        "arceos_ex kernel online commit failed\n",
    );
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
    let result =
        if child_online && ctx.kernel_init_task.flow_state() == expected && mainline_ready(ctx) {
            Ok(())
        } else {
            flow_failure(event, expected, target)
        };
    crate::phases::shutdown_on_error(result, message)
}

fn mainline_ready(ctx: &crate::context::Context) -> bool {
    crate::systems::kernel::enable_in_progress()
        && crate::flows::boot_init_flow::is_online()
        && ctx.kernel_init_task.state() == State::OnCpu
        && ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref.same_identity(ctx.kernel_init_task.task_ref()))
        && ctx.scheduler().kernel_init_stack_switch_started_count() == 1
        && ctx.kernel_init_task.entry_started_count() == 1
        && ctx.kernel_init_task.entry_stack_verified()
        && ctx.kernel_init_task.current_stack_pointer_in_range()
}

fn commit_payload_handoff() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        ctx.kernel_init_task.require_payload_handoff_action(),
        "arceos_ex kernel init payload handoff guard failed\n",
    );
    crate::phases::shutdown_on_error(
        crate::apps::commit_selected_payload(ctx),
        "arceos_ex selected payload handoff commit failed\n",
    );
    crate::phases::shutdown_on_error(
        ctx.selected_payload_handoff.commit(),
        "arceos_ex selected payload handoff fact commit failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::KernelInitFlowPayloadHandoffCommitted);
    crate::checkpoint::dispatch_after_trace(Checkpoint::KernelInitFlowPayloadHandoffCommitted, ctx);
    crate::apps::enter_selected_payload(ctx)
}

fn flow_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::context::context_ref().kernel_init_task.flow_state(),
        expected,
        target,
    )
}
