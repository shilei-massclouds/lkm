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
    printk,
};
use crate::{arch::riscv64::csr, trace, trace::Checkpoint};
use core::sync::atomic::{AtomicUsize, Ordering};

const NS16550A_OF_MATCH: [OfMatchEntry; 1] = [OfMatchEntry::new(b"ns16550a")];
const DEFAULT_REG_SHIFT: u32 = 0;
const DEFAULT_REG_IO_WIDTH: u32 = 1;
const DEFAULT_CLOCK_FREQUENCY: u32 = 0;
const UART_RX: usize = 0;
const UART_TX: usize = 0;
const UART_IER: usize = 1;
const UART_IIR: usize = 2;
const UART_FCR: usize = 2;
const UART_MCR: usize = 4;
const UART_LSR: usize = 5;
const UART_IER_RDI: usize = 1 << 0;
const UART_IER_THRI: usize = 1 << 1;
const UART_IER_RLSI: usize = 1 << 2;
const UART_FCR_ENABLE_FIFO: usize = 1 << 0;
const UART_FCR_CLEAR_RCVR: usize = 1 << 1;
const UART_FCR_CLEAR_XMIT: usize = 1 << 2;
const UART_IIR_NO_INT: usize = 1;
const UART_IIR_ID: usize = 0x0e;
const UART_IIR_THRI: usize = 0x02;
const UART_IIR_RDI: usize = 0x04;
const UART_IIR_RLSI: usize = 0x06;
const UART_IIR_RX_TIMEOUT: usize = 0x0c;
const UART_MCR_OUT2: usize = 1 << 3;
const UART_MCR_LOOP: usize = 1 << 4;
const UART_LSR_DR: usize = 1;
const UART_LSR_THRE: usize = 1 << 5;
const UART_POLL_SPINS: usize = 100_000;
const UART_RX_DRAIN_LIMIT: usize = 16;
const UART_TX_LOAD_SIZE: usize = 8;
const UART_TX_QUEUE_SIZE: usize = 512;
const TTY_FLIP_BUFFER_SIZE: usize = 64;
const TTY_XMIT_FIFO_SIZE: usize = 64;
const PLIC_COMPATIBLE_SIFIVE: &[u8] = b"sifive,plic-1.0.0";
const PLIC_COMPATIBLE_RISCV: &[u8] = b"riscv,plic0";

static UART8250_IRQ_HANDLER_CALLS: AtomicUsize = AtomicUsize::new(0);
static UART8250_THRE_INTERRUPT_REQUESTS: AtomicUsize = AtomicUsize::new(0);
static UART8250_THRE_INTERRUPT_HANDLED: AtomicUsize = AtomicUsize::new(0);
static UART8250_THRI_DISABLED_BY_HANDLER: AtomicUsize = AtomicUsize::new(0);
static UART8250_RX_INTERRUPT_REQUESTS: AtomicUsize = AtomicUsize::new(0);
static UART8250_RX_INTERRUPT_HANDLED: AtomicUsize = AtomicUsize::new(0);
static UART8250_LAST_IIR: AtomicUsize = AtomicUsize::new(UART_IIR_NO_INT);
static UART8250_LAST_LSR: AtomicUsize = AtomicUsize::new(0);

pub static NS16550A_PLATFORM_DRIVER: PlatformDriver = PlatformDriver::new(
    "of_serial",
    OfMatchTable::new(&NS16550A_OF_MATCH),
    ns16550a_probe,
);

pub const NS16550A_PLATFORM_DRIVER_REF: DeviceDriverRef =
    DeviceDriverRef::new(&NS16550A_PLATFORM_DRIVER);

pub fn ns16550a_platform_driver_init(ctx: ContextRef<'_>) -> InitcallReturn {
    crate::objects::printk::write_str("initcall: ns16550a_platform_driver_init\n");
    ctx.platform_driver_register(NS16550A_PLATFORM_DRIVER_REF)
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
    irq_source: u32,
    logical_irq: LogicalIrq,
    irq_resource_ready: bool,
    irq_parent_plic: bool,
    irq_mapping_ready: bool,
    irq_handler_registered: bool,
    irq_handler_hardirq_context_required: bool,
    irq_handler_dispatch_ready: bool,
    interrupt_output_deferred: bool,
    interrupt_driven_ready: bool,
    interrupt_trigger_ready: bool,
    thre_interrupt_enabled: bool,
    thre_interrupt_handled: bool,
    thre_interrupt_loopback: bool,
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
            irq_source: 0,
            logical_irq: LogicalIrq::invalid(),
            irq_resource_ready: false,
            irq_parent_plic: false,
            irq_mapping_ready: false,
            irq_handler_registered: false,
            irq_handler_hardirq_context_required: false,
            irq_handler_dispatch_ready: false,
            interrupt_output_deferred: false,
            interrupt_driven_ready: false,
            interrupt_trigger_ready: false,
            thre_interrupt_enabled: false,
            thre_interrupt_handled: false,
            thre_interrupt_loopback: false,
            line: usize::MAX,
            registered: false,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TtyPort {
    ready: bool,
    initialized: bool,
    online: bool,
    device_ref: DeviceRef,
    line: usize,
    bound_to_uart8250: bool,
    owns_flip_buffer: bool,
    owns_xmit_fifo: bool,
    no_mmio_access: bool,
    no_irq_dispatch: bool,
    no_console_registry_policy: bool,
    uart_startup_drives_runtime_enable: bool,
}

impl TtyPort {
    const fn empty() -> Self {
        Self {
            ready: false,
            initialized: false,
            online: false,
            device_ref: DeviceRef::new(usize::MAX),
            line: usize::MAX,
            bound_to_uart8250: false,
            owns_flip_buffer: false,
            owns_xmit_fifo: false,
            no_mmio_access: true,
            no_irq_dispatch: true,
            no_console_registry_policy: true,
            uart_startup_drives_runtime_enable: false,
        }
    }

    fn from_port(port: Uart8250Port) -> Self {
        if !port.registered || port.line == usize::MAX {
            return Self::empty();
        }

        Self {
            ready: true,
            initialized: false,
            online: false,
            device_ref: port.device_ref,
            line: port.line,
            bound_to_uart8250: true,
            owns_flip_buffer: true,
            owns_xmit_fifo: true,
            no_mmio_access: true,
            no_irq_dispatch: true,
            no_console_registry_policy: true,
            uart_startup_drives_runtime_enable: true,
        }
    }

    fn facts_ready(self, port: Uart8250Port) -> bool {
        self.ready
            && port.registered
            && self.device_ref == port.device_ref
            && self.line == port.line
            && self.bound_to_uart8250
            && self.owns_flip_buffer
            && self.owns_xmit_fifo
            && self.no_mmio_access
            && self.no_irq_dispatch
            && self.no_console_registry_policy
            && self.uart_startup_drives_runtime_enable
    }

    fn enable_for_uart_startup(&mut self) -> bool {
        if !self.ready || self.initialized || self.online {
            return false;
        }

        self.initialized = true;
        self.online = true;
        true
    }
}

#[derive(Clone, Copy)]
pub struct TtyFlipBuffer {
    ready: bool,
    bound_to_tty_port: bool,
    rx_staging_only: bool,
    push_is_observation_boundary: bool,
    n_tty_read_deferred: bool,
    buffer: [u8; TTY_FLIP_BUFFER_SIZE],
    pending_len: usize,
    read_ready_len: usize,
    read_offset: usize,
    read_count: usize,
    push_count: usize,
    total_inserted: usize,
    last_pushed_len: usize,
    last_read_len: usize,
    last_byte: u8,
    overflowed: bool,
}

impl TtyFlipBuffer {
    const fn empty() -> Self {
        Self {
            ready: false,
            bound_to_tty_port: false,
            rx_staging_only: false,
            push_is_observation_boundary: false,
            n_tty_read_deferred: true,
            buffer: [0; TTY_FLIP_BUFFER_SIZE],
            pending_len: 0,
            read_ready_len: 0,
            read_offset: 0,
            read_count: 0,
            push_count: 0,
            total_inserted: 0,
            last_pushed_len: 0,
            last_read_len: 0,
            last_byte: 0,
            overflowed: false,
        }
    }

    fn from_tty_port(tty_port: TtyPort) -> Self {
        if !tty_port.ready || !tty_port.owns_flip_buffer {
            return Self::empty();
        }

        Self {
            ready: true,
            bound_to_tty_port: true,
            rx_staging_only: true,
            push_is_observation_boundary: true,
            n_tty_read_deferred: true,
            buffer: [0; TTY_FLIP_BUFFER_SIZE],
            pending_len: 0,
            read_ready_len: 0,
            read_offset: 0,
            read_count: 0,
            push_count: 0,
            total_inserted: 0,
            last_pushed_len: 0,
            last_read_len: 0,
            last_byte: 0,
            overflowed: false,
        }
    }

    fn facts_ready(self, tty_port: TtyPort) -> bool {
        self.ready
            && tty_port.ready
            && tty_port.owns_flip_buffer
            && self.bound_to_tty_port
            && self.rx_staging_only
            && self.push_is_observation_boundary
            && self.n_tty_read_deferred
    }

    fn compact_ready_prefix(&mut self) {
        if self.read_offset == 0 || self.pending_len != 0 {
            return;
        }

        let unread = self.read_ready_len.saturating_sub(self.read_offset);
        if unread != 0 {
            self.buffer
                .copy_within(self.read_offset..self.read_ready_len, 0);
        }
        self.read_ready_len = unread;
        self.read_offset = 0;
    }

    fn insert_char(&mut self, byte: u8) -> bool {
        if !self.ready {
            self.overflowed = true;
            return false;
        }

        self.compact_ready_prefix();
        let write_index = self.read_ready_len + self.pending_len;
        if write_index >= TTY_FLIP_BUFFER_SIZE {
            self.overflowed = true;
            return false;
        }

        self.buffer[write_index] = byte;
        self.pending_len += 1;
        self.total_inserted = self.total_inserted.saturating_add(1);
        self.last_byte = byte;
        true
    }

    fn push(&mut self) -> bool {
        if !self.ready || self.pending_len == 0 {
            return false;
        }

        self.last_pushed_len = self.pending_len;
        self.push_count = self.push_count.saturating_add(1);
        self.read_ready_len += self.pending_len;
        self.pending_len = 0;
        true
    }

    fn read_ready_data(&mut self, buffer: &mut [u8]) -> Option<usize> {
        if !self.ready {
            return None;
        }
        if buffer.is_empty() || self.read_offset >= self.read_ready_len {
            return Some(0);
        }

        let available = self.read_ready_len - self.read_offset;
        let len = core::cmp::min(buffer.len(), available);
        let end = self.read_offset + len;
        buffer[..len].copy_from_slice(&self.buffer[self.read_offset..end]);
        self.read_offset = end;
        self.last_read_len = len;
        self.read_count = self.read_count.saturating_add(1);
        if self.read_offset == self.read_ready_len {
            self.read_ready_len = 0;
            self.read_offset = 0;
        }
        Some(len)
    }

    fn canonical_readable_len(self) -> Option<usize> {
        if !self.ready {
            return None;
        }
        if self.read_offset >= self.read_ready_len {
            return Some(0);
        }

        let ready = &self.buffer[self.read_offset..self.read_ready_len];
        ready
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|index| index + 1)
            .or(Some(0))
    }

    fn read_ready_data_with_mode(&mut self, buffer: &mut [u8], canonical: bool) -> Option<usize> {
        if !canonical {
            return self.read_ready_data(buffer);
        }
        if !self.ready {
            return None;
        }
        if buffer.is_empty() {
            return Some(0);
        }

        let readable = self.canonical_readable_len()?;
        if readable == 0 {
            return Some(0);
        }

        let len = core::cmp::min(buffer.len(), readable);
        let end = self.read_offset + len;
        buffer[..len].copy_from_slice(&self.buffer[self.read_offset..end]);
        self.read_offset = end;
        self.last_read_len = len;
        self.read_count = self.read_count.saturating_add(1);
        if self.read_offset == self.read_ready_len {
            self.read_ready_len = 0;
            self.read_offset = 0;
        }
        Some(len)
    }

    fn read_ready_available(self) -> Option<bool> {
        if !self.ready {
            return None;
        }

        Some(self.read_offset < self.read_ready_len)
    }

    fn read_ready_available_with_mode(self, canonical: bool) -> Option<bool> {
        if !canonical {
            return self.read_ready_available();
        }

        Some(self.canonical_readable_len()? != 0)
    }

    fn clear_ready_data(&mut self) -> bool {
        if !self.ready {
            return false;
        }

        self.buffer = [0; TTY_FLIP_BUFFER_SIZE];
        self.pending_len = 0;
        self.read_ready_len = 0;
        self.read_offset = 0;
        self.last_read_len = 0;
        true
    }

    fn seed_ready_data_fixture(&mut self, bytes: &[u8]) -> bool {
        if !self.ready
            || bytes.is_empty()
            || bytes.len() > TTY_FLIP_BUFFER_SIZE
            || self.pending_len != 0
            || self.read_ready_len != 0
        {
            return false;
        }

        self.buffer = [0; TTY_FLIP_BUFFER_SIZE];
        self.buffer[..bytes.len()].copy_from_slice(bytes);
        self.pending_len = 0;
        self.read_ready_len = bytes.len();
        self.read_offset = 0;
        self.last_read_len = 0;
        if let Some(last) = bytes.last() {
            self.last_byte = *last;
        }
        true
    }
}

#[derive(Clone, Copy)]
pub struct TtyXmitFifo {
    ready: bool,
    bound_to_tty_port: bool,
    ordinary_tty_write_path: bool,
    distinct_from_printk_console_tx: bool,
    runtime_tx_integration_deferred: bool,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    runtime_tx_integrated: bool,
    buffer: [u8; TTY_XMIT_FIFO_SIZE],
    head: usize,
    tail: usize,
    queued: usize,
    enqueue_count: usize,
    dequeue_count: usize,
    last_enqueued: u8,
    last_dequeued: u8,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    runtime_tx_kicks: usize,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    runtime_tx_drains: usize,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    runtime_tx_empty_stops: usize,
    #[cfg(checkpoint_handler_uart_irq_chain)]
    runtime_tx_guarded_by_local_irq_save: bool,
    overflowed: bool,
    underflowed: bool,
}

impl TtyXmitFifo {
    const fn empty() -> Self {
        Self {
            ready: false,
            bound_to_tty_port: false,
            ordinary_tty_write_path: false,
            distinct_from_printk_console_tx: true,
            runtime_tx_integration_deferred: true,
            #[cfg(checkpoint_handler_uart_irq_chain)]
            runtime_tx_integrated: false,
            buffer: [0; TTY_XMIT_FIFO_SIZE],
            head: 0,
            tail: 0,
            queued: 0,
            enqueue_count: 0,
            dequeue_count: 0,
            last_enqueued: 0,
            last_dequeued: 0,
            #[cfg(checkpoint_handler_uart_irq_chain)]
            runtime_tx_kicks: 0,
            #[cfg(checkpoint_handler_uart_irq_chain)]
            runtime_tx_drains: 0,
            #[cfg(checkpoint_handler_uart_irq_chain)]
            runtime_tx_empty_stops: 0,
            #[cfg(checkpoint_handler_uart_irq_chain)]
            runtime_tx_guarded_by_local_irq_save: false,
            overflowed: false,
            underflowed: false,
        }
    }

    fn from_tty_port(tty_port: TtyPort) -> Self {
        if !tty_port.ready || !tty_port.owns_xmit_fifo {
            return Self::empty();
        }

        Self {
            ready: true,
            bound_to_tty_port: true,
            ordinary_tty_write_path: true,
            distinct_from_printk_console_tx: true,
            runtime_tx_integration_deferred: true,
            #[cfg(checkpoint_handler_uart_irq_chain)]
            runtime_tx_integrated: false,
            buffer: [0; TTY_XMIT_FIFO_SIZE],
            head: 0,
            tail: 0,
            queued: 0,
            enqueue_count: 0,
            dequeue_count: 0,
            last_enqueued: 0,
            last_dequeued: 0,
            #[cfg(checkpoint_handler_uart_irq_chain)]
            runtime_tx_kicks: 0,
            #[cfg(checkpoint_handler_uart_irq_chain)]
            runtime_tx_drains: 0,
            #[cfg(checkpoint_handler_uart_irq_chain)]
            runtime_tx_empty_stops: 0,
            #[cfg(checkpoint_handler_uart_irq_chain)]
            runtime_tx_guarded_by_local_irq_save: false,
            overflowed: false,
            underflowed: false,
        }
    }

    fn facts_ready(self, tty_port: TtyPort) -> bool {
        self.ready
            && tty_port.ready
            && tty_port.owns_xmit_fifo
            && self.bound_to_tty_port
            && self.ordinary_tty_write_path
            && self.distinct_from_printk_console_tx
    }

    fn enqueue(&mut self, byte: u8) -> bool {
        if !self.ready || self.queued == TTY_XMIT_FIFO_SIZE {
            self.overflowed = true;
            return false;
        }

        self.buffer[self.tail] = byte;
        self.tail = (self.tail + 1) % TTY_XMIT_FIFO_SIZE;
        self.queued += 1;
        self.enqueue_count = self.enqueue_count.saturating_add(1);
        self.last_enqueued = byte;
        true
    }

    fn dequeue_for_tx(&mut self) -> Option<u8> {
        if !self.ready || self.queued == 0 {
            self.underflowed = true;
            return None;
        }

        let byte = self.buffer[self.head];
        self.head = (self.head + 1) % TTY_XMIT_FIFO_SIZE;
        self.queued -= 1;
        self.dequeue_count = self.dequeue_count.saturating_add(1);
        self.last_dequeued = byte;
        Some(byte)
    }

    #[cfg(checkpoint_handler_uart_irq_chain)]
    fn mark_runtime_tx_integrated(&mut self) {
        self.runtime_tx_integration_deferred = false;
        self.runtime_tx_integrated = true;
    }
}

#[derive(Clone, Copy)]
pub struct Serial8250RuntimePort {
    ready: bool,
    online: bool,
    device_ref: DeviceRef,
    membase: usize,
    rx_addr: usize,
    tx_addr: usize,
    ier_addr: usize,
    iir_addr: usize,
    lsr_addr: usize,
    reg_shift: u32,
    reg_io_width: u32,
    handles_ier_iir_lsr: bool,
    handler_requires_hardirq: bool,
    owns_port_lock_irqsave: bool,
    tty_flip_buffer_bound: bool,
    tty_xmit_fifo_bound: bool,
    rdi_enabled: bool,
    rlsi_enabled: bool,
    thri_demand_driven: bool,
    rx_fifo_enabled: bool,
    rx_interrupts_deferred_until_enable: bool,
    n_tty_read_deferred: bool,
    console_tx_interrupt_driven: bool,
    rx_drain_limit: usize,
    tx_load_size: usize,
}

impl Serial8250RuntimePort {
    const fn empty() -> Self {
        Self {
            ready: false,
            online: false,
            device_ref: DeviceRef::new(usize::MAX),
            membase: 0,
            rx_addr: 0,
            tx_addr: 0,
            ier_addr: 0,
            iir_addr: 0,
            lsr_addr: 0,
            reg_shift: 0,
            reg_io_width: 0,
            handles_ier_iir_lsr: false,
            handler_requires_hardirq: false,
            owns_port_lock_irqsave: false,
            tty_flip_buffer_bound: false,
            tty_xmit_fifo_bound: false,
            rdi_enabled: false,
            rlsi_enabled: false,
            thri_demand_driven: false,
            rx_fifo_enabled: false,
            rx_interrupts_deferred_until_enable: true,
            n_tty_read_deferred: true,
            console_tx_interrupt_driven: false,
            rx_drain_limit: 0,
            tx_load_size: 0,
        }
    }

    fn from_port(
        port: Uart8250Port,
        tty_port: TtyPort,
        flip_buffer: TtyFlipBuffer,
        xmit_fifo: TtyXmitFifo,
    ) -> Option<Self> {
        if !port.registered
            || !port.ioremapped
            || !port.irq_handler_registered
            || !tty_port.facts_ready(port)
            || !flip_buffer.facts_ready(tty_port)
            || !xmit_fifo.facts_ready(tty_port)
            || !uart_reg_io_width_supported(port.reg_io_width)
            || port.reg_shift > 8
        {
            return None;
        }

        let rx_addr = port
            .membase
            .checked_add(uart_register_offset(port, UART_RX)?)?;
        let tx_addr = port
            .membase
            .checked_add(uart_register_offset(port, UART_TX)?)?;
        let ier_addr = port
            .membase
            .checked_add(uart_register_offset(port, UART_IER)?)?;
        let iir_addr = port
            .membase
            .checked_add(uart_register_offset(port, UART_IIR)?)?;
        let lsr_addr = port
            .membase
            .checked_add(uart_register_offset(port, UART_LSR)?)?;

        Some(Self {
            ready: true,
            online: false,
            device_ref: port.device_ref,
            membase: port.membase,
            rx_addr,
            tx_addr,
            ier_addr,
            iir_addr,
            lsr_addr,
            reg_shift: port.reg_shift,
            reg_io_width: port.reg_io_width,
            handles_ier_iir_lsr: true,
            handler_requires_hardirq: true,
            owns_port_lock_irqsave: true,
            tty_flip_buffer_bound: true,
            tty_xmit_fifo_bound: true,
            rdi_enabled: false,
            rlsi_enabled: false,
            thri_demand_driven: true,
            rx_fifo_enabled: false,
            rx_interrupts_deferred_until_enable: true,
            n_tty_read_deferred: true,
            console_tx_interrupt_driven: false,
            rx_drain_limit: UART_RX_DRAIN_LIMIT,
            tx_load_size: UART_TX_LOAD_SIZE,
        })
    }

    fn facts_ready(
        self,
        port: Uart8250Port,
        tty_port: TtyPort,
        flip_buffer: TtyFlipBuffer,
        xmit_fifo: TtyXmitFifo,
    ) -> bool {
        self.ready
            && port.registered
            && port.ioremapped
            && port.irq_handler_registered
            && self.device_ref == port.device_ref
            && self.membase == port.membase
            && self.rx_addr == port.membase
            && self.tx_addr == port.membase
            && self.ier_addr == port.membase.saturating_add(UART_IER << port.reg_shift)
            && self.iir_addr == port.membase.saturating_add(UART_IIR << port.reg_shift)
            && self.lsr_addr == port.membase.saturating_add(UART_LSR << port.reg_shift)
            && self.reg_shift == port.reg_shift
            && self.reg_io_width == port.reg_io_width
            && self.handles_ier_iir_lsr
            && self.handler_requires_hardirq
            && self.owns_port_lock_irqsave
            && self.tty_flip_buffer_bound
            && self.tty_xmit_fifo_bound
            && self.thri_demand_driven
            && self.n_tty_read_deferred
            && self.rx_drain_limit == UART_RX_DRAIN_LIMIT
            && self.tx_load_size == UART_TX_LOAD_SIZE
            && tty_port.facts_ready(port)
            && flip_buffer.facts_ready(tty_port)
            && xmit_fifo.facts_ready(tty_port)
    }

    fn enable_rx_runtime(&mut self, tty_port: TtyPort) -> bool {
        if !self.ready
            || !self.console_tx_interrupt_driven
            || !tty_port.ready
            || !tty_port.initialized
            || !tty_port.online
            || self.online
            || self.rdi_enabled
            || self.rlsi_enabled
            || self.rx_fifo_enabled
        {
            return false;
        }

        self.online = true;
        self.rdi_enabled = true;
        self.rlsi_enabled = true;
        self.rx_fifo_enabled = true;
        self.rx_interrupts_deferred_until_enable = false;
        true
    }
}

#[derive(Clone, Copy)]
pub struct Ns16550aProbeState {
    port: Uart8250Port,
    tty_port: TtyPort,
    tty_flip_buffer: TtyFlipBuffer,
    tty_xmit_fifo: TtyXmitFifo,
    runtime_port: Serial8250RuntimePort,
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
            tty_port: TtyPort::empty(),
            tty_flip_buffer: TtyFlipBuffer::empty(),
            tty_xmit_fifo: TtyXmitFifo::empty(),
            runtime_port: Serial8250RuntimePort::empty(),
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
    ier_addr: usize,
    iir_addr: usize,
    mcr_addr: usize,
    lsr_addr: usize,
    tx_offset: usize,
    ier_offset: usize,
    iir_offset: usize,
    mcr_offset: usize,
    lsr_offset: usize,
    reg_shift: u32,
    reg_io_width: u32,
    lsr_thre_mask: usize,
    uses_uart_membase: bool,
    uses_lsr_thr_polling: bool,
    uses_sbi: bool,
    interrupt_driven: bool,
    interrupt_output_deferred: bool,
    tx_load_size: usize,
    tx_queue: [u8; UART_TX_QUEUE_SIZE],
    tx_head: usize,
    tx_tail: usize,
    tx_queued: usize,
    tx_queue_overflow: bool,
    tx_irq_kicks: usize,
    tx_irq_drains: usize,
    tx_irq_budget_hits: usize,
    tx_irq_empty_stop: usize,
    tx_irq_guarded_by_local_irq_save: bool,
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
            ier_addr: 0,
            iir_addr: 0,
            mcr_addr: 0,
            lsr_addr: 0,
            tx_offset: 0,
            ier_offset: 0,
            iir_offset: 0,
            mcr_offset: 0,
            lsr_offset: 0,
            reg_shift: 0,
            reg_io_width: 0,
            lsr_thre_mask: 0,
            uses_uart_membase: false,
            uses_lsr_thr_polling: false,
            uses_sbi: false,
            interrupt_driven: false,
            interrupt_output_deferred: false,
            tx_load_size: 0,
            tx_queue: [0; UART_TX_QUEUE_SIZE],
            tx_head: 0,
            tx_tail: 0,
            tx_queued: 0,
            tx_queue_overflow: false,
            tx_irq_kicks: 0,
            tx_irq_drains: 0,
            tx_irq_budget_hits: 0,
            tx_irq_empty_stop: 0,
            tx_irq_guarded_by_local_irq_save: false,
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
        let ier_offset = uart_register_offset(port, UART_IER)?;
        let iir_offset = uart_register_offset(port, UART_IIR)?;
        let mcr_offset = uart_register_offset(port, UART_MCR)?;
        let lsr_offset = uart_register_offset(port, UART_LSR)?;
        let tx_addr = port.membase.checked_add(tx_offset)?;
        let ier_addr = port.membase.checked_add(ier_offset)?;
        let iir_addr = port.membase.checked_add(iir_offset)?;
        let mcr_addr = port.membase.checked_add(mcr_offset)?;
        let lsr_addr = port.membase.checked_add(lsr_offset)?;
        Some(Self {
            ready: true,
            device_ref: port.device_ref,
            membase: port.membase,
            tx_addr,
            ier_addr,
            iir_addr,
            mcr_addr,
            lsr_addr,
            tx_offset,
            ier_offset,
            iir_offset,
            mcr_offset,
            lsr_offset,
            reg_shift: port.reg_shift,
            reg_io_width: port.reg_io_width,
            lsr_thre_mask: UART_LSR_THRE,
            uses_uart_membase: tx_addr == port.membase && lsr_addr > port.membase,
            uses_lsr_thr_polling: true,
            uses_sbi: false,
            interrupt_driven: false,
            interrupt_output_deferred: true,
            tx_load_size: UART_TX_LOAD_SIZE,
            tx_queue: [0; UART_TX_QUEUE_SIZE],
            tx_head: 0,
            tx_tail: 0,
            tx_queued: 0,
            tx_queue_overflow: false,
            tx_irq_kicks: 0,
            tx_irq_drains: 0,
            tx_irq_budget_hits: 0,
            tx_irq_empty_stop: 0,
            tx_irq_guarded_by_local_irq_save: false,
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
        if !self.ready || self.uses_sbi {
            return false;
        }

        self.write_calls = self.write_calls.saturating_add(1);
        self.bytes_accepted = self.bytes_accepted.saturating_add(bytes.len());
        if self.interrupt_driven {
            return self.enqueue_console_bytes_and_kick(bytes);
        }

        if !self.uses_lsr_thr_polling {
            return false;
        }
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

    fn enable_interrupt_driven(&mut self) -> bool {
        if !self.ready
            || !self.uses_uart_membase
            || self.uses_sbi
            || self.interrupt_driven
            || !self.interrupt_output_deferred
        {
            return false;
        }

        self.uses_lsr_thr_polling = false;
        self.interrupt_driven = true;
        self.interrupt_output_deferred = false;
        self.tx_head = 0;
        self.tx_tail = 0;
        self.tx_queued = 0;
        self.tx_queue_overflow = false;
        self.tx_irq_kicks = 0;
        self.tx_irq_drains = 0;
        self.tx_irq_budget_hits = 0;
        self.tx_irq_empty_stop = 0;
        self.tx_irq_guarded_by_local_irq_save = false;
        trace::checkpoint(Checkpoint::Serial8250ConsoleIrqDrivenReady);
        true
    }

    fn enqueue_console_bytes_and_kick(&mut self, bytes: &[u8]) -> bool {
        let saved = csr::save_and_disable_supervisor_interrupts();
        self.tx_irq_guarded_by_local_irq_save = true;
        let mut ok = true;
        for byte in bytes {
            if *byte == b'\n' {
                ok &= self.enqueue_tx_byte(b'\r');
                if ok {
                    self.crlf_insertions = self.crlf_insertions.saturating_add(1);
                }
            }
            ok &= self.enqueue_tx_byte(*byte);
        }
        let kicked = ok && self.kick_tx_interrupt_locked();
        csr::restore_supervisor_interrupts(saved);
        ok && kicked
    }

    fn enqueue_tx_byte(&mut self, byte: u8) -> bool {
        if self.tx_queued == UART_TX_QUEUE_SIZE {
            self.tx_queue_overflow = true;
            return false;
        }

        self.tx_queue[self.tx_tail] = byte;
        self.tx_tail = (self.tx_tail + 1) % UART_TX_QUEUE_SIZE;
        self.tx_queued += 1;
        true
    }

    fn pop_tx_byte(&mut self) -> Option<u8> {
        if self.tx_queued == 0 {
            return None;
        }

        let byte = self.tx_queue[self.tx_head];
        self.tx_head = (self.tx_head + 1) % UART_TX_QUEUE_SIZE;
        self.tx_queued -= 1;
        Some(byte)
    }

    fn kick_tx_interrupt_locked(&mut self) -> bool {
        if self.tx_queued == 0 {
            return true;
        }
        if !self.kick_tx_interrupt_hw_locked() {
            return false;
        }
        self.tx_irq_kicks = self.tx_irq_kicks.saturating_add(1);
        true
    }

    #[cfg(checkpoint_handler_uart_irq_chain)]
    fn kick_ordinary_tty_tx_interrupt_locked(&mut self) -> bool {
        self.kick_tx_interrupt_hw_locked()
    }

    fn kick_tx_interrupt_hw_locked(&mut self) -> bool {
        let mcr = self.read_uart_mcr();
        if !self.write_uart_mcr(mcr | UART_MCR_OUT2) {
            return false;
        }
        let ier = self.read_uart_ier();
        if !self.write_uart_ier(ier | UART_IER_THRI) {
            return false;
        }
        UART8250_THRE_INTERRUPT_REQUESTS.fetch_add(1, Ordering::AcqRel);
        true
    }

    fn drain_one_tx_irq(&mut self) -> bool {
        if !self.interrupt_driven || self.tx_load_size == 0 {
            return false;
        }
        if self.tx_queued == 0 {
            let ier = self.read_uart_ier() & !UART_IER_THRI;
            if self.write_uart_ier(ier) {
                self.tx_irq_empty_stop = self.tx_irq_empty_stop.saturating_add(1);
                UART8250_THRI_DISABLED_BY_HANDLER.fetch_add(1, Ordering::AcqRel);
                return true;
            }
            return false;
        }

        let mut drained = 0usize;
        while self.tx_queued != 0 && drained < self.tx_load_size {
            if self.read_uart_lsr() & UART_LSR_THRE == 0 {
                return true;
            }
            let Some(byte) = self.pop_tx_byte() else {
                break;
            };
            if !self.write_uart_tx(byte) {
                return false;
            }
            self.tx_bytes_submitted = self.tx_bytes_submitted.saturating_add(1);
            self.last_tx_byte = byte;
            self.mmio_writes_performed = true;
            self.tx_irq_drains = self.tx_irq_drains.saturating_add(1);
            drained = drained.saturating_add(1);
        }

        if self.tx_queued != 0 {
            self.tx_irq_budget_hits = self.tx_irq_budget_hits.saturating_add(1);
            return true;
        }

        let ier = self.read_uart_ier() & !UART_IER_THRI;
        if self.write_uart_ier(ier) {
            self.tx_irq_empty_stop = self.tx_irq_empty_stop.saturating_add(1);
            UART8250_THRI_DISABLED_BY_HANDLER.fetch_add(1, Ordering::AcqRel);
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
        self.read_uart_reg(self.lsr_addr)
    }

    fn read_uart_rx(&self) -> usize {
        self.read_uart_reg(self.membase + (UART_RX << self.reg_shift))
    }

    fn read_uart_iir(&self) -> usize {
        self.read_uart_reg(self.iir_addr)
    }

    fn read_uart_ier(&self) -> usize {
        self.read_uart_reg(self.ier_addr)
    }

    fn read_uart_mcr(&self) -> usize {
        self.read_uart_reg(self.mcr_addr)
    }

    fn write_uart_ier(&self, value: usize) -> bool {
        self.write_uart_reg(self.ier_addr, value)
    }

    fn write_uart_fcr(&self, value: usize) -> bool {
        self.write_uart_reg(self.membase + (UART_FCR << self.reg_shift), value)
    }

    fn write_uart_mcr(&self, value: usize) -> bool {
        self.write_uart_reg(self.mcr_addr, value)
    }

    fn read_uart_reg(&self, addr: usize) -> usize {
        match self.reg_io_width {
            1 => unsafe { core::ptr::read_volatile(addr as *const u8) as usize },
            2 => unsafe { core::ptr::read_volatile(addr as *const u16) as usize },
            4 => unsafe { core::ptr::read_volatile(addr as *const u32) as usize },
            _ => 0,
        }
    }

    fn write_uart_tx(&self, byte: u8) -> bool {
        self.write_uart_reg(self.tx_addr, byte as usize)
    }

    fn write_uart_reg(&self, addr: usize, value: usize) -> bool {
        match self.reg_io_width {
            1 => unsafe { core::ptr::write_volatile(addr as *mut u8, value as u8) },
            2 => unsafe { core::ptr::write_volatile(addr as *mut u16, value as u16) },
            4 => unsafe { core::ptr::write_volatile(addr as *mut u32, value as u32) },
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
            && self.ier_addr == port.membase.saturating_add(self.ier_offset)
            && self.iir_addr == port.membase.saturating_add(self.iir_offset)
            && self.mcr_addr == port.membase.saturating_add(self.mcr_offset)
            && self.lsr_addr == port.membase.saturating_add(self.lsr_offset)
            && self.tx_offset == 0
            && self.ier_offset == (UART_IER << port.reg_shift)
            && self.iir_offset == (UART_IIR << port.reg_shift)
            && self.mcr_offset == (UART_MCR << port.reg_shift)
            && self.lsr_offset == (UART_LSR << port.reg_shift)
            && self.reg_shift == port.reg_shift
            && self.reg_io_width == port.reg_io_width
            && self.lsr_thre_mask == UART_LSR_THRE
            && self.uses_uart_membase
            && self.uses_lsr_thr_polling
            && !self.uses_sbi
            && !self.interrupt_driven
            && self.interrupt_output_deferred
            && self.tx_load_size == UART_TX_LOAD_SIZE
    }

    #[cfg(checkpoint_handler_uart_irq_chain)]
    fn irq_driven_facts_ready(self, port: Uart8250Port) -> bool {
        self.ready
            && port.registered
            && port.interrupt_driven_ready
            && self.device_ref == port.device_ref
            && self.membase == port.membase
            && self.tx_addr == port.membase
            && self.uses_uart_membase
            && !self.uses_lsr_thr_polling
            && !self.uses_sbi
            && self.interrupt_driven
            && !self.interrupt_output_deferred
            && self.tx_load_size == UART_TX_LOAD_SIZE
            && self.tx_irq_guarded_by_local_irq_save
            && !self.tx_queue_overflow
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

#[allow(dead_code)]
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
        && state.port.irq_resource_ready
        && state.port.irq_parent_plic
        && state.port.irq_source != 0
        && state.port.irq_mapping_ready
        && state.port.logical_irq.is_valid()
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

pub fn uart8250_port_irq_resource_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered
        && state.port.irq_resource_ready
        && state.port.irq_parent_plic
        && state.port.irq_source != 0
}

pub fn uart8250_port_logical_irq_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered && state.port.irq_mapping_ready && state.port.logical_irq.is_valid()
}

pub fn uart8250_port_irq_source() -> u32 {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .port
            .irq_source
    }
}

pub fn uart8250_port_logical_irq() -> LogicalIrq {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .port
            .logical_irq
    }
}

pub fn uart8250_interrupt_output_still_deferred() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered && state.port.interrupt_output_deferred
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn uart8250_interrupt_driven_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered
        && state.port.interrupt_driven_ready
        && !state.port.interrupt_output_deferred
        && state.runtime_port.facts_ready(
            state.port,
            state.tty_port,
            state.tty_flip_buffer,
            state.tty_xmit_fifo,
        )
        && state.runtime_port.console_tx_interrupt_driven
        && state.write_backend.irq_driven_facts_ready(state.port)
}

pub fn uart8250_interrupt_driven_configured() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered
        && state.port.interrupt_driven_ready
        && !state.port.interrupt_output_deferred
        && state.runtime_port.facts_ready(
            state.port,
            state.tty_port,
            state.tty_flip_buffer,
            state.tty_xmit_fifo,
        )
        && state.runtime_port.console_tx_interrupt_driven
        && state.write_backend.ready
        && state.write_backend.interrupt_driven
        && !state.write_backend.interrupt_output_deferred
}

pub fn uart8250_interrupt_trigger_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered && state.port.interrupt_trigger_ready
}

pub fn uart8250_thre_interrupt_handled() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered && state.port.thre_interrupt_handled
}

pub fn uart8250_irq_handler_registered() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered
        && state.port.irq_handler_registered
        && state.port.irq_handler_hardirq_context_required
        && state.port.irq_handler_dispatch_ready
}

pub fn uart8250_irq_handler_hardirq_context_required() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered && state.port.irq_handler_hardirq_context_required
}

pub fn uart8250_irq_handler_dispatch_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.port.registered && state.port.irq_handler_dispatch_ready
}

pub fn uart8250_irq_handler_call_count() -> usize {
    UART8250_IRQ_HANDLER_CALLS.load(Ordering::Acquire)
}

pub fn uart8250_thre_interrupt_request_count() -> usize {
    UART8250_THRE_INTERRUPT_REQUESTS.load(Ordering::Acquire)
}

pub fn uart8250_thre_interrupt_handled_count() -> usize {
    UART8250_THRE_INTERRUPT_HANDLED.load(Ordering::Acquire)
}

pub fn uart8250_thri_disabled_by_handler_count() -> usize {
    UART8250_THRI_DISABLED_BY_HANDLER.load(Ordering::Acquire)
}

pub fn uart8250_rx_interrupt_request_count() -> usize {
    UART8250_RX_INTERRUPT_REQUESTS.load(Ordering::Acquire)
}

pub fn uart8250_rx_interrupt_handled_count() -> usize {
    UART8250_RX_INTERRUPT_HANDLED.load(Ordering::Acquire)
}

pub fn serial8250_tx_queue_len() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .tx_queued
    }
}

pub fn serial8250_tx_irq_kick_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .tx_irq_kicks
    }
}

pub fn serial8250_tx_irq_drain_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .tx_irq_drains
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn serial8250_tx_irq_budget_hit_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .tx_irq_budget_hits
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn serial8250_tx_irq_empty_stop_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .tx_irq_empty_stop
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn serial8250_tx_byte_count_available_for_irq_probe() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .tx_bytes_submitted
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn serial8250_tx_crlf_insertion_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .crlf_insertions
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn serial8250_last_tx_byte() -> u8 {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .last_tx_byte
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn serial8250_tx_queue_overflowed() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .tx_queue_overflow
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn serial8250_tx_queue_guarded_by_local_irq_save() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .write_backend
            .tx_irq_guarded_by_local_irq_save
    }
}

pub fn tty_port_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_port.facts_ready(state.port)
}

pub fn tty_port_not_backend_owner() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_port.ready
        && state.tty_port.no_mmio_access
        && state.tty_port.no_irq_dispatch
        && state.tty_port.no_console_registry_policy
}

pub fn tty_flip_buffer_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_flip_buffer.facts_ready(state.tty_port)
}

pub fn tty_flip_buffer_empty() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_flip_buffer.ready
        && state.tty_flip_buffer.pending_len == 0
        && state.tty_flip_buffer.read_ready_len == 0
        && state.tty_flip_buffer.read_offset == 0
        && state.tty_flip_buffer.read_count == 0
        && state.tty_flip_buffer.push_count == 0
        && state.tty_flip_buffer.total_inserted == 0
        && state.tty_flip_buffer.last_pushed_len == 0
        && state.tty_flip_buffer.last_read_len == 0
        && state.tty_flip_buffer.last_byte == 0
        && !state.tty_flip_buffer.overflowed
        && state.tty_flip_buffer.buffer[0] == 0
}

pub fn tty_flip_buffer_pushed() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_flip_buffer.ready
        && state.tty_flip_buffer.pending_len == 0
        && state.tty_flip_buffer.push_count != 0
        && state.tty_flip_buffer.total_inserted != 0
        && state.tty_flip_buffer.last_pushed_len != 0
        && !state.tty_flip_buffer.overflowed
}

pub fn tty_flip_buffer_push_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_flip_buffer
            .push_count
    }
}

pub fn tty_flip_buffer_total_inserted() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_flip_buffer
            .total_inserted
    }
}

pub fn tty_flip_buffer_last_pushed_len() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_flip_buffer
            .last_pushed_len
    }
}

pub fn tty_flip_buffer_last_byte() -> u8 {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_flip_buffer
            .last_byte
    }
}

pub fn tty_flip_buffer_overflowed() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_flip_buffer
            .overflowed
    }
}

pub fn tty_flip_buffer_ready_data_bound() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_flip_buffer.ready
        && state.tty_flip_buffer.read_ready_len != 0
        && state.tty_flip_buffer.read_offset == 0
        && !state.tty_flip_buffer.overflowed
}

pub fn tty_flip_buffer_ready_data_consumed() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_flip_buffer.ready
        && state.tty_flip_buffer.read_count != 0
        && state.tty_flip_buffer.last_read_len != 0
        && state.tty_flip_buffer.read_ready_len == 0
        && state.tty_flip_buffer.read_offset == 0
        && !state.tty_flip_buffer.overflowed
}

pub fn seed_tty_ready_data_fixture(bytes: &[u8]) -> bool {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    state.tty_flip_buffer.seed_ready_data_fixture(bytes)
}

pub fn clear_tty_ready_data() -> bool {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    state.tty_flip_buffer.clear_ready_data()
}

pub fn read_tty_ready_data(buffer: &mut [u8]) -> Option<usize> {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    state.tty_flip_buffer.read_ready_data(buffer)
}

pub fn read_tty_ready_data_with_mode(buffer: &mut [u8], canonical: bool) -> Option<usize> {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    state
        .tty_flip_buffer
        .read_ready_data_with_mode(buffer, canonical)
}

pub fn tty_ready_data_available() -> Option<bool> {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_flip_buffer.read_ready_available()
}

pub fn tty_ready_data_available_with_mode(canonical: bool) -> Option<bool> {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state
        .tty_flip_buffer
        .read_ready_available_with_mode(canonical)
}

pub fn tty_xmit_fifo_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_xmit_fifo.facts_ready(state.tty_port)
}

pub fn tty_xmit_fifo_deferred_from_console_tx() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_xmit_fifo.ready
        && state.tty_xmit_fifo.distinct_from_printk_console_tx
        && state.tty_xmit_fifo.runtime_tx_integration_deferred
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn tty_xmit_fifo_runtime_tx_integrated() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_xmit_fifo.facts_ready(state.tty_port)
        && state.tty_xmit_fifo.distinct_from_printk_console_tx
        && !state.tty_xmit_fifo.runtime_tx_integration_deferred
        && state.tty_xmit_fifo.runtime_tx_integrated
}

pub fn probe_tty_xmit_fifo_round_trip(byte: u8) -> bool {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    if !state.tty_xmit_fifo.facts_ready(state.tty_port) {
        return false;
    }
    if !state.tty_xmit_fifo.enqueue(byte) {
        return false;
    }
    state.tty_xmit_fifo.dequeue_for_tx() == Some(byte)
}

pub fn tty_xmit_fifo_round_trip_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.tty_xmit_fifo.facts_ready(state.tty_port)
        && state.tty_xmit_fifo.enqueue_count != 0
        && state.tty_xmit_fifo.dequeue_count != 0
        && state.tty_xmit_fifo.queued == 0
        && state.tty_xmit_fifo.last_enqueued == state.tty_xmit_fifo.last_dequeued
        && !state.tty_xmit_fifo.overflowed
        && !state.tty_xmit_fifo.underflowed
}

pub fn tty_xmit_fifo_queue_len() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_xmit_fifo
            .queued
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub const fn tty_xmit_fifo_capacity() -> usize {
    TTY_XMIT_FIFO_SIZE
}

pub fn tty_xmit_fifo_enqueue_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_xmit_fifo
            .enqueue_count
    }
}

pub fn tty_xmit_fifo_dequeue_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_xmit_fifo
            .dequeue_count
    }
}

pub fn tty_xmit_fifo_last_enqueued() -> u8 {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_xmit_fifo
            .last_enqueued
    }
}

pub fn tty_xmit_fifo_last_dequeued() -> u8 {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_xmit_fifo
            .last_dequeued
    }
}

pub fn tty_xmit_fifo_overflowed() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_xmit_fifo
            .overflowed
    }
}

pub fn tty_xmit_fifo_underflowed() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_xmit_fifo
            .underflowed
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn tty_xmit_fifo_runtime_tx_kick_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_xmit_fifo
            .runtime_tx_kicks
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn tty_xmit_fifo_runtime_tx_drain_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_xmit_fifo
            .runtime_tx_drains
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn tty_xmit_fifo_runtime_tx_empty_stop_count() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_xmit_fifo
            .runtime_tx_empty_stops
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn tty_xmit_fifo_runtime_tx_guarded_by_local_irq_save() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .tty_xmit_fifo
            .runtime_tx_guarded_by_local_irq_save
    }
}

pub fn serial8250_runtime_port_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.runtime_port.facts_ready(
        state.port,
        state.tty_port,
        state.tty_flip_buffer,
        state.tty_xmit_fifo,
    )
}

pub fn serial8250_runtime_rx_deferred() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.runtime_port.ready
        && !state.runtime_port.online
        && !state.runtime_port.rdi_enabled
        && !state.runtime_port.rlsi_enabled
        && state.runtime_port.rx_interrupts_deferred_until_enable
}

pub fn serial8250_runtime_console_tx_ready() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.runtime_port.ready && state.runtime_port.console_tx_interrupt_driven
}

pub fn serial8250_runtime_rx_enabled() -> bool {
    let state = unsafe { (&raw const NS16550A_PROBE_STATE).as_ref().unwrap() };
    state.runtime_port.ready
        && state.runtime_port.online
        && state.runtime_port.rdi_enabled
        && state.runtime_port.rlsi_enabled
        && state.runtime_port.rx_fifo_enabled
        && !state.runtime_port.rx_interrupts_deferred_until_enable
        && state.tty_port.ready
        && state.tty_port.initialized
        && state.tty_port.online
}

pub fn serial8250_runtime_rx_fifo_enabled() -> bool {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .runtime_port
            .rx_fifo_enabled
    }
}

pub fn serial8250_runtime_rx_drain_limit() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .runtime_port
            .rx_drain_limit
    }
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn serial8250_runtime_tx_load_size() -> usize {
    unsafe {
        (&raw const NS16550A_PROBE_STATE)
            .as_ref()
            .unwrap()
            .runtime_port
            .tx_load_size
    }
}

pub fn serial8250_runtime_rx_last_byte() -> u8 {
    tty_flip_buffer_last_byte()
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn uart8250_last_iir() -> usize {
    UART8250_LAST_IIR.load(Ordering::Acquire)
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn uart8250_last_lsr() -> usize {
    UART8250_LAST_LSR.load(Ordering::Acquire)
}

pub fn trigger_uart8250_thre_interrupt_once() -> bool {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    if !state.port.registered
        || !state.port.irq_handler_registered
        || state.port.thre_interrupt_enabled
        || state.port.thre_interrupt_handled
        || !state.write_backend.facts_ready(state.port)
    {
        return false;
    }

    let mut spins = 0usize;
    while state.write_backend.read_uart_lsr() & UART_LSR_THRE == 0 {
        if spins == UART_POLL_SPINS {
            return false;
        }
        core::hint::spin_loop();
        spins += 1;
    }

    let mcr = state.write_backend.read_uart_mcr();
    state.port.interrupt_trigger_ready = true;
    state.port.thre_interrupt_enabled = true;
    state.port.thre_interrupt_loopback = false;
    if !state.write_backend.write_uart_mcr(mcr | UART_MCR_OUT2) {
        state.port.interrupt_trigger_ready = false;
        state.port.thre_interrupt_enabled = false;
        state.port.thre_interrupt_loopback = false;
        return false;
    }

    let ier = state.write_backend.read_uart_ier() | UART_IER_THRI;
    if !state.write_backend.write_uart_ier(ier) {
        state.port.interrupt_trigger_ready = false;
        state.port.thre_interrupt_enabled = false;
        state.port.thre_interrupt_loopback = false;
        return false;
    }
    UART8250_THRE_INTERRUPT_REQUESTS.fetch_add(1, Ordering::AcqRel);
    true
}

pub fn enable_serial8250_interrupt_driven_console() -> bool {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    if !state.serial_console_registered
        || !state.handoff_triggered
        || !state.port.registered
        || !state.port.irq_handler_registered
        || !state.write_backend.facts_ready(state.port)
    {
        return false;
    }
    if !state.runtime_port.ready || state.runtime_port.console_tx_interrupt_driven {
        return false;
    }

    if !state.write_backend.enable_interrupt_driven() {
        return false;
    }
    state.runtime_port.console_tx_interrupt_driven = true;
    state.port.interrupt_output_deferred = false;
    state.port.interrupt_driven_ready = true;
    true
}

pub fn enable_serial8250_runtime_rx() -> bool {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    if !state.port.registered
        || !state.port.irq_handler_registered
        || !state.port.interrupt_driven_ready
        || !state.write_backend.ready
        || !state.write_backend.interrupt_driven
        || state.write_backend.interrupt_output_deferred
        || !state.runtime_port.facts_ready(
            state.port,
            state.tty_port,
            state.tty_flip_buffer,
            state.tty_xmit_fifo,
        )
    {
        return false;
    }

    if state.tty_port.initialized
        || state.tty_port.online
        || state.runtime_port.online
        || state.runtime_port.rdi_enabled
        || state.runtime_port.rlsi_enabled
    {
        return false;
    }

    let fcr = UART_FCR_ENABLE_FIFO | UART_FCR_CLEAR_RCVR | UART_FCR_CLEAR_XMIT;
    if !state.write_backend.write_uart_fcr(fcr)
        || !state.tty_port.enable_for_uart_startup()
        || !state.runtime_port.enable_rx_runtime(state.tty_port)
    {
        return false;
    }
    let ier = state.write_backend.read_uart_ier() | UART_IER_RDI | UART_IER_RLSI;
    state.write_backend.write_uart_ier(ier)
}

pub fn trigger_serial8250_rx_loopback_once(byte: u8) -> bool {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    if !state.port.registered
        || !state.port.irq_handler_registered
        || !state.write_backend.interrupt_driven
        || !state.runtime_port.online
        || !state.runtime_port.rdi_enabled
        || !state.runtime_port.rlsi_enabled
        || state.tty_flip_buffer.push_count != 0
    {
        return false;
    }

    let saved = csr::save_and_disable_supervisor_interrupts();
    let mcr = state.write_backend.read_uart_mcr();
    let ok = state.write_backend.wait_for_tx_ready()
        && state
            .write_backend
            .write_uart_mcr(mcr | UART_MCR_OUT2 | UART_MCR_LOOP)
        && state.write_backend.write_uart_tx(byte)
        && state.write_backend.write_uart_mcr(mcr);
    csr::restore_supervisor_interrupts(saved);
    if ok {
        UART8250_RX_INTERRUPT_REQUESTS.fetch_add(1, Ordering::AcqRel);
    }
    ok
}

pub fn trigger_serial8250_rx_loopback_batch(bytes: &[u8]) -> bool {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    if bytes.is_empty()
        || bytes.len() > state.runtime_port.rx_drain_limit
        || bytes.len() > TTY_FLIP_BUFFER_SIZE
        || !state.port.registered
        || !state.port.irq_handler_registered
        || !state.write_backend.interrupt_driven
        || !state.runtime_port.online
        || !state.runtime_port.rdi_enabled
        || !state.runtime_port.rlsi_enabled
        || !state.runtime_port.rx_fifo_enabled
        || state.tty_flip_buffer.push_count == 0
        || state.tty_flip_buffer.pending_len != 0
        || state.tty_flip_buffer.overflowed
    {
        return false;
    }

    let saved = csr::save_and_disable_supervisor_interrupts();
    let mcr = state.write_backend.read_uart_mcr();
    let fcr = UART_FCR_ENABLE_FIFO | UART_FCR_CLEAR_RCVR | UART_FCR_CLEAR_XMIT;
    let mut ok = state.write_backend.wait_for_tx_ready()
        && state.write_backend.write_uart_fcr(fcr)
        && state
            .write_backend
            .write_uart_mcr(mcr | UART_MCR_OUT2 | UART_MCR_LOOP);
    for byte in bytes {
        ok = ok && state.write_backend.write_uart_tx(*byte);
    }
    ok = ok && state.write_backend.write_uart_mcr(mcr);
    csr::restore_supervisor_interrupts(saved);
    if ok {
        UART8250_RX_INTERRUPT_REQUESTS.fetch_add(1, Ordering::AcqRel);
    }
    ok
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn start_tty_xmit_fifo_runtime_tx(byte: u8) -> bool {
    start_tty_xmit_fifo_runtime_tx_bytes(&[byte])
}

#[cfg(checkpoint_handler_uart_irq_chain)]
pub fn start_tty_xmit_fifo_runtime_tx_batch(bytes: &[u8]) -> bool {
    start_tty_xmit_fifo_runtime_tx_bytes(bytes)
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn start_tty_xmit_fifo_runtime_tx_bytes(bytes: &[u8]) -> bool {
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    if bytes.is_empty()
        || bytes.len() > TTY_XMIT_FIFO_SIZE
        || !state.port.registered
        || !state.port.irq_handler_registered
        || !state.port.interrupt_driven_ready
        || !state.write_backend.interrupt_driven
        || !state.runtime_port.online
        || !state.runtime_port.thri_demand_driven
        || !state.tty_port.online
        || !state.tty_xmit_fifo.facts_ready(state.tty_port)
        || state.tty_xmit_fifo.queued != 0
        || state.tty_xmit_fifo.overflowed
        || state.tty_xmit_fifo.underflowed
        || state.write_backend.tx_queued != 0
        || state.write_backend.tx_queue_overflow
        || state.port.thre_interrupt_enabled
    {
        return false;
    }

    let saved = csr::save_and_disable_supervisor_interrupts();
    state.tty_xmit_fifo.runtime_tx_guarded_by_local_irq_save = true;
    let mut ok = true;
    for byte in bytes {
        ok = ok && state.tty_xmit_fifo.enqueue(*byte);
    }
    ok = ok && state.write_backend.kick_ordinary_tty_tx_interrupt_locked();
    if ok {
        state.tty_xmit_fifo.mark_runtime_tx_integrated();
        state.tty_xmit_fifo.runtime_tx_kicks =
            state.tty_xmit_fifo.runtime_tx_kicks.saturating_add(1);
        state.port.thre_interrupt_enabled = true;
        state.port.thre_interrupt_handled = false;
    }
    csr::restore_supervisor_interrupts(saved);
    ok
}

pub fn handle_uart_irq() {
    UART8250_IRQ_HANDLER_CALLS.fetch_add(1, Ordering::AcqRel);
    let state = unsafe { (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap() };
    if !state.port.registered {
        return;
    }

    let iir = state.write_backend.read_uart_iir();
    let lsr = state.write_backend.read_uart_lsr();
    UART8250_LAST_IIR.store(iir, Ordering::Release);
    UART8250_LAST_LSR.store(lsr, Ordering::Release);
    if iir & UART_IIR_NO_INT != 0 {
        return;
    }

    let interrupt_id = iir & UART_IIR_ID;
    if is_uart_rx_interrupt(interrupt_id) && state.runtime_port.online {
        if handle_rx_chars(state, lsr) {
            UART8250_RX_INTERRUPT_HANDLED.fetch_add(1, Ordering::AcqRel);
        }
    }

    if interrupt_id != UART_IIR_THRI
        || lsr & UART_LSR_THRE == 0
        || !state.port.thre_interrupt_enabled
    {
        return;
    }

    if state.write_backend.interrupt_driven && state.write_backend.tx_queued != 0 {
        if !state.write_backend.drain_one_tx_irq() {
            return;
        }
        state.port.thre_interrupt_enabled = state.write_backend.tx_queued != 0;
        state.port.thre_interrupt_handled = state.write_backend.tx_queued == 0;
        UART8250_THRE_INTERRUPT_HANDLED.fetch_add(1, Ordering::AcqRel);
        return;
    }

    #[cfg(checkpoint_handler_uart_irq_chain)]
    if state.write_backend.interrupt_driven
        && state.tty_xmit_fifo.runtime_tx_integrated
        && state.tty_xmit_fifo.queued != 0
    {
        if !drain_tty_xmit_fifo_irq(state) {
            return;
        }
        state.port.thre_interrupt_enabled = state.tty_xmit_fifo.queued != 0;
        state.port.thre_interrupt_handled = state.tty_xmit_fifo.queued == 0;
        UART8250_THRE_INTERRUPT_HANDLED.fetch_add(1, Ordering::AcqRel);
        return;
    }

    if state.write_backend.interrupt_driven {
        if !state.write_backend.drain_one_tx_irq() {
            return;
        }
        state.port.thre_interrupt_enabled = state.write_backend.tx_queued != 0;
        state.port.thre_interrupt_handled = state.write_backend.tx_queued == 0;
        UART8250_THRE_INTERRUPT_HANDLED.fetch_add(1, Ordering::AcqRel);
        return;
    }

    let ier = state.write_backend.read_uart_ier() & !UART_IER_THRI;
    if state.write_backend.write_uart_ier(ier) {
        if state.port.thre_interrupt_loopback {
            let mcr = state.write_backend.read_uart_mcr();
            let _ = state
                .write_backend
                .write_uart_mcr((mcr | UART_MCR_OUT2) & !UART_MCR_LOOP);
            if state.write_backend.read_uart_lsr() & UART_LSR_DR != 0 {
                let _ = state.write_backend.read_uart_rx();
            }
            state.port.thre_interrupt_loopback = false;
        }
        state.port.thre_interrupt_enabled = false;
        state.port.thre_interrupt_handled = true;
        UART8250_THRE_INTERRUPT_HANDLED.fetch_add(1, Ordering::AcqRel);
        UART8250_THRI_DISABLED_BY_HANDLER.fetch_add(1, Ordering::AcqRel);
    }
}

fn is_uart_rx_interrupt(interrupt_id: usize) -> bool {
    matches!(
        interrupt_id,
        UART_IIR_RDI | UART_IIR_RLSI | UART_IIR_RX_TIMEOUT
    )
}

#[cfg(checkpoint_handler_uart_irq_chain)]
fn drain_tty_xmit_fifo_irq(state: &mut Ns16550aProbeState) -> bool {
    if !state.write_backend.interrupt_driven || !state.tty_xmit_fifo.runtime_tx_integrated {
        return false;
    }

    while state.tty_xmit_fifo.queued != 0 {
        if state.write_backend.read_uart_lsr() & UART_LSR_THRE == 0 {
            return true;
        }
        let Some(byte) = state.tty_xmit_fifo.dequeue_for_tx() else {
            return false;
        };
        if !state.write_backend.write_uart_tx(byte) {
            return false;
        }
        state.write_backend.tx_bytes_submitted =
            state.write_backend.tx_bytes_submitted.saturating_add(1);
        state.write_backend.last_tx_byte = byte;
        state.write_backend.mmio_writes_performed = true;
        state.tty_xmit_fifo.runtime_tx_drains =
            state.tty_xmit_fifo.runtime_tx_drains.saturating_add(1);
    }

    let ier = state.write_backend.read_uart_ier() & !UART_IER_THRI;
    if state.write_backend.write_uart_ier(ier) {
        state.tty_xmit_fifo.runtime_tx_empty_stops =
            state.tty_xmit_fifo.runtime_tx_empty_stops.saturating_add(1);
        UART8250_THRI_DISABLED_BY_HANDLER.fetch_add(1, Ordering::AcqRel);
        return true;
    }
    false
}

fn handle_rx_chars(state: &mut Ns16550aProbeState, initial_lsr: usize) -> bool {
    if !state.runtime_port.online
        || !state.runtime_port.rdi_enabled
        || !state.runtime_port.rlsi_enabled
        || !state.tty_flip_buffer.ready
    {
        return false;
    }

    let mut lsr = initial_lsr;
    let mut drained = 0usize;
    let mut inserted = false;
    while drained < state.runtime_port.rx_drain_limit && lsr & UART_LSR_DR != 0 {
        let byte = state.write_backend.read_uart_rx() as u8;
        if !state.tty_flip_buffer.insert_char(byte) {
            break;
        }
        inserted = true;
        drained += 1;
        lsr = state.write_backend.read_uart_lsr();
    }

    inserted && state.tty_flip_buffer.push()
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
    serial8250_write_call_count_available_for_irq_probe()
}

#[cfg(any(checkpoint_handler_console_handoff, checkpoint_handler_uart_irq_chain))]
pub fn serial8250_write_call_count_available_for_irq_probe() -> usize {
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
    context: &mut PlatformProbeContext<'_>,
    device: DeviceRef,
    node_id: DeviceNodeId,
) -> ProbeResult {
    let Some(mut port) = build_uart8250_port(context.device_tree(), device, node_id) else {
        return ProbeResult::Deferred;
    };
    let Some(mapping) = context.map_platform_device_mmio(device, port.mapbase, port.mapsize) else {
        return ProbeResult::Deferred;
    };
    bind_ioremap_mapping(&mut port, mapping);
    if !bind_irq_resource(&mut port, context) {
        return ProbeResult::Deferred;
    }
    if !bind_irq_handler(&mut port, context) {
        return ProbeResult::Deferred;
    }

    let stdout_path_available = context.device_tree().stdout_path_available();
    let stdout_path_matched =
        stdout_path_available && context.device_tree().stdout_path_selects(port.node_id);
    let serial_console_registered =
        stdout_path_matched && printk::register_serial8250_console(true);
    let write_backend = if serial_console_registered {
        Serial8250WriteBackend::from_port(port).unwrap_or(Serial8250WriteBackend::empty())
    } else {
        Serial8250WriteBackend::empty()
    };
    let tty_port = TtyPort::from_port(port);
    let tty_flip_buffer = TtyFlipBuffer::from_tty_port(tty_port);
    let tty_xmit_fifo = TtyXmitFifo::from_tty_port(tty_port);
    let runtime_port =
        Serial8250RuntimePort::from_port(port, tty_port, tty_flip_buffer, tty_xmit_fifo)
            .unwrap_or(Serial8250RuntimePort::empty());
    let handoff_triggered = serial_console_registered && printk::console_handoff_complete();

    unsafe {
        let state = (&raw mut NS16550A_PROBE_STATE).as_mut().unwrap();
        state.port = port;
        state.tty_port = tty_port;
        state.tty_flip_buffer = tty_flip_buffer;
        state.tty_xmit_fifo = tty_xmit_fifo;
        state.runtime_port = runtime_port;
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
    if !state.serial_console_registered
        || !(state.write_backend.facts_ready(state.port)
            || state.write_backend.interrupt_driven
                && state.write_backend.ready
                && state.port.interrupt_driven_ready)
    {
        return false;
    }
    let interrupt_driven = state.write_backend.interrupt_driven;
    if interrupt_driven {
        state.port.thre_interrupt_enabled = true;
        state.port.thre_interrupt_handled = false;
    }
    let delivered = state.write_backend.record_write(bytes);
    if delivered && interrupt_driven {
        state.port.thre_interrupt_enabled = state.write_backend.tx_queued != 0;
    } else if !delivered && interrupt_driven {
        state.port.thre_interrupt_enabled = false;
    }
    delivered
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
        irq_source: 0,
        logical_irq: LogicalIrq::invalid(),
        irq_resource_ready: false,
        irq_parent_plic: false,
        irq_mapping_ready: false,
        irq_handler_registered: false,
        irq_handler_hardirq_context_required: false,
        irq_handler_dispatch_ready: false,
        interrupt_output_deferred: true,
        interrupt_driven_ready: false,
        interrupt_trigger_ready: false,
        thre_interrupt_enabled: false,
        thre_interrupt_handled: false,
        thre_interrupt_loopback: false,
        line: device.index(),
        registered: true,
    })
}

fn bind_irq_resource(port: &mut Uart8250Port, context: &mut PlatformProbeContext<'_>) -> bool {
    let Some(node) = context.device_tree().node(port.node_id) else {
        return false;
    };
    if !interrupt_parent_is_plic(context.device_tree(), node) {
        return false;
    }
    let Some(source) = uart_interrupt_source(context, node) else {
        return false;
    };
    let Some(logical_irq) = context.map_plic_source(source) else {
        return false;
    };

    port.irq_source = source;
    port.logical_irq = logical_irq;
    port.irq_resource_ready = true;
    port.irq_parent_plic = true;
    port.irq_mapping_ready = context
        .plic_mapping_for_source(source)
        .is_some_and(|mapping| {
            mapping.logical_irq() == logical_irq
                && mapping.source_valid()
                && mapping.source_zero_rejected()
                && mapping.source_range_checked()
                && mapping.duplicate_source_idempotent()
                && mapping.source_gate_defined()
                && mapping.source_gate_closed()
                && mapping.source_enable_deferred()
                && mapping.source_not_enabled()
                && mapping.handler_not_registered()
        });
    port.interrupt_output_deferred = true;
    port.irq_mapping_ready
}

fn bind_irq_handler(port: &mut Uart8250Port, context: &mut PlatformProbeContext<'_>) -> bool {
    if !port.registered || !port.irq_mapping_ready || !port.logical_irq.is_valid() {
        return false;
    }
    if !context.request_irq(
        port.logical_irq,
        port.device_ref,
        IrqHandlerKind::Ns16550aUart,
    ) {
        return false;
    }
    let Some(action) = context.irq_action_for_logical_irq(port.logical_irq) else {
        return false;
    };

    port.irq_handler_registered = action.logical_irq() == port.logical_irq
        && action.device() == port.device_ref
        && action.handler_kind() == IrqHandlerKind::Ns16550aUart
        && action.handler_bound()
        && action.mapped_irq_required()
        && action.source_not_enabled();
    port.irq_handler_hardirq_context_required = action.hardirq_context_required();
    port.irq_handler_dispatch_ready = action.dispatch_ready();
    port.interrupt_output_deferred = true;
    port.irq_handler_registered
        && port.irq_handler_hardirq_context_required
        && port.irq_handler_dispatch_ready
}

fn uart_interrupt_source(
    context: &PlatformProbeContext<'_>,
    node: super::device_tree::DeviceNodeRef<'_>,
) -> Option<u32> {
    let value = node.property(b"interrupts")?.raw_value();
    let start = value.as_ptr() as usize;
    let source = read_be_u32(start, start.checked_add(value.len())?)?;
    context.translate_plic_one_cell_specifier(&[source])
}

fn interrupt_parent_is_plic(
    device_tree: &DeviceTree,
    node: super::device_tree::DeviceNodeRef<'_>,
) -> bool {
    let Some(phandle) = inherited_interrupt_parent_phandle(node) else {
        return false;
    };
    device_tree
        .root()
        .and_then(|root| find_node_by_phandle(root, phandle))
        .is_some_and(|parent| {
            parent.property(b"interrupt-controller").is_some()
                && (parent.has_compatible(PLIC_COMPATIBLE_SIFIVE)
                    || parent.has_compatible(PLIC_COMPATIBLE_RISCV))
        })
}

fn inherited_interrupt_parent_phandle(
    mut node: super::device_tree::DeviceNodeRef<'_>,
) -> Option<u32> {
    loop {
        if let Some(phandle) = read_property_u32(node.property(b"interrupt-parent")) {
            return Some(phandle);
        }
        node = node.parent()?;
    }
}

fn find_node_by_phandle<'dt>(
    node: super::device_tree::DeviceNodeRef<'dt>,
    phandle: u32,
) -> Option<super::device_tree::DeviceNodeRef<'dt>> {
    if read_property_u32(node.property(b"phandle")) == Some(phandle)
        || read_property_u32(node.property(b"linux,phandle")) == Some(phandle)
    {
        return Some(node);
    }
    for child in node.children() {
        if let Some(found) = find_node_by_phandle(child, phandle) {
            return Some(found);
        }
    }
    None
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
