use super::{
    config::Config,
    device::DeviceRef,
    device_tree::{DeviceNodeId, DeviceTree},
    ioremap::Ioremap,
    irq_time::{IrqHandlerRegistry, PlicIrqDomain},
    mm_core::{PageAllocator, PageMetadataMap, PageTableCaches, VmallocAllocator},
};

pub type PlatformProbe = fn(
    &DeviceTree,
    &mut VmallocAllocator,
    &mut PageTableCaches,
    &mut PageAllocator,
    &PageMetadataMap,
    &Config,
    &mut Ioremap,
    &mut PlicIrqDomain,
    &mut IrqHandlerRegistry,
    DeviceRef,
    DeviceNodeId,
) -> ProbeResult;

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
        device_tree: &DeviceTree,
        vmalloc_allocator: &mut VmallocAllocator,
        page_table_caches: &mut PageTableCaches,
        page_allocator: &mut PageAllocator,
        page_metadata_map: &PageMetadataMap,
        config: &Config,
        ioremap: &mut Ioremap,
        plic_irq_domain: &mut PlicIrqDomain,
        irq_handler_registry: &mut IrqHandlerRegistry,
        device: DeviceRef,
        node_id: DeviceNodeId,
    ) -> ProbeResult {
        (self.probe)(
            device_tree,
            vmalloc_allocator,
            page_table_caches,
            page_allocator,
            page_metadata_map,
            config,
            ioremap,
            plic_irq_domain,
            irq_handler_registry,
            device,
            node_id,
        )
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
    _device_tree: &DeviceTree,
    _vmalloc_allocator: &mut VmallocAllocator,
    _page_table_caches: &mut PageTableCaches,
    _page_allocator: &mut PageAllocator,
    _page_metadata_map: &PageMetadataMap,
    _config: &Config,
    _ioremap: &mut Ioremap,
    _plic_irq_domain: &mut PlicIrqDomain,
    _irq_handler_registry: &mut IrqHandlerRegistry,
    _device: DeviceRef,
    _node_id: DeviceNodeId,
) -> ProbeResult {
    ProbeResult::Deferred
}
