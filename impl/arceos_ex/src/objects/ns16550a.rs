use super::{
    device::DeviceRef,
    device_tree::{DeviceNodeId, DeviceTree},
    driver::{DeviceDriverRef, OfMatchEntry, OfMatchTable, PlatformDriver, ProbeResult},
    fdt_reader::{read_be_u32, read_cells},
    initcall::{ContextRef, InitcallReturn},
    ioremap::{IoMemoryMapping, Ioremap},
    mm_core::VmallocAllocator,
    printk,
};

const NS16550A_OF_MATCH: [OfMatchEntry; 1] = [OfMatchEntry::new(b"ns16550a")];
const DEFAULT_REG_SHIFT: u32 = 0;
const DEFAULT_REG_IO_WIDTH: u32 = 1;
const DEFAULT_CLOCK_FREQUENCY: u32 = 0;
const UART_TX: usize = 0;
const UART_LSR: usize = 5;
const UART_LSR_THRE: usize = 1 << 5;
const UART_POLL_SPINS: usize = 100_000;

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
    let vmalloc_allocator = &mut ctx.vmalloc_allocator;
    let ioremap = &mut ctx.ioremap;
    ctx.platform_bus.platform_driver_register(
        NS16550A_PLATFORM_DRIVER_REF,
        device_tree,
        vmalloc_allocator,
        ioremap,
    )
}

crate::device_initcall!(ns16550a_platform_driver_init);

pub fn is_ns16550a_platform_driver(driver: DeviceDriverRef) -> bool {
    driver == NS16550A_PLATFORM_DRIVER_REF
}

#[derive(Clone, Copy)]
pub struct Uart8250Port {
    node_id: DeviceNodeId,
    device_ref: DeviceRef,
    mapbase: usize,
    mapsize: usize,
    membase: usize,
    ioremapped: bool,
    vm_ioremap: bool,
    io_page_protection: bool,
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
            mapbase: 0,
            mapsize: 0,
            membase: 0,
            ioremapped: false,
            vm_ioremap: false,
            io_page_protection: false,
            reg_shift: 0,
            reg_io_width: 0,
            clock_frequency: 0,
            line: usize::MAX,
            registered: false,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Ns16550aProbeState {
    port: Uart8250Port,
    write_backend: Serial8250WriteBackend,
    serial_console_registered: bool,
    stdout_path_available: bool,
    stdout_path_matched: bool,
    handoff_triggered: bool,
}

impl Ns16550aProbeState {
    pub const fn new() -> Self {
        Self {
            port: Uart8250Port::empty(),
            write_backend: Serial8250WriteBackend::empty(),
            serial_console_registered: false,
            stdout_path_available: false,
            stdout_path_matched: false,
            handoff_triggered: false,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Serial8250WriteBackend {
    ready: bool,
    device_ref: DeviceRef,
    membase: usize,
    tx_addr: usize,
    lsr_addr: usize,
    tx_offset: usize,
    lsr_offset: usize,
    reg_shift: u32,
    reg_io_width: u32,
    lsr_thre_mask: usize,
    uses_uart_membase: bool,
    uses_lsr_thr_polling: bool,
    uses_sbi: bool,
    interrupt_driven: bool,
    interrupt_output_deferred: bool,
    write_calls: usize,
    bytes_accepted: usize,
    tx_bytes_submitted: usize,
    crlf_insertions: usize,
    last_tx_byte: u8,
    mmio_writes_performed: bool,
    timed_out: bool,
}

impl Serial8250WriteBackend {
    const fn empty() -> Self {
        Self {
            ready: false,
            device_ref: DeviceRef::new(usize::MAX),
            membase: 0,
            tx_addr: 0,
            lsr_addr: 0,
            tx_offset: 0,
            lsr_offset: 0,
            reg_shift: 0,
            reg_io_width: 0,
            lsr_thre_mask: 0,
            uses_uart_membase: false,
            uses_lsr_thr_polling: false,
            uses_sbi: false,
            interrupt_driven: false,
            interrupt_output_deferred: false,
            write_calls: 0,
            bytes_accepted: 0,
            tx_bytes_submitted: 0,
            crlf_insertions: 0,
            last_tx_byte: 0,
            mmio_writes_performed: false,
            timed_out: false,
        }
    }

    fn from_port(port: Uart8250Port) -> Option<Self> {
        if !port.registered
            || !port.ioremapped
            || !port.vm_ioremap
            || !port.io_page_protection
            || port.membase == 0
            || !uart_reg_io_width_supported(port.reg_io_width)
            || port.reg_shift > 8
        {
            return None;
        }

        let tx_offset = uart_register_offset(port, UART_TX)?;
        let lsr_offset = uart_register_offset(port, UART_LSR)?;
        let tx_addr = port.membase.checked_add(tx_offset)?;
        let lsr_addr = port.membase.checked_add(lsr_offset)?;
        Some(Self {
            ready: true,
            device_ref: port.device_ref,
            membase: port.membase,
            tx_addr,
            lsr_addr,
            tx_offset,
            lsr_offset,
            reg_shift: port.reg_shift,
            reg_io_width: port.reg_io_width,
            lsr_thre_mask: UART_LSR_THRE,
            uses_uart_membase: tx_addr == port.membase && lsr_addr > port.membase,
            uses_lsr_thr_polling: true,
            uses_sbi: false,
            interrupt_driven: false,
            interrupt_output_deferred: true,
            write_calls: 0,
            bytes_accepted: 0,
            tx_bytes_submitted: 0,
            crlf_insertions: 0,
            last_tx_byte: 0,
            mmio_writes_performed: false,
            timed_out: false,
        })
    }

    fn record_write(&mut self, bytes: &[u8]) -> bool {
        if !self.ready || !self.uses_lsr_thr_polling || self.uses_sbi || self.interrupt_driven {
            return false;
        }

        self.write_calls = self.write_calls.saturating_add(1);
        self.bytes_accepted = self.bytes_accepted.saturating_add(bytes.len());
        for byte in bytes {
            if *byte == b'\n' {
                if !self.write_tx_byte(b'\r') {
                    return false;
                }
                self.crlf_insertions = self.crlf_insertions.saturating_add(1);
            }
            if !self.write_tx_byte(*byte) {
                return false;
            }
        }
        true
    }

    fn write_tx_byte(&mut self, byte: u8) -> bool {
        if !self.wait_for_tx_ready() {
            self.timed_out = true;
            return false;
        }
        if !self.write_uart_tx(byte) {
            return false;
        }
        self.tx_bytes_submitted = self.tx_bytes_submitted.saturating_add(1);
        self.last_tx_byte = byte;
        self.mmio_writes_performed = true;
        true
    }

    fn wait_for_tx_ready(&self) -> bool {
        let mut spins = UART_POLL_SPINS;
        while spins != 0 {
            if self.read_uart_lsr() & self.lsr_thre_mask != 0 {
                return true;
            }
            core::hint::spin_loop();
            spins -= 1;
        }
        false
    }

    fn read_uart_lsr(&self) -> usize {
        match self.reg_io_width {
            1 => unsafe { core::ptr::read_volatile(self.lsr_addr as *const u8) as usize },
            2 => unsafe { core::ptr::read_volatile(self.lsr_addr as *const u16) as usize },
            4 => unsafe { core::ptr::read_volatile(self.lsr_addr as *const u32) as usize },
            _ => 0,
        }
    }

    fn write_uart_tx(&self, byte: u8) -> bool {
        match self.reg_io_width {
            1 => unsafe { core::ptr::write_volatile(self.tx_addr as *mut u8, byte) },
            2 => unsafe { core::ptr::write_volatile(self.tx_addr as *mut u16, byte as u16) },
            4 => unsafe { core::ptr::write_volatile(self.tx_addr as *mut u32, byte as u32) },
            _ => return false,
        }
        true
    }

    fn facts_ready(self, port: Uart8250Port) -> bool {
        self.ready
            && port.registered
            && self.device_ref == port.device_ref
            && self.membase == port.membase
            && self.tx_addr == port.membase
            && self.lsr_addr == port.membase.saturating_add(self.lsr_offset)
            && self.tx_offset == 0
            && self.lsr_offset == (UART_LSR << port.reg_shift)
            && self.reg_shift == port.reg_shift
            && self.reg_io_width == port.reg_io_width
            && self.lsr_thre_mask == UART_LSR_THRE
            && self.uses_uart_membase
            && self.uses_lsr_thr_polling
            && !self.uses_sbi
            && !self.interrupt_driven
            && self.interrupt_output_deferred
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

#[cfg(checkpoint_handler_console_handoff)]
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
        && state.port.mapbase != 0
        && state.port.mapsize != 0
        && state.port.membase != 0
        && state.port.membase != state.port.mapbase
        && state.port.ioremapped
        && state.port.vm_ioremap
        && state.port.io_page_protection
        && state.port.reg_shift <= 8
        && uart_reg_io_width_supported(state.port.reg_io_width)
        && (state.port.clock_frequency == DEFAULT_CLOCK_FREQUENCY
            || state.port.clock_frequency > DEFAULT_CLOCK_FREQUENCY)
        && state.port.line != usize::MAX
}

pub fn uart8250_port_ioremapped() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered
        && state.port.ioremapped
        && state.port.vm_ioremap
        && state.port.io_page_protection
        && state.port.membase != 0
        && state.port.membase != state.port.mapbase
}

pub fn serial8250_console_registered() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .serial_console_registered
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_write_backend_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.serial_console_registered && state.write_backend.facts_ready(state.port)
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_write_uses_membase() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.write_backend.ready
        && state.write_backend.uses_uart_membase
        && state.write_backend.membase == state.port.membase
        && state.write_backend.tx_addr == state.port.membase
        && state.port.membase != state.port.mapbase
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_write_uses_lsr_thr_polling() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.write_backend.ready
        && state.write_backend.uses_lsr_thr_polling
        && state.write_backend.tx_offset == 0
        && state.write_backend.lsr_offset == (UART_LSR << state.port.reg_shift)
        && state.write_backend.lsr_thre_mask == UART_LSR_THRE
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_write_does_not_use_sbi() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.write_backend.ready && !state.write_backend.uses_sbi
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_interrupt_output_deferred() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.write_backend.ready
        && state.write_backend.interrupt_output_deferred
        && !state.write_backend.interrupt_driven
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_write_call_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .write_calls
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_tx_byte_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .tx_bytes_submitted
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_mmio_writes_performed() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .mmio_writes_performed
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_write_timed_out() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .timed_out
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn stdout_path_matched() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .stdout_path_matched
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn stdout_path_available() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .stdout_path_available
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
    vmalloc_allocator: &mut VmallocAllocator,
    ioremap: &mut Ioremap,
    device: DeviceRef,
    node_id: DeviceNodeId,
) -> ProbeResult {
    let Some(mut port) = build_uart8250_port(device_tree, device, node_id) else {
        return ProbeResult::Deferred;
    };
    let Some(mapping) =
        ioremap.map_device_mmio(vmalloc_allocator, device, port.mapbase, port.mapsize)
    else {
        return ProbeResult::Deferred;
    };
    bind_ioremap_mapping(&mut port, mapping);

    let stdout_path_available = device_tree.stdout_path_available();
    let stdout_path_matched =
        stdout_path_available && device_tree.stdout_path_selects(port.node_id);
    let serial_console_registered =
        stdout_path_matched && printk::register_serial8250_console(true);
    let write_backend = if serial_console_registered {
        Serial8250WriteBackend::from_port(port).unwrap_or(Serial8250WriteBackend::empty())
    } else {
        Serial8250WriteBackend::empty()
    };
    let handoff_triggered = serial_console_registered && printk::console_handoff_complete();

    unsafe {
        let state = (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap();
        state.port = port;
        state.write_backend = write_backend;
        state.serial_console_registered = serial_console_registered;
        state.stdout_path_available = stdout_path_available;
        state.stdout_path_matched = stdout_path_matched;
        state.handoff_triggered = handoff_triggered;
    }

    ProbeResult::Bound
}

pub fn write_console_bytes(bytes: &[u8]) -> bool {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    if !state.serial_console_registered || !state.write_backend.facts_ready(state.port) {
        return false;
    }
    state.write_backend.record_write(bytes)
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn probe_state_snapshot() -> Ns16550aProbeState {
    unsafe { *(&raw const NS16550A_PROBE_STATE).as_ref().unwrap() }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn restore_probe_state(snapshot: Ns16550aProbeState) {
    unsafe {
        *(&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() = snapshot;
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn reset_probe_state_for_smoke() {
    unsafe {
        *(&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() = Ns16550aProbeState::new();
    }
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
    let mapbase = usize::try_from(mmio_base).ok()?;
    let mapsize = usize::try_from(mmio_size).ok()?;

    let reg_shift = read_property_u32(node.property(b"reg-shift")).unwrap_or(DEFAULT_REG_SHIFT);
    let reg_io_width =
        read_property_u32(node.property(b"reg-io-width")).unwrap_or(DEFAULT_REG_IO_WIDTH);
    let clock_frequency =
        read_property_u32(node.property(b"clock-frequency")).unwrap_or(DEFAULT_CLOCK_FREQUENCY);

    Some(Uart8250Port {
        node_id: node.id(),
        device_ref: device,
        mapbase,
        mapsize,
        membase: 0,
        ioremapped: false,
        vm_ioremap: false,
        io_page_protection: false,
        reg_shift,
        reg_io_width,
        clock_frequency,
        line: device.index(),
        registered: true,
    })
}

fn bind_ioremap_mapping(port: &mut Uart8250Port, mapping: IoMemoryMapping) {
    port.membase = mapping.membase();
    port.ioremapped = mapping.membase_cookie_ready()
        && mapping.phys_base() == port.mapbase
        && mapping.size() == port.mapsize
        && mapping.page_phys_base() <= mapping.phys_base()
        && mapping.mapped_size() >= mapping.size()
        && mapping.virt_base() != 0;
    port.vm_ioremap = mapping.uses_vm_ioremap();
    port.io_page_protection = mapping.uses_io_page_protection();
}

fn uart_register_offset(port: Uart8250Port, reg: usize) -> Option<usize> {
    reg.checked_shl(port.reg_shift)
}

const fn uart_reg_io_width_supported(width: u32) -> bool {
    matches!(width, 1 | 2 | 4)
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
