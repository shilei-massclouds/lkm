use crate::{
    context::Context,
    objects::{
        rootfs::rootfs_phase_ready,
        state::{failed_condition, EventError, EventErrorCode, EventResult, LifecycleEvent, State},
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
    let mut provider = crate::objects::virtio_blk::live_provider(&ctx.kernel_image);
    ctx.ext2_driver.setup(&ctx.block_device_registry)?;
    ctx.ext2_volume
        .preset_default(&mut ctx.block_device_registry, &mut provider)
        .map_err(|_| rootfs_setup_error())?;
    ctx.ext2_filesystem
        .preset(&ctx.ext2_driver, &ctx.ext2_volume)
        .map_err(|_| rootfs_setup_error())?;
    ctx.ext2_filesystem
        .setup(
            &ctx.ext2_driver,
            &ctx.ext2_volume,
            &mut ctx.block_device_registry,
            &mut provider,
        )
        .map_err(|_| rootfs_setup_error())?;
    ctx.rootfs.enable(
        &ctx.rootfs_console_deferred,
        &ctx.saved_command_line,
        &ctx.kernel_init_task,
        &mut ctx.vfs_core,
        &mut ctx.fs_struct,
        &ctx.devfs,
        &ctx.block_device_registry,
        &ctx.ext2_driver,
        &ctx.ext2_volume,
        &mut ctx.ext2_filesystem,
    )?;
    ctx.integrity_keys_deferred.setup(&ctx.rootfs)?;
    ctx.rootfs_boundary.setup(
        &ctx.kunit_runtime_trimmed,
        &ctx.initramfs_sync_deferred,
        &ctx.rootfs_console_deferred,
        &ctx.rootfs,
        &ctx.integrity_keys_deferred,
    )
}

fn rootfs_setup_error() -> EventError {
    EventError::failed(
        EventErrorCode::ConditionFailed,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        State::Ready,
    )
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !rootfs_phase_ready(
        &ctx.kunit_runtime_trimmed,
        &ctx.initramfs_sync_deferred,
        &ctx.rootfs_console_deferred,
        &ctx.rootfs,
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
