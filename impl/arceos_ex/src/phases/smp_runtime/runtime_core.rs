use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        runtime_core::runtime_core_ready,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static RUNTIME_CORE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex runtime core event failed\n",
    );
    crate::phases::smp_runtime::initcall::setup(ctx)
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    if !crate::phases::smp_runtime::smp_bringup::is_ready() {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    ctx.scheduler
        .enable_smp(&mut ctx.kernel_init_task, &ctx.cpu_group)?;
    crate::checkpoint::checkpoint(Checkpoint::RuntimeCorePhaseStarted);
    ctx.workqueue
        .setup_topology(&ctx.scheduler, &ctx.cpu_group)?;
    ctx.async_core_deferred.setup(&ctx.workqueue)?;
    ctx.padata_core_deferred.setup(
        &ctx.async_core_deferred,
        &ctx.cpu_hotplug_state,
        &ctx.cpu_group,
    )?;
    ctx.page_allocator
        .setup_late(&ctx.workqueue, &ctx.cpu_group)?;
    ctx.runtime_core_boundary.setup(
        &ctx.scheduler,
        &ctx.workqueue,
        &ctx.async_core_deferred,
        &ctx.padata_core_deferred,
        &ctx.page_allocator,
    )
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !runtime_core_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&RUNTIME_CORE_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &RUNTIME_CORE_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::RuntimeCorePhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&RUNTIME_CORE_PHASE_STATE) == State::Ready
}

fn runtime_core_phase_ready(ctx: &Context) -> bool {
    runtime_core_ready(
        &ctx.scheduler,
        &ctx.workqueue,
        &ctx.async_core_deferred,
        &ctx.padata_core_deferred,
        &ctx.page_allocator,
        &ctx.runtime_core_boundary,
    ) && !ctx.kernel_init_task.pinned_to_boot_cpu()
        && !ctx.kernel_init_task.pf_no_setaffinity()
}
