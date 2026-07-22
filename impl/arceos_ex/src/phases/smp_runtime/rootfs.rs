use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        rootfs::rootfs_phase_ready,
        state::{EventError, EventErrorCode, EventResult, LifecycleEvent, State, failed_condition},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static ROOTFS_PHASE_STATE: AtomicU8 = AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(preset_start(), "arceos_ex rootfs preset start failed\n");
    crate::checkpoint::checkpoint(Checkpoint::RootfsPhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| adopt_prepared(ctx)),
        "arceos_ex rootfs preset failed\n",
    );
    setup(ctx)
}

fn preset_start() -> EventResult {
    let state = crate::phases::state::load(&ROOTFS_PHASE_STATE);
    if state != State::Base || !crate::phases::smp_runtime::initcall::is_online() {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    ctx.kunit_runtime_trimmed.setup(&ctx.initcall_boundary)?;
    ctx.initramfs_sync_deferred
        .setup(&ctx.kunit_runtime_trimmed, &ctx.workqueue)?;
    ctx.rootfs_console_deferred
        .setup(&ctx.initramfs_sync_deferred, &ctx.kernel_init_task)?;
    crate::checkpoint::checkpoint(Checkpoint::RamdiskExecuteCommandEaccessCheckpoint);
    ctx.rootfs_prepare_namespace_paths.setup(
        &ctx.rootfs_console_deferred,
        &ctx.saved_command_line,
        &ctx.driver_core_base,
        &ctx.workqueue,
        &ctx.initcall_boundary,
        &ctx.config,
    )?;
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
        &ctx.rootfs_prepare_namespace_paths,
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
    ctx.integrity_keys_deferred
        .setup(&ctx.rootfs, &ctx.config)?;
    ctx.rootfs_boundary.setup(
        &ctx.kunit_runtime_trimmed,
        &ctx.initramfs_sync_deferred,
        &ctx.rootfs_console_deferred,
        &ctx.rootfs_prepare_namespace_paths,
        &ctx.rootfs,
        &ctx.integrity_keys_deferred,
    )
}

fn rootfs_setup_error() -> EventError {
    EventError::failed(
        EventErrorCode::ConditionFailed,
        LifecycleEvent::Preset,
        State::Base,
        State::Base,
        State::Prepared,
    )
}

fn adopt_prepared(ctx: &Context) -> EventResult {
    transition_if_ready(
        ctx,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::RootfsPhasePrepared,
    )
}

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        transition_if_ready(
            ctx,
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::RootfsPhaseReady,
        ),
        "arceos_ex rootfs setup failed\n",
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
            Checkpoint::RootfsPhaseOnline,
        ),
        "arceos_ex rootfs enable failed\n",
    );
    crate::phases::smp_runtime::setup_after_rootfs()
}

fn transition_if_ready(
    ctx: &Context,
    event: LifecycleEvent,
    expected: State,
    target: State,
    checkpoint: Checkpoint,
) -> EventResult {
    let state = crate::phases::state::load(&ROOTFS_PHASE_STATE);
    if state != expected
        || !rootfs_phase_ready(
            &ctx.kunit_runtime_trimmed,
            &ctx.initramfs_sync_deferred,
            &ctx.rootfs_console_deferred,
            &ctx.rootfs_prepare_namespace_paths,
            &ctx.rootfs,
            &ctx.integrity_keys_deferred,
            &ctx.rootfs_boundary,
        )
    {
        return failed_condition(event, state, expected, target);
    }

    crate::phases::state::mark_checked(&ROOTFS_PHASE_STATE, event, expected, target, checkpoint)
}

pub fn is_online() -> bool {
    crate::phases::state::load(&ROOTFS_PHASE_STATE) == State::Online
}
