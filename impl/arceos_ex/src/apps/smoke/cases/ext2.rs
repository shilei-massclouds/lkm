use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context,
    objects::{
        ext2::{
            Ext2Driver, Ext2Error, Ext2FileSystem, Ext2FileType, Ext2Volume, EXT2_MAX_BLOCK_SIZE,
            EXT2_ROOT_INO, EXT2_SMOKE_LARGE_FILE_BYTE, EXT2_SMOKE_LARGE_FILE_NAME,
            EXT2_SMOKE_LARGE_FILE_SIZE,
        },
        state::State,
        vfs::{FileSystemKind, VfsError, VfsInodeKind},
        virtio_blk,
    },
};

static mut LARGE_READ_BUFFER: [u8; EXT2_SMOKE_LARGE_FILE_SIZE] = [0; EXT2_SMOKE_LARGE_FILE_SIZE];
const EXT2_MOUNT_POINT_NAME: &[u8] = b"mnt_ext2";

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut Ext2ReadOnlyScenario::new());
    suite.result()
}

struct Ext2ReadOnlyScenario;

impl Ext2ReadOnlyScenario {
    const fn new() -> Self {
        Self
    }
}

impl SmokeScenario for Ext2ReadOnlyScenario {
    fn name(&self) -> &'static str {
        "ext2.vfs_read_only_mount_lookup_and_file_read"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        assertions.assert(
            "block registry ready",
            ctx.block_device_registry.state() == State::Ready,
        );
        assertions.assert(
            "default block present",
            ctx.block_device_registry.default_entry().is_some(),
        );
        assertions.assert(
            "virtio blk ready",
            ctx.virtio_blk_runtime
                .device()
                .is_some_and(|device| device.state() == State::Ready && device.driver_ok()),
        );
    }

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        let Some(default_entry) = ctx.block_device_registry.default_entry() else {
            assertions.assert("default entry", false);
            return;
        };
        let devt = default_entry.devt();

        let mut provider = virtio_blk::live_provider(&ctx.kernel_image);
        let mut driver = Ext2Driver::new();
        assertions.assert_ok(
            "ext2 driver setup",
            driver.setup(&ctx.block_device_registry),
        );
        assertions.assert("driver ready", driver.state() == State::Ready);
        assertions.assert("driver registered", driver.registered());
        assertions.assert("driver read only", driver.read_only());
        assertions.assert("driver mount callback", driver.mount_callback_bound());
        assertions.assert("driver super ops", driver.super_operations_bound());
        assertions.assert("driver inode ops", driver.inode_operations_bound());
        assertions.assert("driver file ops", driver.file_operations_bound());

        let mut volume = Ext2Volume::new();
        let volume_preset = volume.preset_default(&mut ctx.block_device_registry, &mut provider);
        assert_ext2_result(assertions, "volume preset", volume_preset);
        if volume.state() != State::Ready {
            return;
        }
        assertions.assert("volume devt", volume.devt() == Some(devt));
        assertions.assert("volume superblock read", volume.superblock_read());
        assertions.assert("volume magic valid", volume.magic_valid());
        assertions.assert("volume layout valid", volume.layout_valid());
        assertions.assert(
            "volume block size",
            volume.block_size() == EXT2_MAX_BLOCK_SIZE,
        );
        assertions.assert("volume block size supported", volume.block_size_supported());
        assertions.assert("volume nonfatal absent", volume.not_found_nonfatal());

        let mut fs = Ext2FileSystem::new();
        assert_ext2_result(assertions, "filesystem preset", fs.preset(&driver, &volume));
        if fs.state() != State::Prepared {
            return;
        }
        let setup = fs.setup(
            &driver,
            &volume,
            &mut ctx.block_device_registry,
            &mut provider,
        );
        assert_ext2_result(assertions, "filesystem setup", setup);
        if fs.state() != State::Ready {
            return;
        }
        let Some(root_dentry_ref) = ctx.vfs_core.current_root_dentry() else {
            assertions.assert("root dentry present", false);
            return;
        };
        let mount_point_ref = match ctx
            .vfs_core
            .lookup_child(root_dentry_ref, EXT2_MOUNT_POINT_NAME)
        {
            Ok(dentry_ref) => dentry_ref,
            Err(_) => match ctx
                .vfs_core
                .create_dir(root_dentry_ref, EXT2_MOUNT_POINT_NAME)
            {
                Ok(dentry_ref) => dentry_ref,
                Err(_) => {
                    assertions.assert("create ext2 mount point", false);
                    return;
                }
            },
        };
        let mount_ref = match fs.enable(&mut ctx.vfs_core, mount_point_ref) {
            Ok(mount_ref) => mount_ref,
            Err(error) => {
                assert_ext2_result(assertions, "filesystem enable", Err(error));
                return;
            }
        };
        if fs.state() != State::Online {
            return;
        }

        assertions.assert("filesystem devt", fs.devt() == Some(devt));
        assertions.assert("filesystem ready", fs.ready());
        assertions.assert("filesystem mount boundary", fs.mount_boundary_recorded());
        assertions.assert(
            "filesystem vfs mount",
            fs.vfs_mount_ref() == Some(mount_ref),
        );
        assertions.assert(
            "filesystem vfs mount point",
            fs.vfs_mount_point_ref() == Some(mount_point_ref),
        );
        assertions.assert("filesystem vfs lookup", fs.vfs_lookup_entry_bound());
        assertions.assert("filesystem vfs read", fs.vfs_read_entry_bound());
        assertions.assert("filesystem operations", fs.operations_bound());
        assertions.assert("filesystem root dentry", fs.root_dentry_bound());
        assertions.assert("superblock read", fs.superblock_read());
        assertions.assert("magic valid", fs.magic_valid());
        assertions.assert("block size", fs.block_size() == EXT2_MAX_BLOCK_SIZE);
        assertions.assert("block size supported", fs.block_size_supported());
        assertions.assert("group desc read", fs.group_desc_read());
        assertions.assert("inode table", fs.group_inode_table_block() != 0);
        assertions.assert("inode size", fs.inode_size() >= 128);
        assertions.assert("inodes count", fs.inodes_count() != 0);
        assertions.assert("blocks count", fs.blocks_count() != 0);
        assertions.assert("blocks per group", fs.blocks_per_group() != 0);
        assertions.assert("inodes per group", fs.inodes_per_group() != 0);
        assertions.assert("vfs integrated", fs.vfs_mount_ref().is_some());
        assertions.assert("page cache deferred", fs.page_cache_deferred());
        assertions.assert("writes deferred", fs.write_paths_deferred());
        assertions.assert("vfs ext2 mount fact", ctx.vfs_core.ext2_mount_created());

        let Some(mount) = ctx.vfs_core.mount(mount_ref) else {
            assertions.assert("vfs ext2 mount", false);
            return;
        };
        assertions.assert("vfs mount kind", mount.fs_kind() == FileSystemKind::Ext2);
        assertions.assert(
            "vfs mount point",
            mount.mount_point_ref() == Some(mount_point_ref),
        );
        let Some(superblock) = ctx.vfs_core.superblock(mount.superblock_ref()) else {
            assertions.assert("vfs ext2 superblock", false);
            return;
        };
        assertions.assert(
            "vfs superblock kind",
            superblock.fs_kind() == FileSystemKind::Ext2,
        );
        assertions.assert("vfs superblock private", superblock.ext2_private_bound());
        let ext2_root_ref = mount.root_dentry_ref();
        let Some(mount_point) = ctx.vfs_core.dentry(mount_point_ref) else {
            assertions.assert("vfs mount point dentry", false);
            return;
        };
        assertions.assert(
            "vfs mount redirects",
            mount_point.mounted_root() == Some(ext2_root_ref),
        );
        let Some(ext2_root) = ctx.vfs_core.dentry(ext2_root_ref) else {
            assertions.assert("vfs ext2 root dentry", false);
            return;
        };
        let Some(ext2_root_inode) = ctx.vfs_core.inode(ext2_root.inode_ref()) else {
            assertions.assert("vfs ext2 root inode", false);
            return;
        };
        assertions.assert(
            "vfs root dir",
            ext2_root_inode.kind() == VfsInodeKind::Directory,
        );
        assertions.assert("vfs root readonly", ext2_root_inode.read_only_backed());
        assertions.assert(
            "vfs root ext2 ino",
            ext2_root_inode
                .ext2_binding()
                .is_some_and(|binding| binding.ino() == EXT2_ROOT_INO),
        );

        let root = fs.root_inode();
        assertions.assert("root ino", root.ino() == EXT2_ROOT_INO);
        assertions.assert("root dir", root.is_root_dir());
        assertions.assert("root direct block", root.direct_blocks()[0] != 0);
        assertions.assert("root indirect deferred", root.indirect_blocks_deferred());

        let lookup = ctx.vfs_core.lookup_ext2_child(
            &mut fs,
            &mut ctx.block_device_registry,
            &mut provider,
            mount_point_ref,
            EXT2_SMOKE_LARGE_FILE_NAME,
        );
        assertions.assert_ok("lookup smoke large file", lookup);
        let file_dentry_ref = match lookup {
            Ok(dentry_ref) => dentry_ref,
            Err(_) => return,
        };
        let dirent = fs.lookup_dirent();
        assertions.assert("lookup name", fs.lookup_name_bound());
        assertions.assert("lookup reads root", fs.lookup_reads_root_dir());
        assertions.assert(
            "lookup scans direct",
            fs.lookup_direct_blocks_scanned() >= 2,
        );
        assertions.assert(
            "lookup multi direct",
            fs.lookup_multi_direct_block_supported(),
        );
        assertions.assert("lookup notfound nonfatal", fs.lookup_not_found_nonfatal());
        assertions.assert(
            "lookup indirect deferred",
            fs.lookup_indirect_blocks_deferred(),
        );
        assertions.assert("dirent valid", fs.lookup_dirent_valid());
        assertions.assert("dirent name", dirent.name() == EXT2_SMOKE_LARGE_FILE_NAME);
        assertions.assert("dirent inode", dirent.inode() != 0);
        assertions.assert(
            "dirent type",
            dirent.file_type() == Ext2FileType::RegularFile,
        );
        assertions.assert("lookup inode", fs.lookup_returns_inode());
        assertions.assert("file regular", fs.lookup_file_inode().is_regular_file());
        assertions.assert(
            "file direct block",
            fs.lookup_file_inode().direct_blocks()[0] != 0,
        );
        assertions.assert("vfs lookup fact", ctx.vfs_core.ext2_lookup_dispatched());
        let Some(file_dentry) = ctx.vfs_core.dentry(file_dentry_ref) else {
            assertions.assert("vfs file dentry", false);
            return;
        };
        assertions.assert(
            "vfs file name",
            file_dentry.name() == EXT2_SMOKE_LARGE_FILE_NAME,
        );
        let file_inode_ref = file_dentry.inode_ref();
        let Some(file_inode) = ctx.vfs_core.inode(file_inode_ref) else {
            assertions.assert("vfs file inode", false);
            return;
        };
        assertions.assert(
            "vfs file kind",
            file_inode.kind() == VfsInodeKind::RegularFile,
        );
        assertions.assert("vfs file readonly", file_inode.read_only_backed());
        assertions.assert(
            "vfs file ext2 ino",
            file_inode
                .ext2_binding()
                .is_some_and(|binding| binding.ino() == dirent.inode()),
        );
        assertions.assert(
            "vfs file size",
            file_inode.size() == EXT2_SMOKE_LARGE_FILE_SIZE,
        );
        let file_ref = match ctx.vfs_core.open_file(file_dentry_ref) {
            Ok(file_ref) => file_ref,
            Err(_) => {
                assertions.assert("vfs open ext2 file", false);
                return;
            }
        };
        let Some(file) = ctx.vfs_core.file(file_ref) else {
            assertions.assert("vfs file object", false);
            return;
        };
        assertions.assert(
            "vfs file dentry bound",
            file.dentry_ref() == file_dentry_ref,
        );
        assertions.assert("vfs file inode bound", file.inode_ref() == file_inode_ref);

        let mut short_buffer = [0u8; 64];
        let short_read = ctx.vfs_core.read_ext2_file(
            &mut fs,
            &mut ctx.block_device_registry,
            &mut provider,
            file_ref,
            0,
            &mut short_buffer,
        );
        assertions.assert(
            "short buffer rejected",
            matches!(short_read, Err(VfsError::ShortBuffer))
                && fs.file_read_short_buffer_rejected(),
        );

        let buffer = unsafe {
            let ptr = core::ptr::addr_of_mut!(LARGE_READ_BUFFER);
            &mut *ptr
        };
        buffer.fill(0);
        let read = ctx.vfs_core.read_ext2_file(
            &mut fs,
            &mut ctx.block_device_registry,
            &mut provider,
            file_ref,
            0,
            buffer,
        );
        let Ok(len) = read else {
            assertions.assert("read file", false);
            return;
        };
        assertions.assert("read len", len == EXT2_SMOKE_LARGE_FILE_SIZE);
        assertions.assert(
            "read content",
            buffer[..len]
                .iter()
                .all(|byte| *byte == EXT2_SMOKE_LARGE_FILE_BYTE),
        );
        assertions.assert("last read len", fs.last_file_read_len() == len);
        assertions.assert("read direct", fs.file_read_uses_direct_block());
        assertions.assert(
            "read scans direct",
            fs.file_read_direct_blocks_scanned() >= 2,
        );
        assertions.assert(
            "read multi direct",
            fs.file_read_multi_direct_block_supported(),
        );
        assertions.assert("read buffer head", fs.file_read_uses_buffer_head());
        assertions.assert("read copies", fs.file_read_copies_to_caller());
        assertions.assert(
            "read len matches inode",
            fs.file_read_len_matches_inode_size(),
        );
        assertions.assert(
            "read indirect deferred",
            fs.file_read_indirect_blocks_deferred(),
        );
        assertions.assert("read entered from vfs", fs.file_read_entered_from_vfs());
        assertions.assert("vfs read fact", ctx.vfs_core.ext2_read_dispatched());
        assertions.assert(
            "vfs read backend data",
            ctx.vfs_core.file_read_returns_backend_data(),
        );
        let Some(file_after_read) = ctx.vfs_core.file(file_ref) else {
            assertions.assert("vfs file after read", false);
            return;
        };
        assertions.assert("vfs file read len", file_after_read.last_read_len() == len);
        assertions.assert(
            "vfs file backend data",
            file_after_read.read_returns_backend_data(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

fn assert_ext2_result(
    assertions: &mut SmokeAssertions,
    label: &'static str,
    result: Result<(), Ext2Error>,
) {
    match result {
        Ok(()) => assertions.assert(label, true),
        Err(Ext2Error::DriverNotReady) => assertions.assert("ext2 error: driver not ready", false),
        Err(Ext2Error::VolumeNotReady) => assertions.assert("ext2 error: volume not ready", false),
        Err(Ext2Error::FileSystemNotReady) => {
            assertions.assert("ext2 error: filesystem not ready", false)
        }
        Err(Ext2Error::InvalidState) => assertions.assert("ext2 error: invalid state", false),
        Err(Ext2Error::DeviceMissing) => assertions.assert("ext2 error: device missing", false),
        Err(Ext2Error::Io) => assertions.assert("ext2 error: io", false),
        Err(Ext2Error::InvalidSuperblock) => {
            assertions.assert("ext2 error: invalid superblock", false)
        }
        Err(Ext2Error::UnsupportedBlockSize) => {
            assertions.assert("ext2 error: unsupported block size", false)
        }
        Err(Ext2Error::InvalidGroupDesc) => {
            assertions.assert("ext2 error: invalid group desc", false)
        }
        Err(Ext2Error::InvalidInode) => assertions.assert("ext2 error: invalid inode", false),
        Err(Ext2Error::InvalidDirEntry) => {
            assertions.assert("ext2 error: invalid dir entry", false)
        }
        Err(Ext2Error::NotFound) => assertions.assert("ext2 error: not found", false),
        Err(Ext2Error::NotDirectory) => assertions.assert("ext2 error: not directory", false),
        Err(Ext2Error::NotRegularFile) => assertions.assert("ext2 error: not regular file", false),
        Err(Ext2Error::IndirectBlocksUnsupported) => {
            assertions.assert("ext2 error: indirect blocks unsupported", false)
        }
        Err(Ext2Error::ShortBuffer) => assertions.assert("ext2 error: short buffer", false),
    }
}
