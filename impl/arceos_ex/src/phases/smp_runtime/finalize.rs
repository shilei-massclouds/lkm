use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        finalize::finalize_phase_ready,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static FINALIZE_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::checkpoint::checkpoint(Checkpoint::FinalizePhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex finalize event failed\n",
    );
    crate::phases::smp_runtime::setup_after_children()
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    if !crate::phases::smp_runtime::rootfs::is_ready() {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

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

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !finalize_phase_ready(
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
    ) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&FINALIZE_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &FINALIZE_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::FinalizePhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&FINALIZE_PHASE_STATE) == State::Ready
}
