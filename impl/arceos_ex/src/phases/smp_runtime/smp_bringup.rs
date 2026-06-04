use crate::{
    context::Context,
    objects::{
        smp_bringup::smp_bringup_runtime_ready,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static SMP_BRINGUP_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::SmpBringupPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex smp bringup event failed\n",
    );
    crate::phases::smp_runtime::setup_after_children()
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    if !crate::phases::up_multitask::is_ready()
        || !crate::phases::up_multitask::pre_smp_init::is_ready()
    {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    ctx.secondary_idle_tasks.preset(
        &ctx.pre_smp_boundary,
        &ctx.cpu_group,
        &ctx.scheduler,
        &ctx.per_cpu_storage,
    )?;
    ctx.cpu_hotplug_sync
        .preset(&ctx.cpu_group, &ctx.boot_idle_runtime, &ctx.kthreadd_task)?;
    ctx.cpu_start_provider.setup(
        &ctx.cpu_group,
        &ctx.secondary_idle_tasks,
        &ctx.cpu_hotplug_sync,
        &ctx.sbi_ipi,
    )?;
    ctx.secondary_cpu_startup_ack
        .setup(&ctx.cpu_start_provider, &mut ctx.cpu_hotplug_sync)?;
    ctx.secondary_cpu_online_ack.setup(
        &ctx.secondary_cpu_startup_ack,
        &mut ctx.cpu_hotplug_sync,
        &mut ctx.cpu_group,
        &ctx.sbi_ipi,
    )?;
    ctx.smp_bringup_boundary
        .setup(&ctx.secondary_cpu_online_ack, &ctx.cpu_group)
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !smp_bringup_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&SMP_BRINGUP_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &SMP_BRINGUP_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::SmpBringupPhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&SMP_BRINGUP_PHASE_STATE) == State::Ready
}

fn smp_bringup_phase_ready(ctx: &Context) -> bool {
    smp_bringup_runtime_ready(
        &ctx.kernel_init_task,
        &ctx.cpu_group,
        &ctx.secondary_idle_tasks,
        &ctx.cpu_hotplug_sync,
        &ctx.cpu_start_provider,
        &ctx.secondary_cpu_startup_ack,
        &ctx.secondary_cpu_online_ack,
        &ctx.smp_bringup_boundary,
    )
}
