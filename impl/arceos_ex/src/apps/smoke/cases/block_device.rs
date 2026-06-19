use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context,
    objects::{
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
        "block_device.read_default_and_devt"
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
        let mut default_buf = [0u8; 512];
        let mut devt_buf = [0u8; 512];

        let ctx = context();
        let Some(default_entry) = ctx.block_device_registry.default_entry() else {
            assertions.assert("default entry", false);
            return;
        };
        let devt = default_entry.devt();
        assertions.assert(
            "provider virtio blk",
            default_entry.provider_kind() == BlockDeviceProviderKind::VirtioBlk,
        );
        assertions.assert("major", devt.major() == VIRTBLK_MAJOR);
        assertions.assert("minor", devt.minor() == VIRTBLK_FIRST_MINOR);

        let mut provider = virtio_blk::live_provider(&ctx.kernel_image);
        let default_len = match ctx.block_device_registry.read_default(
            &mut provider,
            EXT2_SUPERBLOCK_SECTOR,
            &mut default_buf,
        ) {
            Ok(len) => len,
            Err(_) => {
                assertions.assert("read default", false);
                return;
            }
        };
        assertions.assert("default len", default_len == default_buf.len());
        assertions.assert("default nonzero", default_buf.iter().any(|byte| *byte != 0));
        assertions.assert("default ext2", ext2_magic_observed(&default_buf));
        assertions.assert(
            "default read recorded",
            ctx.block_device_registry.read_default_count() == 1
                && ctx.block_device_registry.default_device_ref_acquired(),
        );

        let devt_len = match ctx.block_device_registry.read_by_devt(
            &mut provider,
            devt,
            EXT2_SUPERBLOCK_SECTOR,
            &mut devt_buf,
        ) {
            Ok(len) => len,
            Err(_) => {
                assertions.assert("read devt", false);
                return;
            }
        };
        assertions.assert("devt len", devt_len == devt_buf.len());
        assertions.assert("devt nonzero", devt_buf.iter().any(|byte| *byte != 0));
        assertions.assert("devt ext2", ext2_magic_observed(&devt_buf));
        assertions.assert(
            "devt read recorded",
            ctx.block_device_registry.read_by_devt_count() == 1
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
                && ctx.block_device_registry.last_read_len() == devt_buf.len(),
        );

        let Some(device) = ctx.virtio_blk_runtime.device() else {
            assertions.assert("virtio blk device", false);
            return;
        };
        assertions.assert("block served", device.block_read_served());
        assertions.assert("block copied", device.block_read_copies_to_caller());
        assertions.assert("blk request count", device.request_count() >= 3);
        assertions.assert("blk completion count", device.completion_count() >= 3);
        assertions.assert(
            "blk last sector",
            device.last_sector() == EXT2_SUPERBLOCK_SECTOR,
        );
        assertions.assert("blk read done", device.read_request_done());
        assertions.assert("blk status", device.last_status() == 0);

        let block = device.block_device();
        assertions.assert("block read count", block.read_count() == 2);
        assertions.assert(
            "block read sector",
            block.last_read_sector() == EXT2_SUPERBLOCK_SECTOR,
        );
        assertions.assert("block read len", block.last_read_len() == devt_buf.len());
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
