use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        ext2::{
            EXT2_ALPINE_RELEASE_FILE_CONTENT, EXT2_ALPINE_RELEASE_FILE_NAME,
            EXT2_ALPINE_RELEASE_PATH, EXT2_MAX_BLOCK_SIZE, EXT2_NDIR_BLOCKS,
        },
        printk,
        rootfs::ROOTFS_REAL_MOUNT_POINT_NAME,
        state::State,
        vfs::{FileSystemKind, VfsInodeKind},
        virtio_blk,
    },
    phases,
};

static mut ROOTFS_READ_BUFFER: [u8; EXT2_ALPINE_RELEASE_FILE_CONTENT.len()] =
    [0; EXT2_ALPINE_RELEASE_FILE_CONTENT.len()];
static mut ROOTFS_INIT_READ_BUFFER: [u8; TEMP_USER_INIT_MAX_READ] = [0; TEMP_USER_INIT_MAX_READ];

const TEMP_USER_INIT_PATH: &[u8] = b"/sbin/init";
const TEMP_USER_INIT_FILE_NAME: &[u8] = b"init";
const TEMP_USER_INIT_MAX_READ: usize = EXT2_MAX_BLOCK_SIZE * EXT2_NDIR_BLOCKS;
const ELF_HEADER_LEN: usize = 64;
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LSB: u8 = 1;
const ELF_VERSION_CURRENT: u8 = 1;
const ELF_TYPE_EXEC: u16 = 2;
const ELF_MACHINE_RISCV: u16 = 243;

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

    if ctx.rootfs.state() != State::Online
        || !ctx.rootfs.ramdisk_eaccess_requires_prepare_namespace()
        || !ctx.rootfs.prepare_namespace_position_preserved()
        || !ctx.rootfs.prepare_namespace_inputs_ready()
        || !ctx.rootfs.initial_ramfs_still_active()
        || !ctx.rootfs.devfs_available()
        || !ctx.rootfs.block_root_device_candidate_bound()
        || !ctx.rootfs.ext2_driver_ready()
        || !ctx.rootfs.ext2_volume_ready()
        || !ctx.rootfs.ext2_filesystem_ready()
        || !ctx.rootfs.real_mount_point_created()
        || !ctx.rootfs.real_ext2_mount_created()
        || !ctx.rootfs.ms_move_done()
        || !ctx.rootfs.chroot_dot_done()
        || !ctx.rootfs.current_root_is_real_ext2()
    {
        printk::write_str("rootfs enable facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.fs_struct.state() != State::Ready
        || !ctx.fs_struct.pwd_chdir_to_real_root()
        || !ctx.fs_struct.chroot_dot_done()
        || !ctx.fs_struct.root_pwd_same()
    {
        printk::write_str("fs_struct root switch facts invalid\n");
        return SmokeResult::Failed;
    }

    let Some(root_mount_ref) = ctx.vfs_core.current_root_mount(&ctx.fs_struct) else {
        printk::write_str("rootfs current root mount missing\n");
        return SmokeResult::Failed;
    };
    let Some(root_mount) = ctx.vfs_core.mount(root_mount_ref) else {
        printk::write_str("rootfs current root mount invalid\n");
        return SmokeResult::Failed;
    };
    let Some(current_root_ref) = ctx.vfs_core.current_root_dentry(&ctx.fs_struct) else {
        printk::write_str("rootfs current root dentry missing\n");
        return SmokeResult::Failed;
    };
    if root_mount.fs_kind() != FileSystemKind::Ext2
        || root_mount.root_dentry_ref() != current_root_ref
        || ctx.vfs_core.current_pwd_dentry(&ctx.fs_struct) != Some(current_root_ref)
        || !ctx.vfs_core.current_root_is_ext2(&ctx.fs_struct)
        || !ctx.vfs_core.mount_moved_to_root()
    {
        printk::write_str("rootfs current root is not ext2\n");
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
    if ctx.rootfs.root_device_ref() != Some(default_entry.device_ref())
        || ctx.rootfs.root_device_devt() != Some(default_entry.devt())
    {
        printk::write_str("root block device candidate binding invalid\n");
        return SmokeResult::Failed;
    }

    let Some(real_mount_point_ref) = ctx.rootfs.real_mount_point_ref() else {
        printk::write_str("rootfs ext2 mount point missing\n");
        return SmokeResult::Failed;
    };
    let Some(real_mount_ref) = ctx.rootfs.real_mount_ref() else {
        printk::write_str("rootfs ext2 mount missing\n");
        return SmokeResult::Failed;
    };
    let Some(real_mount_point) = ctx.vfs_core.dentry(real_mount_point_ref) else {
        printk::write_str("rootfs ext2 mount point invalid\n");
        return SmokeResult::Failed;
    };
    let Some(real_mount) = ctx.vfs_core.mount(real_mount_ref) else {
        printk::write_str("rootfs ext2 mount invalid\n");
        return SmokeResult::Failed;
    };
    if real_mount_point.name() != ROOTFS_REAL_MOUNT_POINT_NAME
        || real_mount.fs_kind() != FileSystemKind::Ext2
        || real_mount.mount_point_ref() != ctx.vfs_core.initial_root_dentry()
        || real_mount.root_dentry_ref() != current_root_ref
        || real_mount_point.mounted_root().is_some()
    {
        printk::write_str("rootfs ext2 moved mount facts invalid\n");
        return SmokeResult::Failed;
    }
    let Some(initial_root_ref) = ctx.vfs_core.initial_root_dentry() else {
        printk::write_str("initial root dentry missing\n");
        return SmokeResult::Failed;
    };
    let Some(initial_root) = ctx.vfs_core.dentry(initial_root_ref) else {
        printk::write_str("initial root dentry invalid\n");
        return SmokeResult::Failed;
    };
    if initial_root.mounted_root() != Some(current_root_ref) {
        printk::write_str("rootfs moved root binding invalid\n");
        return SmokeResult::Failed;
    }
    if ctx.ext2_driver.state() != State::Ready
        || ctx.ext2_volume.state() != State::Ready
        || ctx.ext2_filesystem.state() != State::Online
        || ctx.ext2_filesystem.vfs_mount_ref() != Some(real_mount_ref)
        || ctx.ext2_filesystem.vfs_mount_point_ref() != Some(real_mount_point_ref)
    {
        printk::write_str("rootfs ext2 object facts invalid\n");
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

    let mut provider = virtio_blk::live_provider(&ctx.kernel_image);
    let buffer = unsafe {
        let ptr = core::ptr::addr_of_mut!(ROOTFS_READ_BUFFER);
        &mut *ptr
    };
    buffer.fill(0);
    match ctx.vfs_core.read_path(
        &ctx.fs_struct,
        &mut ctx.ext2_filesystem,
        &mut ctx.block_device_registry,
        &mut provider,
        EXT2_ALPINE_RELEASE_PATH,
        buffer,
    ) {
        Ok(len)
            if len == EXT2_ALPINE_RELEASE_FILE_CONTENT.len()
                && &buffer[..len] == EXT2_ALPINE_RELEASE_FILE_CONTENT
                && ctx.ext2_filesystem.lookup_dirent().name() == EXT2_ALPINE_RELEASE_FILE_NAME => {}
        _ => {
            printk::write_str("rootfs direct ext2 read failed\n");
            return SmokeResult::Failed;
        }
    }
    if ctx.vfs_core.path_walk_crossed_mount() {
        printk::write_str("rootfs direct read unexpectedly crossed mount\n");
        return SmokeResult::Failed;
    }

    let init_buffer = unsafe {
        let ptr = core::ptr::addr_of_mut!(ROOTFS_INIT_READ_BUFFER);
        &mut *ptr
    };
    init_buffer.fill(0);
    match ctx.vfs_core.read_path(
        &ctx.fs_struct,
        &mut ctx.ext2_filesystem,
        &mut ctx.block_device_registry,
        &mut provider,
        TEMP_USER_INIT_PATH,
        init_buffer,
    ) {
        Ok(len)
            if len >= ELF_HEADER_LEN
                && is_temp_user_init_elf(&init_buffer[..len])
                && ctx.ext2_filesystem.lookup_dirent().name() == TEMP_USER_INIT_FILE_NAME => {}
        _ => {
            printk::write_str("rootfs temporary /sbin/init ELF read failed\n");
            return SmokeResult::Failed;
        }
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

    printk::write_str("rootfs next=finalize current_root=ext2 read=/etc/alpine-release init=elf\n");
    SmokeResult::Passed
}

fn is_temp_user_init_elf(buffer: &[u8]) -> bool {
    if buffer.len() < ELF_HEADER_LEN {
        return false;
    }
    if &buffer[0..4] != b"\x7fELF"
        || buffer[4] != ELF_CLASS_64
        || buffer[5] != ELF_DATA_LSB
        || buffer[6] != ELF_VERSION_CURRENT
    {
        return false;
    }

    u16::from_le_bytes([buffer[16], buffer[17]]) == ELF_TYPE_EXEC
        && u16::from_le_bytes([buffer[18], buffer[19]]) == ELF_MACHINE_RISCV
}
