use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        runtime_core::runtime_core_ready,
        state::{EventResult, LifecycleEvent, State, failed_condition},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static RUNTIME_CORE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        preset_start(),
        "arceos_ex runtime core preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::RuntimeCorePhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| adopt_prepared(ctx)),
        "arceos_ex runtime core preset failed\n",
    );
    setup(ctx)
}

fn preset_start() -> EventResult {
    let state = crate::phases::state::load(&RUNTIME_CORE_PHASE_STATE);
    if state != State::Base || !crate::phases::smp_runtime::smp_bringup::is_online() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    ctx.scheduler.enable_smp(
        &mut ctx.kernel_init_task,
        &ctx.kernel_init_flow,
        &ctx.cpu_group,
    )?;
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

fn adopt_prepared(ctx: &Context) -> EventResult {
    transition_if_ready(
        ctx,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::RuntimeCorePhasePrepared,
    )
}

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        transition_if_ready(
            ctx,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::RuntimeCorePhaseReady,
        ),
        "arceos_ex runtime core setup failed\n",
    );
    enable(ctx)
}

fn enable(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        transition_if_ready(
            ctx,
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::RuntimeCorePhaseOnline,
        ),
        "arceos_ex runtime core enable failed\n",
    );
    crate::phases::smp_runtime::setup_after_runtime_core()
}

fn transition_if_ready(
    ctx: &Context,
    event: LifecycleEvent,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
) -> EventResult {
    let state = crate::phases::state::load(&RUNTIME_CORE_PHASE_STATE);
    if state != expected || !runtime_core_phase_ready(ctx) {
        return failed_condition(event, state, expected, target);
    }
    crate::phases::state::mark_checked(
        &RUNTIME_CORE_PHASE_STATE,
        event,
        expected,
        target,
        checkpoint,
    )
}

pub fn is_online() -> bool {
    crate::phases::state::load(&RUNTIME_CORE_PHASE_STATE) == State::Online
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
