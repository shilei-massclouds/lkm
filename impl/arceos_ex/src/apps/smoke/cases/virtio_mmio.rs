use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    objects::virtio_mmio::{VirtioMmioHeader, VirtioMmioHeaderStatus, VIRTIO_ID_RNG},
};

const VIRTIO_MMIO_VENDOR_QEMU: u32 = u32::from_le_bytes(*b"QEMU");

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut HeaderClassifierScenario);
    suite.result()
}

struct HeaderClassifierScenario;

impl SmokeScenario for HeaderClassifierScenario {
    fn name(&self) -> &'static str {
        "virtio_mmio.header_classifier"
    }

    fn setup(&mut self, _assertions: &mut SmokeAssertions) {}

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let valid_rng = VirtioMmioHeader::valid_device(VIRTIO_ID_RNG, VIRTIO_MMIO_VENDOR_QEMU);
        assertions.assert("valid rng header", valid_rng.valid());
        assertions.assert("valid rng candidate", valid_rng.rng_candidate());
        assertions.assert("valid rng id", valid_rng.device_id() == VIRTIO_ID_RNG);
        assertions.assert("valid rng version", valid_rng.version() == 2);
        assertions.assert(
            "valid rng vendor",
            valid_rng.vendor_id() == VIRTIO_MMIO_VENDOR_QEMU,
        );

        let invalid_magic = VirtioMmioHeader::new(0, 2, 4, 1);
        assertions.assert(
            "invalid magic",
            invalid_magic.status() == VirtioMmioHeaderStatus::InvalidMagic,
        );
        assertions.assert("invalid magic not rng", !invalid_magic.rng_candidate());

        let unsupported_version = VirtioMmioHeader::new(u32::from_le_bytes(*b"virt"), 3, 4, 1);
        assertions.assert(
            "unsupported version",
            unsupported_version.status() == VirtioMmioHeaderStatus::UnsupportedVersion,
        );

        let placeholder = VirtioMmioHeader::new(u32::from_le_bytes(*b"virt"), 2, 0, 1);
        assertions.assert("placeholder", placeholder.placeholder_device());
        assertions.assert("placeholder not rng", !placeholder.rng_candidate());
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
