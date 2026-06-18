use super::{
    config::Config,
    device::DeviceRef,
    device_tree::{DeviceNodeId, DeviceTree},
    ioremap::{IoMemoryMapping, Ioremap},
    irq_time::{
        IrqAction, IrqHandlerKind, IrqHandlerRegistry, LogicalIrq, PlicIrqDomain, PlicIrqMapping,
    },
    mm_core::{PageAllocator, PageMetadataMap, PageTableCaches, VmallocAllocator},
    virtio::VirtioBus,
    virtio_mmio::VirtioMmioTransportDevice,
};

pub struct PlatformProbeContext<'a> {
    device_tree: &'a DeviceTree,
    vmalloc_allocator: &'a mut VmallocAllocator,
    page_table_caches: &'a mut PageTableCaches,
    page_allocator: &'a mut PageAllocator,
    page_metadata_map: &'a PageMetadataMap,
    config: &'a Config,
    ioremap: &'a mut Ioremap,
    plic_irq_domain: &'a mut PlicIrqDomain,
    irq_handler_registry: &'a mut IrqHandlerRegistry,
    virtio_bus: &'a mut VirtioBus,
}

impl<'a> PlatformProbeContext<'a> {
    pub fn new(
        device_tree: &'a DeviceTree,
        vmalloc_allocator: &'a mut VmallocAllocator,
        page_table_caches: &'a mut PageTableCaches,
        page_allocator: &'a mut PageAllocator,
        page_metadata_map: &'a PageMetadataMap,
        config: &'a Config,
        ioremap: &'a mut Ioremap,
        plic_irq_domain: &'a mut PlicIrqDomain,
        irq_handler_registry: &'a mut IrqHandlerRegistry,
        virtio_bus: &'a mut VirtioBus,
    ) -> Self {
        Self {
            device_tree,
            vmalloc_allocator,
            page_table_caches,
            page_allocator,
            page_metadata_map,
            config,
            ioremap,
            plic_irq_domain,
            irq_handler_registry,
            virtio_bus,
        }
    }

    pub const fn device_tree(&self) -> &DeviceTree {
        self.device_tree
    }

    pub fn map_platform_device_mmio(
        &mut self,
        device: DeviceRef,
        phys_base: usize,
        size: usize,
    ) -> Option<IoMemoryMapping> {
        self.ioremap.map_device_mmio(
            self.vmalloc_allocator,
            self.page_table_caches,
            self.page_allocator,
            self.page_metadata_map,
            self.config,
            device,
            phys_base,
            size,
        )
    }

    pub fn ioremap_has_platform_device_mapping(&self, device: DeviceRef) -> bool {
        self.ioremap.mapping_count() != 0 && self.ioremap.mapping_for_device(device).is_some()
    }

    pub fn map_plic_source(&mut self, source: u32) -> Option<LogicalIrq> {
        self.plic_irq_domain.map_source(source)
    }

    pub fn translate_plic_one_cell_specifier(&self, specifier: &[u32]) -> Option<u32> {
        self.plic_irq_domain.translate_one_cell_specifier(specifier)
    }

    pub fn plic_mapping_for_source(&self, source: u32) -> Option<&PlicIrqMapping> {
        self.plic_irq_domain.mapping_for_source(source)
    }

    pub fn request_irq(
        &mut self,
        logical_irq: LogicalIrq,
        device: DeviceRef,
        handler_kind: IrqHandlerKind,
    ) -> bool {
        self.irq_handler_registry.request_irq(
            self.plic_irq_domain,
            logical_irq,
            device,
            handler_kind,
        )
    }

    pub fn irq_action_for_logical_irq(&self, logical_irq: LogicalIrq) -> Option<&IrqAction> {
        self.irq_handler_registry
            .action_for_logical_irq(logical_irq)
    }

    pub fn register_virtio_mmio_device(
        &mut self,
        transport: VirtioMmioTransportDevice,
    ) -> Option<super::virtio::VirtioDeviceRef> {
        self.virtio_bus.register_mmio_device(transport)
    }
}

pub type PlatformProbe =
    for<'a> fn(&mut PlatformProbeContext<'a>, DeviceRef, DeviceNodeId) -> ProbeResult;

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
        context: &mut PlatformProbeContext<'_>,
        device: DeviceRef,
        node_id: DeviceNodeId,
    ) -> ProbeResult {
        (self.probe)(context, device, node_id)
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
    _context: &mut PlatformProbeContext<'_>,
    _device: DeviceRef,
    _node_id: DeviceNodeId,
) -> ProbeResult {
    ProbeResult::Deferred
}
