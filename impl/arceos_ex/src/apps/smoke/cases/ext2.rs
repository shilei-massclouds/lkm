use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context,
    objects::{
        ext2::{
            Ext2FileType, EXT2_ALPINE_INSTALLED_DB_FILE_NAME, EXT2_ALPINE_INSTALLED_DB_MAX_SIZE,
            EXT2_ALPINE_INSTALLED_DB_PATH, EXT2_MAX_BLOCK_SIZE, EXT2_ROOT_INO,
        },
        rootfs::ROOTFS_REAL_MOUNT_POINT_NAME,
        state::State,
        vfs::{FileSystemKind, VfsInodeKind},
        virtio_blk,
    },
};

static mut LARGE_READ_BUFFER: [u8; EXT2_ALPINE_INSTALLED_DB_MAX_SIZE] =
    [0; EXT2_ALPINE_INSTALLED_DB_MAX_SIZE];

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
        "ext2.vfs_path_read_only_mount_lookup_and_file_read"
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
        assertions.assert("driver ready", ctx.ext2_driver.state() == State::Ready);
        assertions.assert("driver registered", ctx.ext2_driver.registered());
        assertions.assert("driver read only", ctx.ext2_driver.read_only());
        assertions.assert(
            "driver mount callback",
            ctx.ext2_driver.mount_callback_bound(),
        );
        assertions.assert("driver super ops", ctx.ext2_driver.super_operations_bound());
        assertions.assert("driver inode ops", ctx.ext2_driver.inode_operations_bound());
        assertions.assert("driver file ops", ctx.ext2_driver.file_operations_bound());

        assertions.assert("volume ready", ctx.ext2_volume.state() == State::Ready);
        assertions.assert("volume devt", ctx.ext2_volume.devt() == Some(devt));
        assertions.assert("volume superblock read", ctx.ext2_volume.superblock_read());
        assertions.assert("volume magic valid", ctx.ext2_volume.magic_valid());
        assertions.assert("volume layout valid", ctx.ext2_volume.layout_valid());
        assertions.assert(
            "volume block size",
            ctx.ext2_volume.block_size() == EXT2_MAX_BLOCK_SIZE,
        );
        assertions.assert(
            "volume block size supported",
            ctx.ext2_volume.block_size_supported(),
        );
        assertions.assert(
            "volume nonfatal absent",
            ctx.ext2_volume.not_found_nonfatal(),
        );

        if ctx.ext2_filesystem.state() != State::Online {
            assertions.assert("filesystem online", false);
            return;
        }
        let fs = &ctx.ext2_filesystem;
        let Some(mount_ref) = fs.vfs_mount_ref() else {
            assertions.assert("filesystem vfs mount", false);
            return;
        };
        let Some(mount_point_ref) = fs.vfs_mount_point_ref() else {
            assertions.assert("filesystem vfs mount point", false);
            return;
        };

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
            mount.mount_point_ref() == ctx.vfs_core.initial_root_dentry(),
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
            "vfs mount point name",
            mount_point.name() == ROOTFS_REAL_MOUNT_POINT_NAME,
        );
        assertions.assert("vfs mount redirects", mount_point.mounted_root().is_none());
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

        let buffer = unsafe {
            let ptr = core::ptr::addr_of_mut!(LARGE_READ_BUFFER);
            &mut *ptr
        };
        buffer.fill(0);
        let read = ctx.vfs_core.read_path(
            &ctx.fs_struct,
            &mut ctx.ext2_filesystem,
            &mut ctx.block_device_registry,
            &mut provider,
            EXT2_ALPINE_INSTALLED_DB_PATH,
            buffer,
        );
        let Ok(len) = read else {
            assertions.assert("read path", false);
            return;
        };

        let dirent = *ctx.ext2_filesystem.lookup_dirent();
        assertions.assert("lookup name", ctx.ext2_filesystem.lookup_name_bound());
        assertions.assert("lookup reads dir", ctx.ext2_filesystem.lookup_reads_dir());
        assertions.assert(
            "lookup scans direct",
            ctx.ext2_filesystem.lookup_direct_blocks_scanned() >= 1,
        );
        assertions.assert(
            "lookup multi direct",
            ctx.ext2_filesystem.lookup_multi_direct_block_supported(),
        );
        assertions.assert(
            "lookup notfound nonfatal",
            ctx.ext2_filesystem.lookup_not_found_nonfatal(),
        );
        assertions.assert(
            "lookup indirect deferred",
            ctx.ext2_filesystem.lookup_indirect_blocks_deferred(),
        );
        assertions.assert("dirent valid", ctx.ext2_filesystem.lookup_dirent_valid());
        assertions.assert(
            "dirent name",
            dirent.name() == EXT2_ALPINE_INSTALLED_DB_FILE_NAME,
        );
        assertions.assert("dirent inode", dirent.inode() != 0);
        assertions.assert(
            "dirent type",
            dirent.file_type() == Ext2FileType::RegularFile,
        );
        assertions.assert("lookup inode", ctx.ext2_filesystem.lookup_returns_inode());
        assertions.assert(
            "file regular",
            ctx.ext2_filesystem.lookup_file_inode().is_regular_file(),
        );
        assertions.assert(
            "file direct block",
            ctx.ext2_filesystem.lookup_file_inode().direct_blocks()[0] != 0,
        );
        assertions.assert(
            "file single indirect supported",
            ctx.ext2_filesystem
                .lookup_file_inode()
                .single_indirect_block()
                != 0
                || file_inode_size_within_direct_blocks(ctx.ext2_filesystem.lookup_file_inode()),
        );
        assertions.assert("vfs lookup fact", ctx.vfs_core.ext2_lookup_dispatched());
        assertions.assert(
            "absolute path walk supported",
            ctx.vfs_core.absolute_path_walk_supported(),
        );
        assertions.assert("path walk resolved", ctx.vfs_core.path_walk_resolved());
        assertions.assert("open path file", ctx.vfs_core.open_path_allocated_file());
        assertions.assert("read path data", ctx.vfs_core.read_path_returns_data());
        let file_dentry_ref = match ctx.vfs_core.walk_path(
            &ctx.fs_struct,
            &mut ctx.ext2_filesystem,
            &mut ctx.block_device_registry,
            &mut provider,
            EXT2_ALPINE_INSTALLED_DB_PATH,
        ) {
            Ok(dentry_ref) => dentry_ref,
            Err(_) => {
                assertions.assert("vfs walk file path", false);
                return;
            }
        };
        let Some(file_dentry) = ctx.vfs_core.dentry(file_dentry_ref) else {
            assertions.assert("vfs file dentry", false);
            return;
        };
        let file_name_matches = file_dentry.name() == EXT2_ALPINE_INSTALLED_DB_FILE_NAME;
        let file_dentry_ref_value = file_dentry.dentry_ref();
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
        assertions.assert("vfs file name", file_name_matches);
        let file_inode_ref = file.inode_ref();
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
            "vfs file spans blocks",
            file_inode.size() > EXT2_MAX_BLOCK_SIZE
                && file_inode.size() <= EXT2_ALPINE_INSTALLED_DB_MAX_SIZE,
        );
        assertions.assert(
            "vfs file dentry bound",
            file.dentry_ref() == file_dentry_ref_value,
        );
        assertions.assert("vfs file inode bound", file.inode_ref() == file_inode_ref);

        assertions.assert(
            "read len",
            len > EXT2_MAX_BLOCK_SIZE && len <= EXT2_ALPINE_INSTALLED_DB_MAX_SIZE,
        );
        assertions.assert(
            "read content",
            contains_bytes(&buffer[..len], b"P:alpine-baselayout\n")
                && contains_bytes(&buffer[..len], b"A:riscv64\n"),
        );
        assertions.assert(
            "last read len",
            ctx.ext2_filesystem.last_file_read_len() == len,
        );
        assertions.assert(
            "read direct",
            ctx.ext2_filesystem.file_read_uses_direct_block(),
        );
        assertions.assert(
            "read scans direct",
            ctx.ext2_filesystem.file_read_direct_blocks_scanned() >= 2,
        );
        assertions.assert(
            "read multi direct",
            ctx.ext2_filesystem.file_read_multi_direct_block_supported(),
        );
        assertions.assert(
            "read buffer head",
            ctx.ext2_filesystem.file_read_uses_buffer_head(),
        );
        assertions.assert(
            "read copies",
            ctx.ext2_filesystem.file_read_copies_to_caller(),
        );
        assertions.assert(
            "read len matches inode",
            ctx.ext2_filesystem.file_read_len_matches_inode_size(),
        );
        assertions.assert(
            "read indirect deferred",
            ctx.ext2_filesystem.file_read_indirect_blocks_deferred(),
        );
        assertions.assert(
            "read entered from vfs",
            ctx.ext2_filesystem.file_read_entered_from_vfs(),
        );
        assertions.assert("vfs read fact", ctx.vfs_core.ext2_read_dispatched());
        assertions.assert(
            "vfs read backend data",
            ctx.vfs_core.file_read_returns_backend_data(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

fn file_inode_size_within_direct_blocks(inode: &crate::objects::ext2::Ext2InodeRecord) -> bool {
    inode.size() as usize <= EXT2_MAX_BLOCK_SIZE * crate::objects::ext2::EXT2_NDIR_BLOCKS
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
