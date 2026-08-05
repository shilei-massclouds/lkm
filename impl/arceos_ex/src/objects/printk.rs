use super::{
    boot_param::BootParam,
    earlycon,
    interrupt_type::InterruptType,
    irq_spinlock::IrqSpinLock,
    memblock::MemBlock,
    ns16550a,
    per_cpu_storage::PerCpuStorage,
    state::{EventResult, Lifecycle, LifecycleEvent, State, failed_condition},
};
use crate::{
    arch::riscv64::sbi,
    checkpoint::{self, Checkpoint},
};
use core::fmt::{self, Write};

#[allow(dead_code)]
const BUFFER_SIZE: usize = 4096;

#[allow(dead_code)]
static PRINTK_BUFFER: IrqSpinLock<PrintkBuffer> = IrqSpinLock::new(PrintkBuffer::new());
static CONSOLE_REGISTRY: IrqSpinLock<ConsoleRegistry> = IrqSpinLock::new(ConsoleRegistry::new());
static PRINTK_WRITE_LOCK: IrqSpinLock<()> = IrqSpinLock::new(());

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PrintkRoute {
    BufferOnly,
    BootConsole,
    Serial8250,
}

#[derive(Clone, Copy)]
pub struct ConsoleRegistry {
    boot_console_registered: bool,
    boot_console_online: bool,
    boot_console_unregistered: bool,
    boot_console_removed_from_registry: bool,
    serial8250_console_registered: bool,
    preferred_console_from_stdout: bool,
    serial8250_consdev: bool,
    serial8250_write_ready: bool,
    keep_bootcon: bool,
    handoff_complete: bool,
    boot_pending_flushed_before_serial_handoff: bool,
    legacy_earlycon_drain_blocked_after_handoff: bool,
    serial8250_online_trace_emitted: bool,
    boot_console_offline_trace_emitted: bool,
    serial8250_delivered_records_not_replayed: bool,
    route: PrintkRoute,
}

impl ConsoleRegistry {
    pub const fn new() -> Self {
        Self {
            boot_console_registered: false,
            boot_console_online: false,
            boot_console_unregistered: false,
            boot_console_removed_from_registry: false,
            serial8250_console_registered: false,
            preferred_console_from_stdout: false,
            serial8250_consdev: false,
            serial8250_write_ready: false,
            keep_bootcon: false,
            handoff_complete: false,
            boot_pending_flushed_before_serial_handoff: false,
            legacy_earlycon_drain_blocked_after_handoff: false,
            serial8250_online_trace_emitted: false,
            boot_console_offline_trace_emitted: false,
            serial8250_delivered_records_not_replayed: false,
            route: PrintkRoute::BufferOnly,
        }
    }

    fn register_boot_console(&mut self) {
        self.boot_console_registered = true;
        self.boot_console_online = true;
        self.boot_console_unregistered = false;
        self.boot_console_removed_from_registry = false;
        self.route = PrintkRoute::BootConsole;
    }

    fn register_serial8250_console(&mut self, preferred_from_stdout: bool) -> bool {
        if !self.boot_console_registered || !preferred_from_stdout {
            return false;
        }

        if self.handoff_complete {
            return true;
        }

        drain_buffer_to(sbi::putchar);
        self.boot_pending_flushed_before_serial_handoff = true;
        self.serial8250_console_registered = true;
        self.preferred_console_from_stdout = true;
        self.serial8250_consdev = true;
        self.serial8250_write_ready = true;
        self.route = PrintkRoute::Serial8250;
        self.handoff_complete = true;
        self.legacy_earlycon_drain_blocked_after_handoff = true;
        self.serial8250_online_trace_emitted = true;
        if !self.keep_bootcon {
            self.boot_console_online = false;
            self.boot_console_unregistered = true;
            self.boot_console_removed_from_registry = true;
            self.boot_console_offline_trace_emitted = true;
        }
        true
    }

    #[allow(dead_code)]
    fn set_keep_bootcon(&mut self, enabled: bool) {
        self.keep_bootcon = enabled;
    }
}

#[allow(dead_code)]
pub struct PrintkBuffer {
    lifecycle: Lifecycle,
    buffer: [u8; BUFFER_SIZE],
    read: usize,
    write: usize,
    runtime_ready: bool,
    percpu_data_ready: bool,
    records_preserved: bool,
    setup_local_irq_save_restore_used: bool,
    setup_local_irq_guard_bound_to_boot_cpu: bool,
}

#[allow(dead_code)]
impl PrintkBuffer {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            buffer: [0; BUFFER_SIZE],
            read: 0,
            write: 0,
            runtime_ready: false,
            percpu_data_ready: false,
            records_preserved: false,
            setup_local_irq_save_restore_used: false,
            setup_local_irq_guard_bound_to_boot_cpu: false,
        }
    }

    pub fn preset(&mut self) -> EventResult {
        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::PrintkBufferPrepared,
        )
    }

    pub fn setup(
        &mut self,
        memblock: &MemBlock,
        per_cpu_storage: &PerCpuStorage,
        boot_param: &BootParam,
        boot_cpu_local_interrupt: &mut InterruptType,
    ) -> EventResult {
        if self.lifecycle.state() != State::Prepared
            || memblock.state() != State::Online
            || per_cpu_storage.state() != State::Ready
            || boot_param.state() != State::Ready
            || boot_cpu_local_interrupt.local_state() != State::Ready
            || !boot_cpu_local_interrupt.disabled()
        {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        let read = self.read;
        let write = self.write;

        /*
         * PrintkBufferSetupLocalInterruptContext:
         * Linux setup_log_buf() uses local_irq_save()/local_irq_restore()
         * around the active printk buffer switch and initial record copy.
         */
        boot_cpu_local_interrupt.save_and_disable()?;
        self.percpu_data_ready = true;
        self.records_preserved = self.read == read && self.write == write;
        self.setup_local_irq_save_restore_used = true;
        self.setup_local_irq_guard_bound_to_boot_cpu = true;
        boot_cpu_local_interrupt.restore()?;
        self.runtime_ready = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::PrintkBufferReady,
        )
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        if self.lifecycle.state() != State::Prepared && self.lifecycle.state() != State::Ready {
            return;
        }

        for byte in bytes {
            let next = (self.write + 1) % BUFFER_SIZE;
            if next == self.read {
                self.read = (self.read + 1) % BUFFER_SIZE;
            }
            self.buffer[self.write] = *byte;
            self.write = next;
        }
    }

    pub fn drain_to(&mut self, mut sink: impl FnMut(u8)) {
        while self.read != self.write {
            let byte = self.buffer[self.read];
            self.read = (self.read + 1) % BUFFER_SIZE;
            sink(byte);
        }
    }

    pub fn discard_delivered(&mut self) {
        self.read = self.write;
    }

    pub fn is_prepared(&self) -> bool {
        self.lifecycle.state() == State::Prepared
    }

    pub fn is_ready(&self) -> bool {
        self.lifecycle.state() == State::Ready
            && self.runtime_ready
            && self.percpu_data_ready
            && self.records_preserved
            && self.setup_local_irq_save_restore_used
            && self.setup_local_irq_guard_bound_to_boot_cpu
    }

    #[allow(dead_code)]
    pub const fn runtime_ready(&self) -> bool {
        self.runtime_ready
    }

    #[allow(dead_code)]
    pub const fn percpu_data_ready(&self) -> bool {
        self.percpu_data_ready
    }

    #[allow(dead_code)]
    pub const fn records_preserved(&self) -> bool {
        self.records_preserved
    }

    #[allow(dead_code)]
    pub const fn setup_local_irq_save_restore_used(&self) -> bool {
        self.setup_local_irq_save_restore_used
    }

    pub fn setup_local_irq_guard_used_by(&self, boot_cpu_local_interrupt: &InterruptType) -> bool {
        self.lifecycle.state() == State::Ready
            && self.setup_local_irq_save_restore_used
            && self.setup_local_irq_guard_bound_to_boot_cpu
            && boot_cpu_local_interrupt.local_state() == State::Ready
            && boot_cpu_local_interrupt.disabled()
    }
}

#[allow(dead_code)]
pub fn preset() -> EventResult {
    PRINTK_BUFFER.lock().preset()
}

#[allow(dead_code)]
pub fn write_str(message: &str) {
    write_bytes(message.as_bytes());
}

pub fn write_bytes(bytes: &[u8]) {
    let _write_guard = PRINTK_WRITE_LOCK.lock();
    if capture_stress_mem(bytes) {
        return;
    }

    PRINTK_BUFFER.lock().write_bytes(bytes);
    match route() {
        PrintkRoute::BufferOnly => {}
        PrintkRoute::BootConsole => {}
        PrintkRoute::Serial8250 => {
            if ns16550a::write_console_bytes(bytes) {
                PRINTK_BUFFER.lock().discard_delivered();
                CONSOLE_REGISTRY
                    .lock()
                    .serial8250_delivered_records_not_replayed = true;
            }
        }
    }
}

#[cfg(checkpoint_handler_stress_mem)]
fn capture_stress_mem(bytes: &[u8]) -> bool {
    crate::stress_mem::capture_bytes(bytes);
    true
}

#[cfg(not(checkpoint_handler_stress_mem))]
fn capture_stress_mem(_bytes: &[u8]) -> bool {
    false
}

pub fn register_boot_console() {
    let _write_guard = PRINTK_WRITE_LOCK.lock();
    CONSOLE_REGISTRY.lock().register_boot_console();
}

pub fn register_serial8250_console(preferred_from_stdout: bool) -> bool {
    let (registered, emit_serial_online, emit_boot_offline) = {
        let _write_guard = PRINTK_WRITE_LOCK.lock();
        let mut registry = CONSOLE_REGISTRY.lock();
        let serial_online_before = registry.serial8250_online_trace_emitted;
        let boot_offline_before = registry.boot_console_offline_trace_emitted;
        let registered = registry.register_serial8250_console(preferred_from_stdout);
        (
            registered,
            registered && !serial_online_before && registry.serial8250_online_trace_emitted,
            registered && !boot_offline_before && registry.boot_console_offline_trace_emitted,
        )
    };
    if emit_serial_online {
        checkpoint::checkpoint(Checkpoint::Serial8250ConsoleOnline);
    }
    if emit_boot_offline {
        checkpoint::checkpoint(Checkpoint::BootConsoleOffline);
    }
    if registered && console_handoff_complete() && earlycon::disable_after_handoff().is_err() {
        panic!("earlycon disable failed during console handoff");
    }
    registered
}

#[allow(dead_code)]
#[cfg(checkpoint_handler_console_handoff)]
pub fn set_keep_bootcon(enabled: bool) {
    CONSOLE_REGISTRY.lock().set_keep_bootcon(enabled);
}

pub fn boot_console_registered() -> bool {
    CONSOLE_REGISTRY.lock().boot_console_registered
}

pub fn boot_console_online() -> bool {
    CONSOLE_REGISTRY.lock().boot_console_online
}

pub fn boot_console_unregistered() -> bool {
    CONSOLE_REGISTRY.lock().boot_console_unregistered
}

pub fn boot_console_removed_from_registry() -> bool {
    CONSOLE_REGISTRY.lock().boot_console_removed_from_registry
}

pub fn serial8250_console_registered() -> bool {
    CONSOLE_REGISTRY.lock().serial8250_console_registered
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_consdev() -> bool {
    CONSOLE_REGISTRY.lock().serial8250_consdev
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_write_ready() -> bool {
    CONSOLE_REGISTRY.lock().serial8250_write_ready
}

pub fn preferred_console_from_stdout() -> bool {
    CONSOLE_REGISTRY.lock().preferred_console_from_stdout
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn keep_bootcon() -> bool {
    CONSOLE_REGISTRY.lock().keep_bootcon
}

pub fn console_handoff_complete() -> bool {
    CONSOLE_REGISTRY.lock().handoff_complete
}

pub fn route() -> PrintkRoute {
    CONSOLE_REGISTRY.lock().route
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn boot_pending_flushed_before_serial_handoff() -> bool {
    {
        CONSOLE_REGISTRY
            .lock()
            .boot_pending_flushed_before_serial_handoff
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn legacy_earlycon_drain_blocked_after_handoff() -> bool {
    {
        CONSOLE_REGISTRY
            .lock()
            .legacy_earlycon_drain_blocked_after_handoff
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_online_trace_emitted() -> bool {
    CONSOLE_REGISTRY.lock().serial8250_online_trace_emitted
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn boot_console_offline_trace_emitted() -> bool {
    CONSOLE_REGISTRY.lock().boot_console_offline_trace_emitted
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_delivered_records_not_replayed() -> bool {
    {
        CONSOLE_REGISTRY
            .lock()
            .serial8250_delivered_records_not_replayed
    }
}

pub fn earlycon_drain_allowed() -> bool {
    let registry = CONSOLE_REGISTRY.lock();
    registry.route == PrintkRoute::BootConsole
        || (registry.keep_bootcon && registry.boot_console_online && !registry.handoff_complete)
}

#[allow(dead_code)]
pub fn setup(
    memblock: &MemBlock,
    per_cpu_storage: &PerCpuStorage,
    boot_param: &BootParam,
    boot_cpu_local_interrupt: &mut InterruptType,
) -> EventResult {
    PRINTK_BUFFER.lock().setup(
        memblock,
        per_cpu_storage,
        boot_param,
        boot_cpu_local_interrupt,
    )
}

#[allow(dead_code)]
pub fn write_byte(byte: u8) {
    write_bytes(&[byte]);
}

#[allow(dead_code)]
pub fn write_fmt(args: fmt::Arguments<'_>) {
    let _ = PrintkWriter.write_fmt(args);
}

pub fn is_prepared() -> bool {
    PRINTK_BUFFER.lock().is_prepared()
}

pub fn is_ready() -> bool {
    PRINTK_BUFFER.lock().is_ready()
}

pub fn setup_local_irq_save_restore_used() -> bool {
    PRINTK_BUFFER.lock().setup_local_irq_save_restore_used()
}

pub fn setup_local_irq_guard_used_by(boot_cpu_local_interrupt: &InterruptType) -> bool {
    {
        PRINTK_BUFFER
            .lock()
            .setup_local_irq_guard_used_by(boot_cpu_local_interrupt)
    }
}

#[allow(dead_code)]
pub fn drain_to(sink: impl FnMut(u8)) {
    let _write_guard = PRINTK_WRITE_LOCK.lock();
    drain_buffer_to(sink);
}

#[allow(dead_code)]
fn drain_buffer_to(sink: impl FnMut(u8)) {
    PRINTK_BUFFER.lock().drain_to(sink);
}

#[allow(dead_code)]
struct PrintkWriter;

impl Write for PrintkWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_str(s);
        Ok(())
    }
}
