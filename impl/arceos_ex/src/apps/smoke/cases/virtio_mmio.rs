use crate::{
    apps::smoke::{
        harness::{SmokeAssertions, SmokeScenario, SmokeSuite},
        SmokeResult,
    },
    context::context_ref,
    objects::{
        driver::ProbeResult,
        virtio_mmio::{
            self, VirtioMmioHeader, VirtioMmioHeaderStatus, VIRTIO_ID_RNG,
            VIRTIO_MMIO_PLATFORM_DRIVER_REF,
        },
    },
};

const VIRTIO_MMIO_VENDOR_QEMU: u32 = u32::from_le_bytes(*b"QEMU");

pub fn run() -> SmokeResult {
    let mut suite = SmokeSuite::new();
    suite.scenario(&mut HeaderClassifierScenario);
    suite.scenario(&mut PlatformProbeObservationScenario);
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

struct PlatformProbeObservationScenario;

impl SmokeScenario for PlatformProbeObservationScenario {
    fn name(&self) -> &'static str {
        "virtio_mmio.platform_probe_observation"
    }

    fn setup(&mut self, _assertions: &mut SmokeAssertions) {}

    fn run(&mut self, assertions: &mut SmokeAssertions) {
        let ctx = context_ref();
        let summary = virtio_mmio::probe_summary();
        assertions.assert("probe called", summary.probe_called());
        assertions.assert("matched devices", summary.matched_devices() != 0);
        assertions.assert(
            "driver registered",
            ctx.platform_bus
                .platform_driver_registered(VIRTIO_MMIO_PLATFORM_DRIVER_REF),
        );
        assertions.assert(
            "driver ref identity",
            virtio_mmio::is_virtio_mmio_platform_driver(VIRTIO_MMIO_PLATFORM_DRIVER_REF),
        );
        assertions.assert(
            "summary classified all matches",
            summary.transport_count()
                + summary.placeholder_count()
                + summary.invalid_magic_count()
                + summary.unsupported_version_count()
                == summary.matched_devices(),
        );
        assertions.assert("no rng device yet", summary.rng_candidate_count() == 0);

        let mut observed_match = false;
        let mut observed_probe = false;
        let mut index = 0usize;
        while index < ctx.platform_bus.klist_device_count() {
            let Some(device_ref) = ctx.platform_bus.klist_device_ref(index) else {
                break;
            };
            if ctx
                .platform_bus
                .platform_match_attempted(VIRTIO_MMIO_PLATFORM_DRIVER_REF, device_ref)
            {
                observed_match = true;
            }
            if ctx
                .platform_bus
                .platform_probe_called(VIRTIO_MMIO_PLATFORM_DRIVER_REF, device_ref)
            {
                observed_probe = true;
                assertions.assert(
                    "probe result recorded",
                    ctx.platform_bus
                        .platform_probe_result(VIRTIO_MMIO_PLATFORM_DRIVER_REF, device_ref)
                        != Some(ProbeResult::Deferred),
                );
            }
            index += 1;
        }
        assertions.assert("match observed", observed_match);
        assertions.assert("probe observed", observed_probe);

        let transport = summary.last_transport();
        assertions.assert(
            "resource parsed",
            transport.mapbase() != 0 && transport.mapsize() != 0,
        );
        assertions.assert(
            "device ref exists",
            ctx.platform_bus
                .platform_device(transport.device_ref())
                .is_some(),
        );
        assertions.assert("node id valid", transport.node_id().index() != usize::MAX);
        assertions.assert("ioremap attempted", transport.ioremapped());
        assertions.assert("membase recorded", transport.membase() != 0);
        assertions.assert("irq resource parsed", transport.irq_source().is_some());
        assertions.assert(
            "header read",
            transport.magic() == u32::from_le_bytes(*b"virt"),
        );
        assertions.assert(
            "version supported",
            transport.version() == 1 || transport.version() == 2,
        );
        assertions.assert(
            "vendor read",
            transport.vendor_id() != 0 || transport.placeholder_device(),
        );
        assertions.assert(
            "header classified",
            transport.header_valid() || transport.placeholder_device(),
        );
    }

    fn teardown(&mut self, _assertions: &mut SmokeAssertions) {}
}
