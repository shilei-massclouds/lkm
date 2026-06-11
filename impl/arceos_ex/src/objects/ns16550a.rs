use super::{
    device::DeviceRef,
    device_tree::{DeviceNodeId, DeviceTree},
    driver::{DeviceDriverRef, OfMatchEntry, OfMatchTable, PlatformDriver, ProbeResult},
    fdt_reader::{read_be_u32, read_cells},
    initcall::{ContextRef, InitcallReturn},
    printk,
};

const NS16550A_OF_MATCH: [OfMatchEntry; 1] = [OfMatchEntry::new(b"ns16550a")];
const DEFAULT_REG_SHIFT: u32 = 0;
const DEFAULT_REG_IO_WIDTH: u32 = 1;
const DEFAULT_CLOCK_FREQUENCY: u32 = 0;

pub static NS16550A_PLATFORM_DRIVER: PlatformDriver = PlatformDriver::new(
    "of_serial",
    OfMatchTable::new(&NS16550A_OF_MATCH),
    ns16550a_probe,
);

pub const NS16550A_PLATFORM_DRIVER_REF: DeviceDriverRef =
    DeviceDriverRef::new(&NS16550A_PLATFORM_DRIVER);

pub fn ns16550a_platform_driver_init(ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: ns16550a_platform_driver_init\n");
    let device_tree = &ctx.device_tree;
    ctx.platform_bus
        .platform_driver_register(NS16550A_PLATFORM_DRIVER_REF, device_tree)
}

crate::device_initcall!(ns16550a_platform_driver_init);

pub fn is_ns16550a_platform_driver(driver: DeviceDriverRef) -> bool {
    driver == NS16550A_PLATFORM_DRIVER_REF
}

#[derive(Clone, Copy)]
pub struct Uart8250Port {
    node_id: DeviceNodeId,
    device_ref: DeviceRef,
    mmio_base: u64,
    mmio_size: u64,
    reg_shift: u32,
    reg_io_width: u32,
    clock_frequency: u32,
    line: usize,
    registered: bool,
}

impl Uart8250Port {
    const fn empty() -> Self {
        Self {
            node_id: DeviceNodeId::invalid(),
            device_ref: DeviceRef::new(usize::MAX),
            mmio_base: 0,
            mmio_size: 0,
            reg_shift: 0,
            reg_io_width: 0,
            clock_frequency: 0,
            line: usize::MAX,
            registered: false,
        }
    }
}

pub struct Ns16550aProbeState {
    port: Uart8250Port,
    serial_console_registered: bool,
    stdout_path_matched: bool,
    handoff_triggered: bool,
}

impl Ns16550aProbeState {
    pub const fn new() -> Self {
        Self {
            port: Uart8250Port::empty(),
            serial_console_registered: false,
            stdout_path_matched: false,
            handoff_triggered: false,
        }
    }
}

static mut NS16550A_PROBE_STATE: Ns16550aProbeState = Ns16550aProbeState::new();

pub fn uart8250_port_registered() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .port
            .registered
    }
}

pub fn uart8250_port_node_id() -> Option<DeviceNodeId> {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    if state.port.registered {
        Some(state.port.node_id)
    } else {
        None
    }
}

pub fn uart8250_port_device_ref() -> Option<DeviceRef> {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    if state.port.registered {
        Some(state.port.device_ref)
    } else {
        None
    }
}

pub fn uart8250_port_resources_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered
        && state.port.mmio_base != 0
        && state.port.mmio_size != 0
        && state.port.reg_io_width != 0
        && state.port.line != usize::MAX
}

pub fn serial8250_console_registered() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .serial_console_registered
    }
}

pub fn stdout_path_matched() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .stdout_path_matched
    }
}

pub fn handoff_triggered() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .handoff_triggered
    }
}

fn ns16550a_probe(
    device_tree: &DeviceTree,
    device: DeviceRef,
    node_id: DeviceNodeId,
) -> ProbeResult {
    let Some(port) = build_uart8250_port(device_tree, device, node_id) else {
        return ProbeResult::Deferred;
    };

    let stdout_path_matched = device_tree.stdout_path_selects(port.node_id);
    let serial_console_registered =
        stdout_path_matched && printk::register_serial8250_console(true);
    let handoff_triggered = serial_console_registered && printk::console_handoff_complete();

    unsafe {
        let state = (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap();
        state.port = port;
        state.serial_console_registered = serial_console_registered;
        state.stdout_path_matched = stdout_path_matched;
        state.handoff_triggered = handoff_triggered;
    }

    ProbeResult::Bound
}

fn build_uart8250_port(
    device_tree: &DeviceTree,
    device: DeviceRef,
    node_id: DeviceNodeId,
) -> Option<Uart8250Port> {
    let node = device_tree.node(node_id)?;
    let reg = node.property(b"reg")?.raw_value();
    let reg_base = reg.as_ptr() as usize;
    let address_cells = parent_address_cells(device_tree, node.id())?;
    let size_cells = parent_size_cells(device_tree, node.id())?;
    let (mmio_base, used) = read_cells(reg_base, reg.len(), address_cells)?;
    let (mmio_size, _) = read_cells(reg_base.checked_add(used)?, reg.len() - used, size_cells)?;

    let reg_shift = read_property_u32(node.property(b"reg-shift")).unwrap_or(DEFAULT_REG_SHIFT);
    let reg_io_width =
        read_property_u32(node.property(b"reg-io-width")).unwrap_or(DEFAULT_REG_IO_WIDTH);
    let clock_frequency =
        read_property_u32(node.property(b"clock-frequency")).unwrap_or(DEFAULT_CLOCK_FREQUENCY);

    Some(Uart8250Port {
        node_id: node.id(),
        device_ref: device,
        mmio_base,
        mmio_size,
        reg_shift,
        reg_io_width,
        clock_frequency,
        line: device.index(),
        registered: true,
    })
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

fn read_property_u32(property: Option<super::device_tree::DevicePropertyRef<'_>>) -> Option<u32> {
    let value = property?.raw_value();
    read_be_u32(
        value.as_ptr() as usize,
        value.as_ptr() as usize + value.len(),
    )
}
