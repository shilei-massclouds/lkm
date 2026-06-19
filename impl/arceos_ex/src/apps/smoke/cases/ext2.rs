use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context,
    objects::{
        ext2::{
            Ext2FileType, Ext2Mount, Ext2Type, EXT2_MIN_BLOCK_SIZE, EXT2_ROOT_INO,
            EXT2_SMOKE_FILE_CONTENT, EXT2_SMOKE_FILE_NAME,
        },
        state::State,
        virtio_blk,
    },
};

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
        "ext2.read_only_root_lookup_and_file_read"
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

        let mut fs_type = Ext2Type::new();
        assertions.assert_ok("ext2 type setup", fs_type.setup(&ctx.block_device_registry));
        assertions.assert("type ready", fs_type.state() == State::Ready);
        assertions.assert("type declared", fs_type.declared());
        assertions.assert("type read only", fs_type.read_only());
        assertions.assert("type mount callback", fs_type.mount_callback_bound());

        let mut provider = virtio_blk::live_provider(&ctx.kernel_image);
        let mut mount = Ext2Mount::new();
        let mounted = mount.mount_default(&fs_type, &mut ctx.block_device_registry, &mut provider);
        assertions.assert_ok("mount default", mounted);
        if mount.state() != State::Ready {
            return;
        }

        assertions.assert("mount devt", mount.devt() == Some(devt));
        assertions.assert("mount ready", mount.ready());
        assertions.assert("superblock read", mount.superblock_read());
        assertions.assert("magic valid", mount.magic_valid());
        assertions.assert("block size", mount.block_size() == EXT2_MIN_BLOCK_SIZE);
        assertions.assert("block size supported", mount.block_size_supported());
        assertions.assert("group desc read", mount.group_desc_read());
        assertions.assert("inode table", mount.group_inode_table_block() != 0);
        assertions.assert("inode size", mount.inode_size() >= 128);
        assertions.assert("inodes count", mount.inodes_count() != 0);
        assertions.assert("blocks count", mount.blocks_count() != 0);
        assertions.assert("blocks per group", mount.blocks_per_group() != 0);
        assertions.assert("inodes per group", mount.inodes_per_group() != 0);
        assertions.assert("vfs deferred", mount.vfs_integration_deferred());
        assertions.assert("page cache deferred", mount.page_cache_deferred());
        assertions.assert("writes deferred", mount.write_paths_deferred());

        let root = mount.root_inode();
        assertions.assert("root ino", root.ino() == EXT2_ROOT_INO);
        assertions.assert("root dir", root.is_root_dir());
        assertions.assert("root direct block", root.direct_blocks()[0] != 0);
        assertions.assert("root indirect deferred", root.indirect_blocks_deferred());

        let lookup = mount.lookup_root(
            &mut ctx.block_device_registry,
            &mut provider,
            EXT2_SMOKE_FILE_NAME,
        );
        assertions.assert_ok("lookup smoke file", lookup);
        let dirent = mount.lookup_dirent();
        assertions.assert("lookup name", mount.lookup_name_bound());
        assertions.assert("lookup reads root", mount.lookup_reads_root_dir());
        assertions.assert("dirent valid", mount.lookup_dirent_valid());
        assertions.assert("dirent name", dirent.name() == EXT2_SMOKE_FILE_NAME);
        assertions.assert("dirent inode", dirent.inode() != 0);
        assertions.assert(
            "dirent type",
            dirent.file_type() == Ext2FileType::RegularFile,
        );
        assertions.assert("lookup inode", mount.lookup_returns_inode());
        assertions.assert("file regular", mount.lookup_file_inode().is_regular_file());
        assertions.assert(
            "file direct block",
            mount.lookup_file_inode().direct_blocks()[0] != 0,
        );

        let mut buffer = [0u8; 64];
        let read =
            mount.read_lookup_file(&mut ctx.block_device_registry, &mut provider, &mut buffer);
        let Ok(len) = read else {
            assertions.assert("read file", false);
            return;
        };
        assertions.assert("read len", len == EXT2_SMOKE_FILE_CONTENT.len());
        assertions.assert("read content", &buffer[..len] == EXT2_SMOKE_FILE_CONTENT);
        assertions.assert("last read len", mount.last_file_read_len() == len);
        assertions.assert("read direct", mount.file_read_uses_direct_block());
        assertions.assert("read buffer head", mount.file_read_uses_buffer_head());
        assertions.assert("read copies", mount.file_read_copies_to_caller());
        assertions.assert(
            "read len matches inode",
            mount.file_read_len_matches_inode_size(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
