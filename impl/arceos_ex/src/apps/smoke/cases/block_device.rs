use crate::{
    apps::smoke::{
        SmokeResult,
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
    },
    context::context,
    objects::{
        bio::{self, BUFFER_HEAD_SECTOR_SIZE, BioSubmitPath},
        block_device::{BlockDeviceProviderKind, VIRTBLK_FIRST_MINOR, VIRTBLK_MAJOR},
        state::State,
        virtio_blk,
    },
};

const EXT2_SUPERBLOCK_SECTOR: u64 = 2;
const EXT2_SUPER_MAGIC_OFFSET_IN_SECTOR: usize = 56;
const EXT2_SUPER_MAGIC: u16 = 0xef53;

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut BlockDeviceReadScenario::new());
    suite.result()
}

struct BlockDeviceReadScenario;

impl BlockDeviceReadScenario {
    const fn new() -> Self {
        Self
    }
}

impl SmokeScenario for BlockDeviceReadScenario {
    fn name(&self) -> &'static str {
        "block_io.buffer_head_read_default_and_devt"
    }

    fn setup(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context();
        assertions.assert(
            "registry ready",
            ctx.block_device_registry.state() == State::Ready,
        );
        assertions.assert(
            "device count",
            ctx.block_device_registry.device_count() == 1,
        );
        assertions.assert(
            "default present",
            ctx.block_device_registry.default_entry().is_some(),
        );
        assertions.assert(
            "major lookup",
            ctx.block_device_registry.major_minor_lookup_ready(),
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
        let default_device_ref = default_entry.device_ref();
        let provider_kind = default_entry.provider_kind();
        assertions.assert(
            "provider virtio blk",
            provider_kind == BlockDeviceProviderKind::VirtioBlk,
        );
        assertions.assert("major", devt.major() == VIRTBLK_MAJOR);
        assertions.assert("minor", devt.minor() == VIRTBLK_FIRST_MINOR);

        let mut provider = virtio_blk::live_provider(&ctx.kernel_image);
        let default_read_count_before = ctx.block_device_registry.read_default_count();
        let devt_read_count_before = ctx.block_device_registry.read_by_devt_count();
        let Some(device_before) = ctx.virtio_blk_runtime.device() else {
            assertions.assert("virtio blk device before", false);
            return;
        };
        let request_count_before = device_before.request_count();
        let completion_count_before = device_before.completion_count();
        let block_read_count_before = device_before.block_device().read_count();

        let default_bh = match bio::sb_bread_default(
            &mut ctx.block_device_registry,
            &mut provider,
            EXT2_SUPERBLOCK_SECTOR,
        ) {
            Ok(bh) => bh,
            Err(_) => {
                assertions.assert("sb_bread default", false);
                return;
            }
        };
        assertions.assert("default bh ready", default_bh.state() == State::Ready);
        assertions.assert(
            "default bh len",
            default_bh.len() == BUFFER_HEAD_SECTOR_SIZE,
        );
        assertions.assert("default bh uptodate", default_bh.uptodate());
        assertions.assert("default bh nonzero", default_bh.data_nonzero());
        assertions.assert("default bh ext2", ext2_magic_observed(default_bh.data()));
        assertions.assert(
            "default bh sector",
            default_bh.sector() == EXT2_SUPERBLOCK_SECTOR,
        );
        assertions.assert("default bh devt", default_bh.devt() == devt);
        assertions.assert(
            "default bh device",
            default_bh.device_ref() == default_device_ref,
        );
        assertions.assert(
            "default bh block size",
            default_bh.block_size() == BUFFER_HEAD_SECTOR_SIZE,
        );
        assertions.assert("default sb_bread", default_bh.sb_bread_called());
        assertions.assert("default bread_gfp", default_bh.bread_gfp_called());
        assertions.assert("default submit_bio", default_bh.submit_bio_wait_used());
        assertions.assert(
            "default bio path",
            default_bh.bio().submit_path() == BioSubmitPath::DefaultBlockDevice,
        );
        assertions.assert("default bio submitted", default_bh.bio().submitted());
        assertions.assert(
            "default bio completion",
            default_bh.bio().completion_observed(),
        );
        assertions.assert("default bio status", default_bh.bio().status_ok());
        assertions.assert("default bio copies", default_bh.bio().copies_to_caller());
        assertions.assert(
            "default bio blk mq",
            default_bh.bio().blk_mq_submit_bio_entered(),
        );
        assertions.assert(
            "default bio device",
            default_bh.bio().device_ref() == default_device_ref,
        );
        assertions.assert("default bio devt", default_bh.bio().devt() == devt);
        assertions.assert(
            "default bio sector",
            default_bh.bio().sector() == EXT2_SUPERBLOCK_SECTOR,
        );
        assertions.assert(
            "default bio len",
            default_bh.bio().len() == BUFFER_HEAD_SECTOR_SIZE,
        );
        assertions.assert("default bio buffer", default_bh.bio().buffer_bound());
        assertions.assert(
            "default read recorded",
            ctx.block_device_registry.read_default_count()
                == default_read_count_before.saturating_add(1)
                && ctx.block_device_registry.default_device_ref_acquired(),
        );

        let devt_bh = match bio::sb_bread_by_devt(
            &mut ctx.block_device_registry,
            &mut provider,
            devt,
            EXT2_SUPERBLOCK_SECTOR,
        ) {
            Ok(bh) => bh,
            Err(_) => {
                assertions.assert("sb_bread devt", false);
                return;
            }
        };
        assertions.assert("devt bh ready", devt_bh.state() == State::Ready);
        assertions.assert("devt bh len", devt_bh.len() == BUFFER_HEAD_SECTOR_SIZE);
        assertions.assert("devt bh uptodate", devt_bh.uptodate());
        assertions.assert("devt bh nonzero", devt_bh.data_nonzero());
        assertions.assert("devt bh ext2", ext2_magic_observed(devt_bh.data()));
        assertions.assert("devt bh sector", devt_bh.sector() == EXT2_SUPERBLOCK_SECTOR);
        assertions.assert("devt bh devt", devt_bh.devt() == devt);
        assertions.assert(
            "devt bh block size",
            devt_bh.block_size() == BUFFER_HEAD_SECTOR_SIZE,
        );
        assertions.assert("devt submit_bio", devt_bh.submit_bio_wait_used());
        assertions.assert(
            "devt bio path",
            devt_bh.bio().submit_path() == BioSubmitPath::MajorMinorLookup,
        );
        assertions.assert("devt bio submitted", devt_bh.bio().submitted());
        assertions.assert("devt bio completion", devt_bh.bio().completion_observed());
        assertions.assert("devt bio status", devt_bh.bio().status_ok());
        assertions.assert("devt bio copies", devt_bh.bio().copies_to_caller());
        assertions.assert(
            "devt bio device",
            devt_bh.bio().device_ref() == default_device_ref,
        );
        assertions.assert("devt bio devt", devt_bh.bio().devt() == devt);
        assertions.assert(
            "devt bio sector",
            devt_bh.bio().sector() == EXT2_SUPERBLOCK_SECTOR,
        );
        assertions.assert("devt bio buffer", devt_bh.bio().buffer_bound());
        assertions.assert(
            "devt read recorded",
            ctx.block_device_registry.read_by_devt_count()
                == devt_read_count_before.saturating_add(1)
                && ctx.block_device_registry.major_minor_ref_acquired(),
        );
        assertions.assert(
            "registry read invokes provider",
            ctx.block_device_registry.read_invokes_provider(),
        );
        assertions.assert(
            "registry read completion",
            ctx.block_device_registry.read_completion_observed(),
        );
        assertions.assert(
            "registry copies",
            ctx.block_device_registry.read_copies_to_caller(),
        );
        assertions.assert(
            "registry nonzero",
            ctx.block_device_registry.read_returns_nonzero(),
        );
        assertions.assert(
            "registry last read",
            ctx.block_device_registry.last_read_sector() == EXT2_SUPERBLOCK_SECTOR
                && ctx.block_device_registry.last_read_len() == devt_bh.len(),
        );

        let Some(device) = ctx.virtio_blk_runtime.device() else {
            assertions.assert("virtio blk device", false);
            return;
        };
        assertions.assert("block served", device.block_read_served());
        assertions.assert("block copied", device.block_read_copies_to_caller());
        assertions.assert(
            "blk request count",
            device.request_count() == request_count_before.saturating_add(2),
        );
        assertions.assert(
            "blk completion count",
            device.completion_count() == completion_count_before.saturating_add(2),
        );
        assertions.assert(
            "blk last sector",
            device.last_sector() == EXT2_SUPERBLOCK_SECTOR,
        );
        assertions.assert("blk read done", device.read_request_done());
        assertions.assert("blk status", device.last_status() == 0);

        let block = device.block_device();
        assertions.assert(
            "block read count",
            block.read_count() == block_read_count_before.saturating_add(2),
        );
        assertions.assert(
            "block read sector",
            block.last_read_sector() == EXT2_SUPERBLOCK_SECTOR,
        );
        assertions.assert("block read len", block.last_read_len() == devt_bh.len());
        assertions.assert("block submitted", block.read_submitted());
        assertions.assert("block completion", block.read_completion_observed());
        assertions.assert("block copies", block.read_copies_to_caller());
        assertions.assert("block nonzero", block.read_returns_nonzero());
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}

fn ext2_magic_observed(buffer: &[u8]) -> bool {
    let offset = EXT2_SUPER_MAGIC_OFFSET_IN_SECTOR;
    offset + 1 < buffer.len()
        && u16::from_le_bytes([buffer[offset], buffer[offset + 1]]) == EXT2_SUPER_MAGIC
}
