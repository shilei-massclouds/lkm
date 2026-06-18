use super::{
    device::DeviceRef,
    device_tree::{DeviceNodeId, DeviceTree},
    driver::{
        DeviceDriverRef, OfMatchEntry, OfMatchTable, PlatformDriver, PlatformProbeContext,
        ProbeResult,
    },
    fdt_reader::{read_be_u32, read_cells},
    initcall::{ContextRef, InitcallReturn},
    ioremap::IoMemoryMapping,
    irq_time::{IrqHandlerKind, LogicalIrq},
};
use crate::{checkpoint, trace::Checkpoint};
use core::sync::atomic::{AtomicUsize, Ordering};

const VIRTIO_MMIO_OF_MATCH: [OfMatchEntry; 1] = [OfMatchEntry::new(b"virtio,mmio")];
const VIRTIO_MMIO_MAGIC: u32 = u32::from_le_bytes(*b"virt");
const VIRTIO_MMIO_VERSION_MIN: u32 = 1;
const VIRTIO_MMIO_VERSION_MAX: u32 = 2;
pub const VIRTIO_ID_RNG: u32 = 4;
pub const VIRTIO_MMIO_GUEST_PAGE_SIZE: usize = 4096;
pub const VIRTIO_MMIO_VRING_ALIGN: usize = 4096;
pub const VIRTIO_MMIO_INT_VRING: u32 = 1 << 0;
const VIRTIO_MMIO_DEVICE_FEATURES: usize = 0x010;
const VIRTIO_MMIO_DEVICE_FEATURES_SEL: usize = 0x014;
const VIRTIO_MMIO_DRIVER_FEATURES: usize = 0x020;
const VIRTIO_MMIO_DRIVER_FEATURES_SEL: usize = 0x024;
const VIRTIO_MMIO_GUEST_PAGE_SIZE_REG: usize = 0x028;
const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030;
const VIRTIO_MMIO_QUEUE_NUM_MAX: usize = 0x034;
const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038;
const VIRTIO_MMIO_QUEUE_ALIGN: usize = 0x03c;
const VIRTIO_MMIO_QUEUE_PFN: usize = 0x040;
const VIRTIO_MMIO_QUEUE_READY: usize = 0x044;
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050;
const VIRTIO_MMIO_INTERRUPT_STATUS: usize = 0x060;
const VIRTIO_MMIO_INTERRUPT_ACK: usize = 0x064;
const VIRTIO_MMIO_STATUS: usize = 0x070;
const VIRTIO_MMIO_QUEUE_DESC_LOW: usize = 0x080;
const VIRTIO_MMIO_QUEUE_DESC_HIGH: usize = 0x084;
const VIRTIO_MMIO_QUEUE_AVAIL_LOW: usize = 0x090;
const VIRTIO_MMIO_QUEUE_AVAIL_HIGH: usize = 0x094;
const VIRTIO_MMIO_QUEUE_USED_LOW: usize = 0x0a0;
const VIRTIO_MMIO_QUEUE_USED_HIGH: usize = 0x0a4;
const VIRTIO_CONFIG_S_ACKNOWLEDGE: u32 = 1;
const VIRTIO_CONFIG_S_DRIVER: u32 = 2;
const VIRTIO_CONFIG_S_DRIVER_OK: u32 = 4;
const VIRTIO_CONFIG_S_FEATURES_OK: u32 = 8;

static VIRTIO_MMIO_IRQ_HANDLER_CALLS: AtomicUsize = AtomicUsize::new(0);

pub static VIRTIO_MMIO_PLATFORM_DRIVER: PlatformDriver = PlatformDriver::new(
    "virtio-mmio",
    OfMatchTable::new(&VIRTIO_MMIO_OF_MATCH),
    virtio_mmio_probe,
);

pub const VIRTIO_MMIO_PLATFORM_DRIVER_REF: DeviceDriverRef =
    DeviceDriverRef::new(&VIRTIO_MMIO_PLATFORM_DRIVER);

pub fn virtio_mmio_platform_driver_init(ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: virtio_mmio_platform_driver_init\n");
    let result = ctx.platform_driver_register(VIRTIO_MMIO_PLATFORM_DRIVER_REF);
    if result == InitcallReturn::Ok && ctx.virtio_bus.device_count() != 0 {
        checkpoint::dispatch(Checkpoint::VirtioBusDeviceAdded, ctx);
    }
    result
}

crate::device_initcall!(virtio_mmio_platform_driver_init);

#[allow(dead_code)]
pub fn is_virtio_mmio_platform_driver(driver: DeviceDriverRef) -> bool {
    driver == VIRTIO_MMIO_PLATFORM_DRIVER_REF
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum VirtioMmioHeaderStatus {
    Unknown,
    Valid,
    InvalidMagic,
    UnsupportedVersion,
    PlaceholderDevice,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct VirtioMmioTransportDevice {
    device_ref: DeviceRef,
    node_id: DeviceNodeId,
    mapbase: usize,
    mapsize: usize,
    membase: usize,
    irq_source: Option<u32>,
    logical_irq: LogicalIrq,
    irq_mapping_ready: bool,
    irq_handler_registered: bool,
    irq_source_gate_open: bool,
    ioremapped: bool,
    vm_ioremap: bool,
    io_page_protection: bool,
    header_status: VirtioMmioHeaderStatus,
    magic: u32,
    version: u32,
    device_id: u32,
    vendor_id: u32,
    rng_candidate: bool,
}

#[allow(dead_code)]
impl VirtioMmioTransportDevice {
    const fn empty() -> Self {
        Self {
            device_ref: DeviceRef::new(usize::MAX),
            node_id: DeviceNodeId::invalid(),
            mapbase: 0,
            mapsize: 0,
            membase: 0,
            irq_source: None,
            logical_irq: LogicalIrq::invalid(),
            irq_mapping_ready: false,
            irq_handler_registered: false,
            irq_source_gate_open: false,
            ioremapped: false,
            vm_ioremap: false,
            io_page_protection: false,
            header_status: VirtioMmioHeaderStatus::Unknown,
            magic: 0,
            version: 0,
            device_id: 0,
            vendor_id: 0,
            rng_candidate: false,
        }
    }

    pub const fn device_ref(self) -> DeviceRef {
        self.device_ref
    }

    pub const fn node_id(self) -> DeviceNodeId {
        self.node_id
    }

    pub const fn mapbase(self) -> usize {
        self.mapbase
    }

    pub const fn mapsize(self) -> usize {
        self.mapsize
    }

    pub const fn membase(self) -> usize {
        self.membase
    }

    pub const fn irq_source(self) -> Option<u32> {
        self.irq_source
    }

    pub const fn logical_irq(self) -> LogicalIrq {
        self.logical_irq
    }

    pub const fn irq_mapping_ready(self) -> bool {
        self.irq_mapping_ready
    }

    pub const fn irq_handler_registered(self) -> bool {
        self.irq_handler_registered
    }

    pub const fn irq_source_gate_open(self) -> bool {
        self.irq_source_gate_open
    }

    pub const fn ioremapped(self) -> bool {
        self.ioremapped
    }

    pub const fn vm_ioremap(self) -> bool {
        self.vm_ioremap
    }

    pub const fn io_page_protection(self) -> bool {
        self.io_page_protection
    }

    pub const fn header_valid(self) -> bool {
        matches!(self.header_status, VirtioMmioHeaderStatus::Valid)
    }

    pub const fn header_status(self) -> VirtioMmioHeaderStatus {
        self.header_status
    }

    pub const fn placeholder_device(self) -> bool {
        matches!(
            self.header_status,
            VirtioMmioHeaderStatus::PlaceholderDevice
        )
    }

    pub const fn magic(self) -> u32 {
        self.magic
    }

    pub const fn version(self) -> u32 {
        self.version
    }

    pub const fn device_id(self) -> u32 {
        self.device_id
    }

    pub const fn vendor_id(self) -> u32 {
        self.vendor_id
    }

    pub const fn rng_candidate(self) -> bool {
        self.rng_candidate
    }

    pub const fn ready_for_virtio_core(self) -> bool {
        self.header_valid()
            && self.ioremapped
            && self.mapsize != 0
            && self.membase != 0
            && self.irq_mapping_ready
            && self.irq_handler_registered
    }

    fn with_header(mut self, header: VirtioMmioHeader) -> Self {
        self.magic = header.magic;
        self.version = header.version;
        self.device_id = header.device_id;
        self.vendor_id = header.vendor_id;
        self.header_status = classify_header(header);
        self.rng_candidate =
            self.header_status == VirtioMmioHeaderStatus::Valid && self.device_id == VIRTIO_ID_RNG;
        self
    }

    fn bind_ioremap(mut self, mapping: IoMemoryMapping) -> Self {
        self.membase = mapping.membase();
        self.ioremapped = mapping.membase_cookie_ready()
            && mapping.phys_base() == self.mapbase
            && mapping.size() == self.mapsize
            && mapping.page_phys_base() <= mapping.phys_base()
            && mapping.mapped_size() >= mapping.size()
            && mapping.virt_base() != 0;
        self.vm_ioremap = mapping.uses_vm_ioremap();
        self.io_page_protection = mapping.uses_io_page_protection();
        self
    }

    fn bind_irq(mut self, context: &mut PlatformProbeContext<'_>) -> Self {
        let Some(source) = self.irq_source else {
            return self;
        };
        let Some(logical_irq) = context.map_plic_source(source) else {
            return self;
        };

        self.logical_irq = logical_irq;
        self.irq_mapping_ready = context
            .plic_mapping_for_source(source)
            .is_some_and(|mapping| {
                mapping.logical_irq() == logical_irq
                    && mapping.source_valid()
                    && mapping.source_gate_defined()
                    && mapping.source_gate_closed()
                    && mapping.source_not_enabled()
            });
        if !self.irq_mapping_ready {
            return self;
        }

        if context.request_irq(logical_irq, self.device_ref, IrqHandlerKind::VirtioMmio) {
            self.irq_handler_registered = context
                .irq_action_for_logical_irq(logical_irq)
                .is_some_and(|action| {
                    action.logical_irq() == logical_irq
                        && action.device() == self.device_ref
                        && action.handler_kind() == IrqHandlerKind::VirtioMmio
                        && action.handler_bound()
                        && action.mapped_irq_required()
                        && action.dispatch_ready()
                });
        }
        self
    }

    pub fn enable_irq_source_gate(
        &mut self,
        plic: &super::irq_time::Plic,
        plic_irq_domain: &mut super::irq_time::PlicIrqDomain,
    ) -> bool {
        if self.irq_source_gate_open {
            return true;
        }
        let Some(source) = self.irq_source else {
            return false;
        };
        if !self.irq_mapping_ready || !self.logical_irq.is_valid() {
            return false;
        }
        if plic_irq_domain.enable_source_gate(plic, source).is_err() {
            return false;
        }
        self.irq_source_gate_open =
            plic_irq_domain
                .mapping_for_source(source)
                .is_some_and(|mapping| {
                    mapping.logical_irq() == self.logical_irq && mapping.source_gate_open()
                });
        self.irq_source_gate_open
    }
}

#[derive(Clone, Copy)]
pub struct VirtioMmioHeader {
    magic: u32,
    version: u32,
    device_id: u32,
    vendor_id: u32,
}

impl VirtioMmioHeader {
    pub const fn new(magic: u32, version: u32, device_id: u32, vendor_id: u32) -> Self {
        Self {
            magic,
            version,
            device_id,
            vendor_id,
        }
    }

    pub const fn valid_device(device_id: u32, vendor_id: u32) -> Self {
        Self::new(VIRTIO_MMIO_MAGIC, 2, device_id, vendor_id)
    }

    pub const fn status(self) -> VirtioMmioHeaderStatus {
        classify_header(self)
    }

    pub const fn valid(self) -> bool {
        matches!(self.status(), VirtioMmioHeaderStatus::Valid)
    }

    pub const fn placeholder_device(self) -> bool {
        matches!(self.status(), VirtioMmioHeaderStatus::PlaceholderDevice)
    }

    pub const fn rng_candidate(self) -> bool {
        self.valid() && self.device_id == VIRTIO_ID_RNG
    }

    pub const fn version(self) -> u32 {
        self.version
    }

    pub const fn device_id(self) -> u32 {
        self.device_id
    }

    pub const fn vendor_id(self) -> u32 {
        self.vendor_id
    }
}

fn virtio_mmio_probe(
    context: &mut PlatformProbeContext<'_>,
    device: DeviceRef,
    node_id: DeviceNodeId,
) -> ProbeResult {
    let Some(base_transport) = build_transport_from_node(context.device_tree(), device, node_id)
    else {
        return ProbeResult::Deferred;
    };
    let Some(mapping) =
        context.map_platform_device_mmio(device, base_transport.mapbase, base_transport.mapsize)
    else {
        return ProbeResult::Deferred;
    };

    let transport = base_transport.bind_ioremap(mapping);
    let header = read_header_from_mmio(transport.membase);
    let transport = transport.with_header(header);
    print_probe(transport);

    if transport.header_valid() {
        let transport = transport.bind_irq(context);
        if context.register_virtio_mmio_device(transport).is_some() {
            ProbeResult::Bound
        } else {
            ProbeResult::Deferred
        }
    } else {
        ProbeResult::Rejected
    }
}

fn build_transport_from_node(
    device_tree: &DeviceTree,
    device: DeviceRef,
    node_id: DeviceNodeId,
) -> Option<VirtioMmioTransportDevice> {
    let node = device_tree.node(node_id)?;
    let reg = node.property(b"reg")?.raw_value();
    let reg_base = reg.as_ptr() as usize;
    let address_cells = parent_address_cells(device_tree, node.id())?;
    let size_cells = parent_size_cells(device_tree, node.id())?;
    let (mmio_base, used) = read_cells(reg_base, reg.len(), address_cells)?;
    let (mmio_size, _) = read_cells(reg_base.checked_add(used)?, reg.len() - used, size_cells)?;

    Some(VirtioMmioTransportDevice {
        device_ref: device,
        node_id: node.id(),
        mapbase: usize::try_from(mmio_base).ok()?,
        mapsize: usize::try_from(mmio_size).ok()?,
        irq_source: read_first_interrupt_source(node),
        ..VirtioMmioTransportDevice::empty()
    })
}

fn read_header_from_mmio(base: usize) -> VirtioMmioHeader {
    VirtioMmioHeader {
        magic: read_mmio_u32(base, 0x000),
        version: read_mmio_u32(base, 0x004),
        device_id: read_mmio_u32(base, 0x008),
        vendor_id: read_mmio_u32(base, 0x00c),
    }
}

const fn classify_header(header: VirtioMmioHeader) -> VirtioMmioHeaderStatus {
    if header.magic != VIRTIO_MMIO_MAGIC {
        return VirtioMmioHeaderStatus::InvalidMagic;
    }
    if header.version < VIRTIO_MMIO_VERSION_MIN || header.version > VIRTIO_MMIO_VERSION_MAX {
        return VirtioMmioHeaderStatus::UnsupportedVersion;
    }
    if header.device_id == 0 {
        return VirtioMmioHeaderStatus::PlaceholderDevice;
    }
    VirtioMmioHeaderStatus::Valid
}

fn read_mmio_u32(base: usize, offset: usize) -> u32 {
    let Some(addr) = base.checked_add(offset) else {
        return 0;
    };
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

fn write_mmio_u32(base: usize, offset: usize, value: u32) -> bool {
    let Some(addr) = base.checked_add(offset) else {
        return false;
    };
    unsafe {
        core::ptr::write_volatile(addr as *mut u32, value);
    }
    true
}

fn write_mmio_u64_halves(base: usize, low_offset: usize, high_offset: usize, value: usize) -> bool {
    write_mmio_u32(base, low_offset, value as u32)
        && write_mmio_u32(base, high_offset, (value >> 32) as u32)
}

pub fn reset_status(transport: VirtioMmioTransportDevice) -> bool {
    write_mmio_u32(transport.membase(), VIRTIO_MMIO_STATUS, 0)
}

pub fn driver_status_setup(transport: VirtioMmioTransportDevice) -> bool {
    let base = transport.membase();
    if !write_mmio_u32(base, VIRTIO_MMIO_STATUS, VIRTIO_CONFIG_S_ACKNOWLEDGE) {
        return false;
    }
    if !write_mmio_u32(
        base,
        VIRTIO_MMIO_STATUS,
        VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER,
    ) {
        return false;
    }

    let _ = write_mmio_u32(base, VIRTIO_MMIO_DEVICE_FEATURES_SEL, 0);
    let _ = read_mmio_u32(base, VIRTIO_MMIO_DEVICE_FEATURES);
    let _ = write_mmio_u32(base, VIRTIO_MMIO_DEVICE_FEATURES_SEL, 1);
    let _ = read_mmio_u32(base, VIRTIO_MMIO_DEVICE_FEATURES);
    let _ = write_mmio_u32(base, VIRTIO_MMIO_DRIVER_FEATURES_SEL, 0);
    let _ = write_mmio_u32(base, VIRTIO_MMIO_DRIVER_FEATURES, 0);
    let _ = write_mmio_u32(base, VIRTIO_MMIO_DRIVER_FEATURES_SEL, 1);
    let _ = write_mmio_u32(base, VIRTIO_MMIO_DRIVER_FEATURES, 0);

    if transport.version() == 2 {
        if !write_mmio_u32(
            base,
            VIRTIO_MMIO_STATUS,
            VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER | VIRTIO_CONFIG_S_FEATURES_OK,
        ) {
            return false;
        }
        let status = read_mmio_u32(base, VIRTIO_MMIO_STATUS);
        if status & VIRTIO_CONFIG_S_FEATURES_OK == 0 {
            return false;
        }
    }
    true
}

pub fn driver_status_ready(transport: VirtioMmioTransportDevice) -> bool {
    let status = read_mmio_u32(transport.membase(), VIRTIO_MMIO_STATUS);
    write_mmio_u32(
        transport.membase(),
        VIRTIO_MMIO_STATUS,
        status | VIRTIO_CONFIG_S_DRIVER_OK,
    )
}

#[derive(Clone, Copy)]
pub struct VirtioMmioQueueConfig {
    pub queue_index: u16,
    pub queue_size: u16,
    pub desc_phys: usize,
    pub avail_phys: usize,
    pub used_phys: usize,
}

#[derive(Clone, Copy)]
pub struct VirtioMmioQueueSetupResult {
    pub queue_size: u16,
    pub num_max: u16,
    pub legacy_pfn_written: bool,
    pub modern_addrs_written: bool,
    pub queue_ready_written: bool,
}

impl VirtioMmioQueueSetupResult {}

pub fn setup_queue(
    transport: VirtioMmioTransportDevice,
    config: VirtioMmioQueueConfig,
) -> Option<VirtioMmioQueueSetupResult> {
    let base = transport.membase();
    write_mmio_u32(base, VIRTIO_MMIO_QUEUE_SEL, u32::from(config.queue_index));
    let num_max = read_mmio_u32(base, VIRTIO_MMIO_QUEUE_NUM_MAX);
    if num_max == 0 || config.queue_size == 0 || u32::from(config.queue_size) > num_max {
        return None;
    }

    if transport.version() == 1 {
        if !write_mmio_u32(
            base,
            VIRTIO_MMIO_GUEST_PAGE_SIZE_REG,
            VIRTIO_MMIO_GUEST_PAGE_SIZE as u32,
        ) {
            return None;
        }
        if !write_mmio_u32(base, VIRTIO_MMIO_QUEUE_NUM, u32::from(config.queue_size)) {
            return None;
        }
        if !write_mmio_u32(
            base,
            VIRTIO_MMIO_QUEUE_ALIGN,
            VIRTIO_MMIO_VRING_ALIGN as u32,
        ) {
            return None;
        }
        let pfn = config.desc_phys >> 12;
        if pfn >> 32 != 0 {
            return None;
        }
        if !write_mmio_u32(base, VIRTIO_MMIO_QUEUE_PFN, pfn as u32) {
            return None;
        }
        let num_max = u16::try_from(num_max).ok()?;
        return Some(VirtioMmioQueueSetupResult {
            queue_size: config.queue_size,
            num_max,
            legacy_pfn_written: true,
            modern_addrs_written: false,
            queue_ready_written: false,
        });
    }

    if transport.version() == 2 {
        if !write_mmio_u32(base, VIRTIO_MMIO_QUEUE_NUM, u32::from(config.queue_size))
            || !write_mmio_u64_halves(
                base,
                VIRTIO_MMIO_QUEUE_DESC_LOW,
                VIRTIO_MMIO_QUEUE_DESC_HIGH,
                config.desc_phys,
            )
            || !write_mmio_u64_halves(
                base,
                VIRTIO_MMIO_QUEUE_AVAIL_LOW,
                VIRTIO_MMIO_QUEUE_AVAIL_HIGH,
                config.avail_phys,
            )
            || !write_mmio_u64_halves(
                base,
                VIRTIO_MMIO_QUEUE_USED_LOW,
                VIRTIO_MMIO_QUEUE_USED_HIGH,
                config.used_phys,
            )
            || !write_mmio_u32(base, VIRTIO_MMIO_QUEUE_READY, 1)
        {
            return None;
        }
        let num_max = u16::try_from(num_max).ok()?;
        return Some(VirtioMmioQueueSetupResult {
            queue_size: config.queue_size,
            num_max,
            legacy_pfn_written: false,
            modern_addrs_written: true,
            queue_ready_written: true,
        });
    }

    None
}

pub fn notify_queue(transport: VirtioMmioTransportDevice, queue_index: u16) -> bool {
    write_mmio_u32(
        transport.membase(),
        VIRTIO_MMIO_QUEUE_NOTIFY,
        u32::from(queue_index),
    )
}

pub fn handle_virtio_mmio_irq() {
    VIRTIO_MMIO_IRQ_HANDLER_CALLS.fetch_add(1, Ordering::AcqRel);
    let Some(transport) = crate::objects::virtio_rng::live_mmio_transport() else {
        return;
    };
    let status = read_mmio_u32(transport.membase(), VIRTIO_MMIO_INTERRUPT_STATUS);
    if status == 0 {
        return;
    }
    let _ = write_mmio_u32(transport.membase(), VIRTIO_MMIO_INTERRUPT_ACK, status);
    crate::objects::virtio_rng::note_mmio_irq(status);
    if status & VIRTIO_MMIO_INT_VRING != 0 {
        crate::objects::virtio_rng::handle_irq_completion();
    }
}

#[allow(dead_code)]
pub fn irq_handler_calls() -> usize {
    VIRTIO_MMIO_IRQ_HANDLER_CALLS.load(Ordering::Acquire)
}

fn print_probe(transport: VirtioMmioTransportDevice) {
    crate::objects::printk::write_str("platform_driver: virtio-mmio probed ");
    crate::objects::printk::write_fmt(format_args!(
        "device=node#{} type={} id={} version={} status={}\n",
        transport.node_id.index(),
        virtio_device_type_name(transport.device_id),
        transport.device_id,
        transport.version,
        header_status_name(transport.header_status),
    ));
}

fn virtio_device_type_name(device_id: u32) -> &'static str {
    match device_id {
        0 => "placeholder",
        VIRTIO_ID_RNG => "rng",
        _ => "unknown",
    }
}

fn header_status_name(status: VirtioMmioHeaderStatus) -> &'static str {
    match status {
        VirtioMmioHeaderStatus::Unknown => "unknown",
        VirtioMmioHeaderStatus::Valid => "valid",
        VirtioMmioHeaderStatus::InvalidMagic => "invalid-magic",
        VirtioMmioHeaderStatus::UnsupportedVersion => "unsupported-version",
        VirtioMmioHeaderStatus::PlaceholderDevice => "placeholder",
    }
}

fn read_first_interrupt_source(node: super::device_tree::DeviceNodeRef<'_>) -> Option<u32> {
    let value = node.property(b"interrupts")?.raw_value();
    let start = value.as_ptr() as usize;
    read_be_u32(start, start.checked_add(value.len())?)
}

fn parent_address_cells(device_tree: &DeviceTree, node_id: DeviceNodeId) -> Option<usize> {
    let parent = device_tree.node(node_id)?.parent()?;
    read_cells_u32(parent.property(b"#address-cells")).or(Some(2))
}

fn parent_size_cells(device_tree: &DeviceTree, node_id: DeviceNodeId) -> Option<usize> {
    let parent = device_tree.node(node_id)?.parent()?;
    read_cells_u32(parent.property(b"#size-cells")).or(Some(1))
}

fn read_cells_u32(property: Option<super::device_tree::DevicePropertyRef<'_>>) -> Option<usize> {
    let value = property?.raw_value();
    let cells = read_be_u32(
        value.as_ptr() as usize,
        value.as_ptr() as usize + value.len(),
    )?;
    usize::try_from(cells).ok()
}
