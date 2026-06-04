use crate::{
    context::Context,
    objects::{
        rootfs::rootfs_phase_ready,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static ROOTFS_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::RootfsPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex rootfs event failed\n",
    );
    crate::phases::smp_runtime::finalize::setup(ctx)
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    if !crate::phases::smp_runtime::initcall::is_ready() {
        return failed_condition(
            LifecycleEvent::Setup,
            State::Base,
            State::Ready,
            State::Ready,
        );
    }

    ctx.kunit_runtime_trimmed.setup(&ctx.initcall_boundary)?;
    ctx.initramfs_sync_deferred
        .setup(&ctx.kunit_runtime_trimmed, &ctx.workqueue)?;
    ctx.rootfs_console_deferred
        .setup(&ctx.initramfs_sync_deferred, &ctx.kernel_init_task)?;
    ctx.rootfs_enable_deferred.setup(
        &ctx.rootfs_console_deferred,
        &ctx.saved_command_line,
        &ctx.kernel_init_task,
    )?;
    ctx.integrity_keys_deferred
        .setup(&ctx.rootfs_enable_deferred)?;
    ctx.rootfs_boundary.setup(
        &ctx.kunit_runtime_trimmed,
        &ctx.initramfs_sync_deferred,
        &ctx.rootfs_console_deferred,
        &ctx.rootfs_enable_deferred,
        &ctx.integrity_keys_deferred,
    )
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !rootfs_phase_ready(
        &ctx.kunit_runtime_trimmed,
        &ctx.initramfs_sync_deferred,
        &ctx.rootfs_console_deferred,
        &ctx.rootfs_enable_deferred,
        &ctx.integrity_keys_deferred,
        &ctx.rootfs_boundary,
    ) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&ROOTFS_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &ROOTFS_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::RootfsPhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&ROOTFS_PHASE_STATE) == State::Ready
}
