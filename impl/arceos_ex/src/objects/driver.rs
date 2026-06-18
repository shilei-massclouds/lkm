use super::{
    config::Config,
    device::DeviceRef,
    device_tree::{DeviceNodeId, DeviceTree},
    ioremap::Ioremap,
    irq_time::{IrqHandlerRegistry, PlicIrqDomain},
    mm_core::{PageAllocator, PageMetadataMap, PageTableCaches, VmallocAllocator},
    virtio::VirtioBus,
};

pub struct PlatformProbeResources<'a> {
    pub device_tree: &'a DeviceTree,
    pub vmalloc_allocator: &'a mut VmallocAllocator,
    pub page_table_caches: &'a mut PageTableCaches,
    pub page_allocator: &'a mut PageAllocator,
    pub page_metadata_map: &'a PageMetadataMap,
    pub config: &'a Config,
    pub ioremap: &'a mut Ioremap,
    pub plic_irq_domain: &'a mut PlicIrqDomain,
    pub irq_handler_registry: &'a mut IrqHandlerRegistry,
    pub virtio_bus: &'a mut VirtioBus,
}

pub type PlatformProbe =
    for<'a> fn(&mut PlatformProbeResources<'a>, DeviceRef, DeviceNodeId) -> ProbeResult;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ProbeResult {
    Bound,
    Deferred,
    Rejected,
}

pub struct OfMatchEntry {
    compatible: &'static [u8],
}

impl OfMatchEntry {
    pub const fn new(compatible: &'static [u8]) -> Self {
        Self { compatible }
    }

    pub const fn compatible(&self) -> &'static [u8] {
        self.compatible
    }
}

pub struct OfMatchTable {
    entries: &'static [OfMatchEntry],
}

impl OfMatchTable {
    pub const fn new(entries: &'static [OfMatchEntry]) -> Self {
        Self { entries }
    }

    pub const fn empty() -> Self {
        Self { entries: &[] }
    }

    pub fn matches(&self, compatible: &[u8]) -> bool {
        let mut index = 0usize;
        while index < self.entries.len() {
            if self.entries[index].compatible() == compatible {
                return true;
            }
            index += 1;
        }
        false
    }
}

pub struct DeviceDriver {
    name: &'static str,
    of_match_table: OfMatchTable,
}

impl DeviceDriver {
    pub const fn new(name: &'static str, of_match_table: OfMatchTable) -> Self {
        Self {
            name,
            of_match_table,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn of_match_table(&self) -> &OfMatchTable {
        &self.of_match_table
    }
}

pub struct PlatformDriver {
    driver: DeviceDriver,
    probe: PlatformProbe,
}

impl PlatformDriver {
    pub const fn new(
        name: &'static str,
        of_match_table: OfMatchTable,
        probe: PlatformProbe,
    ) -> Self {
        Self {
            driver: DeviceDriver::new(name, of_match_table),
            probe,
        }
    }

    pub const fn driver(&self) -> &DeviceDriver {
        &self.driver
    }

    pub fn probe(
        &self,
        resources: &mut PlatformProbeResources<'_>,
        device: DeviceRef,
        node_id: DeviceNodeId,
    ) -> ProbeResult {
        (self.probe)(resources, device, node_id)
    }
}

#[derive(Clone, Copy)]
pub struct DeviceDriverRef {
    driver: &'static PlatformDriver,
}

impl DeviceDriverRef {
    pub const fn new(driver: &'static PlatformDriver) -> Self {
        Self { driver }
    }

    pub const fn driver(self) -> &'static PlatformDriver {
        self.driver
    }
}

impl PartialEq for DeviceDriverRef {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.driver, other.driver)
    }
}

impl Eq for DeviceDriverRef {}

pub static MOCK_PLATFORM_DRIVER: PlatformDriver = PlatformDriver::new(
    "mock_platform_driver",
    OfMatchTable::empty(),
    mock_deferred_probe,
);

pub const MOCK_PLATFORM_DRIVER_REF: DeviceDriverRef = DeviceDriverRef::new(&MOCK_PLATFORM_DRIVER);

fn mock_deferred_probe(
    _resources: &mut PlatformProbeResources<'_>,
    _device: DeviceRef,
    _node_id: DeviceNodeId,
) -> ProbeResult {
    ProbeResult::Deferred
}
