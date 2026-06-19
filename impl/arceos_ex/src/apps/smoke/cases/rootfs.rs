use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        printk,
        state::State,
        vfs::{FileSystemKind, VfsInodeKind},
    },
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
        || !ctx.rootfs_enable_deferred.prepare_namespace_inputs_ready()
        || !ctx.rootfs_enable_deferred.initial_ramfs_still_active()
        || !ctx.rootfs_enable_deferred.devfs_available()
        || !ctx
            .rootfs_enable_deferred
            .block_root_device_candidate_bound()
        || !ctx.rootfs_enable_deferred.real_mount_deferred()
        || !ctx.rootfs_enable_deferred.ms_move_deferred()
        || !ctx.rootfs_enable_deferred.chroot_deferred()
    {
        printk::write_str("rootfs enable deferred facts invalid\n");
        return SmokeResult::Failed;
    }

    let Some(root_mount_ref) = ctx.vfs_core.current_root_mount() else {
        printk::write_str("rootfs current root mount missing\n");
        return SmokeResult::Failed;
    };
    let Some(root_mount) = ctx.vfs_core.mount(root_mount_ref) else {
        printk::write_str("rootfs current root mount invalid\n");
        return SmokeResult::Failed;
    };
    if root_mount.fs_kind() != FileSystemKind::RamFs {
        printk::write_str("rootfs current root is not initial ramfs\n");
        return SmokeResult::Failed;
    }

    let Some(dev_mount_point_ref) = ctx.devfs.mount_point_ref() else {
        printk::write_str("devfs mount point missing\n");
        return SmokeResult::Failed;
    };
    let Some(dev_mount_point) = ctx.vfs_core.dentry(dev_mount_point_ref) else {
        printk::write_str("devfs mount point invalid\n");
        return SmokeResult::Failed;
    };
    let Some(dev_root_ref) = ctx.devfs.root_dentry_ref() else {
        printk::write_str("devfs root missing\n");
        return SmokeResult::Failed;
    };
    if dev_mount_point.name() != b"dev" || dev_mount_point.mounted_root() != Some(dev_root_ref) {
        printk::write_str("devfs is not mounted at /dev\n");
        return SmokeResult::Failed;
    }

    let Some(default_entry) = ctx.block_device_registry.default_entry() else {
        printk::write_str("root block device candidate missing\n");
        return SmokeResult::Failed;
    };
    if ctx.rootfs_enable_deferred.root_device_ref() != Some(default_entry.device_ref())
        || ctx.rootfs_enable_deferred.root_device_devt() != Some(default_entry.devt())
    {
        printk::write_str("root block device candidate binding invalid\n");
        return SmokeResult::Failed;
    }

    let Some(block_node) = ctx.devfs.block_node() else {
        printk::write_str("devfs block node missing\n");
        return SmokeResult::Failed;
    };
    let Some(block_dentry) = ctx.vfs_core.dentry(block_node.dentry_ref()) else {
        printk::write_str("devfs block dentry invalid\n");
        return SmokeResult::Failed;
    };
    let Some(block_inode) = ctx.vfs_core.inode(block_dentry.inode_ref()) else {
        printk::write_str("devfs block inode invalid\n");
        return SmokeResult::Failed;
    };
    if block_node.block_device_ref() != Some(default_entry.device_ref())
        || block_node.devt() != Some(default_entry.devt())
        || block_inode.kind() != VfsInodeKind::DeviceNode
    {
        printk::write_str("devfs block node binding invalid\n");
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

    printk::write_str("rootfs next=finalize prepare_namespace_inputs=ready real_mount=deferred\n");
    SmokeResult::Passed
}
