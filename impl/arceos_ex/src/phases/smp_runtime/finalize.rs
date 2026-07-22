use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        finalize::finalize_phase_ready,
        state::{EventResult, LifecycleEvent, State, failed_condition},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static FINALIZE_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(preset_start(), "arceos_ex finalize preset start failed\n");
    crate::checkpoint::checkpoint(Checkpoint::FinalizePhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| adopt_prepared(ctx)),
        "arceos_ex finalize preset failed\n",
    );
    setup(ctx)
}

fn preset_start() -> EventResult {
    let state = crate::phases::state::load(&FINALIZE_PHASE_STATE);
    if state != State::Base || !crate::phases::smp_runtime::rootfs::is_online() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    ctx.async_full_sync_deferred
        .setup(&ctx.rootfs_boundary, &ctx.async_core_deferred)?;
    ctx.system_state
        .enter_freeing_initmem(&ctx.async_full_sync_deferred)?;
    ctx.init_memory_cleanup_deferred
        .setup(&ctx.async_full_sync_deferred, &ctx.system_state)?;
    ctx.kernel_mapping_protection_deferred
        .setup(&ctx.init_memory_cleanup_deferred)?;
    ctx.pti_finalize_trimmed
        .setup(&ctx.kernel_mapping_protection_deferred)?;
    ctx.system_state.enable(
        &ctx.init_memory_cleanup_deferred,
        &ctx.kernel_mapping_protection_deferred,
        &ctx.pti_finalize_trimmed,
    )?;
    ctx.numa_default_policy_trimmed
        .setup(&ctx.pti_finalize_trimmed, &ctx.system_state)?;
    ctx.rcu_boot_end.setup(
        &ctx.numa_default_policy_trimmed,
        &ctx.system_state,
        &mut ctx.rcu_core,
    )?;
    ctx.sysctl_args_deferred
        .setup(&ctx.rcu_boot_end, &ctx.saved_command_line)?;
    ctx.finalize_boundary.setup(
        &ctx.async_full_sync_deferred,
        &ctx.init_memory_cleanup_deferred,
        &ctx.kernel_mapping_protection_deferred,
        &ctx.pti_finalize_trimmed,
        &ctx.numa_default_policy_trimmed,
        &ctx.rcu_boot_end,
        &ctx.sysctl_args_deferred,
    )
}

fn adopt_prepared(ctx: &Context) -> EventResult {
    transition_if_ready(
        ctx,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::FinalizePhasePrepared,
    )
}

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        transition_if_ready(
            ctx,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::FinalizePhaseReady,
        ),
        "arceos_ex finalize setup failed\n",
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
            Checkpoint::FinalizePhaseOnline,
        ),
        "arceos_ex finalize enable failed\n",
    );
    crate::phases::smp_runtime::setup_after_finalize()
}

fn transition_if_ready(
    ctx: &Context,
    event: LifecycleEvent,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
) -> EventResult {
    let state = crate::phases::state::load(&FINALIZE_PHASE_STATE);
    if state != expected
        || !finalize_phase_ready(
            &ctx.async_full_sync_deferred,
            &ctx.init_memory_cleanup_deferred,
            &ctx.kernel_mapping_protection_deferred,
            &ctx.pti_finalize_trimmed,
            &ctx.numa_default_policy_trimmed,
            &ctx.system_state,
            &ctx.rcu_core,
            &ctx.rcu_boot_end,
            &ctx.sysctl_args_deferred,
            &ctx.finalize_boundary,
        )
    {
        return failed_condition(event, state, expected, target);
    }

    crate::phases::state::mark_checked(&FINALIZE_PHASE_STATE, event, expected, target, checkpoint)
}

pub fn is_online() -> bool {
    crate::phases::state::load(&FINALIZE_PHASE_STATE) == State::Online
}
