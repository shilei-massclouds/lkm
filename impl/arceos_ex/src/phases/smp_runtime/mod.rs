pub mod finalize;
pub mod initcall;
pub mod pre_smp_init;
pub mod rootfs;
pub mod runtime_core;
pub mod smp_bringup;

use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::state::{failed_condition, EventResult, LifecycleEvent, State},
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static SMP_RUNTIME_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup() -> ! {
    let ctx = crate::context::context();
    crate::phases::shutdown_on_error(
        require_mainline(ctx, LifecycleEvent::Preset, State::Base, State::Prepared),
        "arceos_ex smp runtime preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::SmpRuntimePhaseStarted);
    pre_smp_init::preset(ctx)
}

pub fn preset_after_pre_smp_init() -> ! {
    let ctx = crate::context::context();
    let result = if pre_smp_init::is_online()
        && mainline_ready(ctx)
        && crate::phases::state::load(&SMP_RUNTIME_PHASE_STATE) == State::Base
    {
        crate::phases::state::mark_checked(
            &SMP_RUNTIME_PHASE_STATE,
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::SmpRuntimePhasePrepared,
        )
    } else {
        phase_failure(LifecycleEvent::Preset, State::Base, State::Prepared)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex smp runtime preset failed\n");
    smp_bringup::preset(ctx)
}

pub fn setup_after_smp_bringup() -> ! {
    let ctx = crate::context::context();
    let result = if pre_smp_init::is_online()
        && smp_bringup::is_online()
        && mainline_ready(ctx)
        && crate::phases::state::load(&SMP_RUNTIME_PHASE_STATE) == State::Prepared
    {
        crate::phases::state::mark_checked(
            &SMP_RUNTIME_PHASE_STATE,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::SmpRuntimePhaseReady,
        )
    } else {
        phase_failure(LifecycleEvent::Setup, State::Prepared, State::Ready)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex smp runtime setup failed\n");
    runtime_core::preset(ctx)
}

pub fn enable_after_runtime_core() -> ! {
    require_enable_continuation(runtime_core::is_online(), "runtime core");
    initcall::preset(crate::context::context())
}

pub fn enable_after_initcall() -> ! {
    require_enable_continuation(initcall::is_online(), "initcall");
    rootfs::preset(crate::context::context())
}

pub fn enable_after_rootfs() -> ! {
    require_enable_continuation(rootfs::is_online(), "rootfs");
    finalize::preset(crate::context::context())
}

pub fn enable_after_finalize() -> ! {
    let ctx = crate::context::context_ref();
    let result = if children_online()
        && mainline_ready(ctx)
        && crate::phases::state::load(&SMP_RUNTIME_PHASE_STATE) == State::Ready
    {
        crate::phases::state::mark_checked(
            &SMP_RUNTIME_PHASE_STATE,
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::SmpRuntimePhaseOnline,
        )
    } else {
        phase_failure(LifecycleEvent::Enable, State::Ready, State::Online)
    };
    crate::phases::shutdown_on_error(result, "arceos_ex smp runtime enable failed\n");
    crate::systems::kernel::enable_after_smp_runtime()
}

fn require_enable_continuation(child_online: bool, child: &str) {
    let ctx = crate::context::context_ref();
    let result = if child_online
        && mainline_ready(ctx)
        && crate::phases::state::load(&SMP_RUNTIME_PHASE_STATE) == State::Ready
    {
        Ok(())
    } else {
        phase_failure(LifecycleEvent::Enable, State::Ready, State::Online)
    };
    let message = match child {
        "runtime core" => "arceos_ex smp runtime after runtime core failed\n",
        "initcall" => "arceos_ex smp runtime after initcall failed\n",
        _ => "arceos_ex smp runtime after rootfs failed\n",
    };
    crate::phases::shutdown_on_error(result, message)
}

pub fn is_online() -> bool {
    crate::phases::state::load(&SMP_RUNTIME_PHASE_STATE) == State::Online && children_online()
}

fn children_online() -> bool {
    pre_smp_init::is_online()
        && smp_bringup::is_online()
        && runtime_core::is_online()
        && initcall::is_online()
        && rootfs::is_online()
        && finalize::is_online()
}

fn require_mainline(
    ctx: &Context,
    event: LifecycleEvent,
    expected: State,
    target: State,
) -> EventResult {
    let actual = crate::phases::state::load(&SMP_RUNTIME_PHASE_STATE);
    if actual != expected || !crate::phases::up_multitask::is_online() || !mainline_ready(ctx) {
        return failed_condition(event, actual, expected, target);
    }
    Ok(())
}

fn mainline_ready(ctx: &Context) -> bool {
    ctx.kernel_init_task.state() == State::Online
        && ctx.boot_cpu_current_task.current_is_kernel_init()
        && ctx.scheduler.kernel_init_stack_switch_started_count() == 1
        && ctx.kernel_init_task.entry_started_count() == 1
        && ctx.kernel_init_task.entry_stack_verified()
        && ctx.kernel_init_task.current_stack_pointer_in_range()
}

fn phase_failure(event: LifecycleEvent, expected: State, target: State) -> EventResult {
    failed_condition(
        event,
        crate::phases::state::load(&SMP_RUNTIME_PHASE_STATE),
        expected,
        target,
    )
}
