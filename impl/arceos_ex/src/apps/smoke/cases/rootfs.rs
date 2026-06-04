use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::smp_runtime::rootfs::is_ready() || !phases::smp_runtime::is_ready() {
        printk::write_str("rootfs phase is not ready\n");
        return SmokeResult::Failed;
    }

    if ctx.kunit_runtime_trimmed.state() != State::Ready
        || !ctx.kunit_runtime_trimmed.trimmed_noop()
        || !ctx.kunit_runtime_trimmed.position_preserved()
    {
        printk::write_str("kunit runtime trimmed facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.initramfs_sync_deferred.state() != State::Ready
        || !ctx.initramfs_sync_deferred.wait_deferred()
        || !ctx
            .initramfs_sync_deferred
            .async_cookie_boundary_preserved()
    {
        printk::write_str("initramfs sync deferred facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.rootfs_console_deferred.state() != State::Ready
        || !ctx.rootfs_console_deferred.setup_deferred()
        || !ctx
            .rootfs_console_deferred
            .pid1_console_fd_position_preserved()
    {
        printk::write_str("rootfs console deferred facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.rootfs_enable_deferred.state() != State::Ready
        || !ctx
            .rootfs_enable_deferred
            .ramdisk_eaccess_requires_prepare_namespace()
        || !ctx.rootfs_enable_deferred.enable_deferred()
        || !ctx
            .rootfs_enable_deferred
            .prepare_namespace_position_preserved()
    {
        printk::write_str("rootfs enable deferred facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.integrity_keys_deferred.state() != State::Ready
        || !ctx.integrity_keys_deferred.setup_deferred()
        || !ctx.integrity_keys_deferred.load_keys_position_preserved()
        || !ctx.integrity_keys_deferred.config_integrity_enabled()
    {
        printk::write_str("integrity keys deferred facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.rootfs_boundary.state() != State::Ready || !ctx.rootfs_boundary.finalize_next_boundary()
    {
        printk::write_str("rootfs boundary facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_str("rootfs next=finalize rootfs_enable=deferred\n");
    SmokeResult::Passed
}
