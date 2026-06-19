use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context,
    objects::{
        ext2::{
            Ext2Driver, Ext2Error, Ext2FileSystem, Ext2FileType, Ext2Volume, EXT2_MAX_BLOCK_SIZE,
            EXT2_ROOT_INO, EXT2_SMOKE_FILE_CONTENT, EXT2_SMOKE_FILE_NAME,
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
        assert_ext2_result(
            assertions,
            "filesystem enable",
            fs.enable_deferred(&ctx.vfs_core),
        );
        if fs.state() != State::Online {
            return;
        }

        assertions.assert("filesystem devt", fs.devt() == Some(devt));
        assertions.assert("filesystem ready", fs.ready());
        assertions.assert("filesystem mount boundary", fs.mount_boundary_recorded());
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
        assertions.assert("vfs deferred", fs.vfs_integration_deferred());
        assertions.assert("page cache deferred", fs.page_cache_deferred());
        assertions.assert("writes deferred", fs.write_paths_deferred());

        let root = fs.root_inode();
        assertions.assert("root ino", root.ino() == EXT2_ROOT_INO);
        assertions.assert("root dir", root.is_root_dir());
        assertions.assert("root direct block", root.direct_blocks()[0] != 0);
        assertions.assert("root indirect deferred", root.indirect_blocks_deferred());

        let lookup = fs.lookup_root(
            &mut ctx.block_device_registry,
            &mut provider,
            EXT2_SMOKE_FILE_NAME,
        );
        assertions.assert_ok("lookup smoke file", lookup);
        let dirent = fs.lookup_dirent();
        assertions.assert("lookup name", fs.lookup_name_bound());
        assertions.assert("lookup reads root", fs.lookup_reads_root_dir());
        assertions.assert("dirent valid", fs.lookup_dirent_valid());
        assertions.assert("dirent name", dirent.name() == EXT2_SMOKE_FILE_NAME);
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

        let mut buffer = [0u8; 64];
        let read = fs.read_lookup_file(&mut ctx.block_device_registry, &mut provider, &mut buffer);
        let Ok(len) = read else {
            assertions.assert("read file", false);
            return;
        };
        assertions.assert("read len", len == EXT2_SMOKE_FILE_CONTENT.len());
        assertions.assert("read content", &buffer[..len] == EXT2_SMOKE_FILE_CONTENT);
        assertions.assert("last read len", fs.last_file_read_len() == len);
        assertions.assert("read direct", fs.file_read_uses_direct_block());
        assertions.assert("read buffer head", fs.file_read_uses_buffer_head());
        assertions.assert("read copies", fs.file_read_copies_to_caller());
        assertions.assert(
            "read len matches inode",
            fs.file_read_len_matches_inode_size(),
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
