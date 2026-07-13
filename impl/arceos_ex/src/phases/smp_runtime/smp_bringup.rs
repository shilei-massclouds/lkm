use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        smp_bringup::smp_bringup_runtime_ready,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static SMP_BRINGUP_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        preset_start(),
        "arceos_ex smp bringup preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::SmpBringupPhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| adopt_prepared(ctx)),
        "arceos_ex smp bringup preset failed\n",
    );
    setup(ctx)
}

fn preset_start() -> EventResult {
    let state = crate::phases::state::load(&SMP_BRINGUP_PHASE_STATE);
    if state != State::Base
        || !crate::phases::up_multitask::is_online()
        || !crate::phases::smp_runtime::pre_smp_init::is_online()
    {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    ctx.secondary_idle_tasks.preset(
        &ctx.pre_smp_boundary,
        &ctx.cpu_group,
        &ctx.scheduler,
        &ctx.per_cpu_storage,
    )?;
    ctx.smpboot_threads_lock.preset_static()?;
    ctx.smpboot_threads_lock.setup()?;
    ctx.cpu_hotplug_sync.preset(
        &ctx.cpu_group,
        &ctx.boot_idle_runtime,
        &ctx.kthreadd_task,
        &mut ctx.cpu_hotplug_lock,
        &mut ctx.smpboot_threads_lock,
    )?;
    ctx.cpu_add_remove_lock.preset_static()?;
    ctx.cpu_add_remove_lock.setup()?;
    ctx.cpu_running_wait_lock
        .setup_with_checkpoint(Checkpoint::CpuRunningWaitLockReady)?;
    ctx.done_up_wait_lock
        .setup_with_checkpoint(Checkpoint::CpuDoneUpWaitLockReady)?;
    ctx.cpu_start_provider.setup(
        &ctx.cpu_group,
        &ctx.secondary_idle_tasks,
        &ctx.cpu_hotplug_sync,
        &ctx.sbi,
        &ctx.sbi_ipi,
        &ctx.kernel_image,
        &ctx.static_objects,
        &ctx.lds,
        &mut ctx.cpu_add_remove_lock,
        &mut ctx.cpu_hotplug_lock,
    )?;
    ctx.ap_entry_prelude_phase.setup(
        &ctx.cpu_start_provider,
        &ctx.cpu_group,
        &ctx.secondary_idle_tasks,
        &ctx.vm,
        &ctx.event_stream,
        &ctx.exception_stream,
    )?;
    ctx.ap_smp_callin_phase.setup(
        &ctx.ap_entry_prelude_phase,
        &ctx.cpu_group,
        &ctx.cpu_hotplug_sync,
        &ctx.sbi_ipi,
        &ctx.init_mm,
    )?;
    ctx.ap_online_idle_phase.setup(
        &ctx.ap_smp_callin_phase,
        &ctx.cpu_group,
        &ctx.cpu_hotplug_sync,
    )?;
    ctx.secondary_cpu_startup_ack.setup(
        &ctx.cpu_start_provider,
        &ctx.ap_smp_callin_phase,
        &mut ctx.cpu_hotplug_sync,
        &mut ctx.cpu_running_wait_lock,
        &mut ctx.boot_cpu_local_interrupt,
        &mut ctx.scheduler,
    )?;
    ctx.secondary_cpu_online_ack.setup(
        &ctx.secondary_cpu_startup_ack,
        &ctx.ap_online_idle_phase,
        &ctx.ap_smp_callin_phase,
        &mut ctx.cpu_hotplug_sync,
        &mut ctx.cpu_group,
        &mut ctx.secondary_cpus,
        &ctx.sbi_ipi,
        &mut ctx.done_up_wait_lock,
        &mut ctx.boot_cpu_local_interrupt,
        &mut ctx.scheduler,
    )?;
    ctx.smp_bringup_boundary
        .setup(&ctx.secondary_cpu_online_ack, &ctx.cpu_group)
}

fn adopt_prepared(ctx: &Context) -> EventResult {
    transition_if_ready(
        ctx,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::SmpBringupPhasePrepared,
    )
}

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        transition_if_ready(
            ctx,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::SmpBringupPhaseReady,
        ),
        "arceos_ex smp bringup setup failed\n",
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
            Checkpoint::SmpBringupPhaseOnline,
        ),
        "arceos_ex smp bringup enable failed\n",
    );
    crate::phases::smp_runtime::setup_after_smp_bringup()
}

fn transition_if_ready(
    ctx: &Context,
    event: LifecycleEvent,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
) -> EventResult {
    let state = crate::phases::state::load(&SMP_BRINGUP_PHASE_STATE);
    if state != expected || !smp_bringup_phase_ready(ctx) {
        return failed_condition(event, state, expected, target);
    }

    crate::phases::state::mark_checked(
        &SMP_BRINGUP_PHASE_STATE,
        event,
        expected,
        target,
        checkpoint,
    )
}

pub fn is_online() -> bool {
    crate::phases::state::load(&SMP_BRINGUP_PHASE_STATE) == State::Online
}

fn smp_bringup_phase_ready(ctx: &Context) -> bool {
    smp_bringup_runtime_ready(
        &ctx.kernel_init_task,
        &ctx.cpu_group,
        &ctx.secondary_idle_tasks,
        &ctx.smpboot_threads_lock,
        &ctx.cpu_hotplug_sync,
        &ctx.cpu_add_remove_lock,
        &ctx.cpu_hotplug_lock,
        &ctx.cpu_running_wait_lock,
        &ctx.done_up_wait_lock,
        &ctx.cpu_start_provider,
        &ctx.ap_entry_prelude_phase,
        &ctx.ap_smp_callin_phase,
        &ctx.ap_online_idle_phase,
        &ctx.secondary_cpu_startup_ack,
        &ctx.secondary_cpu_online_ack,
        &ctx.smp_bringup_boundary,
    )
}
