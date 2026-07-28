use super::{
    boot_param::BootParam,
    earlycon,
    interrupt_type::InterruptType,
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
static mut PRINTK_BUFFER: PrintkBuffer = PrintkBuffer::new();
static mut CONSOLE_REGISTRY: ConsoleRegistry = ConsoleRegistry::new();

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
        checkpoint::checkpoint(Checkpoint::Serial8250ConsoleOnline);
        self.serial8250_online_trace_emitted = true;
        if !self.keep_bootcon {
            self.boot_console_online = false;
            self.boot_console_unregistered = true;
            self.boot_console_removed_from_registry = true;
            checkpoint::checkpoint(Checkpoint::BootConsoleOffline);
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
    unsafe { (&raw mut PRINTK_BUFFER).as_mut().unwrap().preset() }
}

#[allow(dead_code)]
pub fn write_str(message: &str) {
    write_bytes(message.as_bytes());
}

pub fn write_bytes(bytes: &[u8]) {
    if capture_stress_mem(bytes) {
        return;
    }

    unsafe {
        (&raw mut PRINTK_BUFFER)
            .as_mut()
            .unwrap()
            .write_bytes(bytes);
    }
    match route() {
        PrintkRoute::BufferOnly => {}
        PrintkRoute::BootConsole => {}
        PrintkRoute::Serial8250 => {
            if ns16550a::write_console_bytes(bytes) {
                unsafe {
                    (&raw mut PRINTK_BUFFER)
                        .as_mut()
                        .unwrap()
                        .discard_delivered();
                    (&raw mut CONSOLE_REGISTRY)
                        .as_mut()
                        .unwrap()
                        .serial8250_delivered_records_not_replayed = true;
                }
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
    unsafe {
        (&raw mut CONSOLE_REGISTRY)
            .as_mut()
            .unwrap()
            .register_boot_console();
    }
}

pub fn register_serial8250_console(preferred_from_stdout: bool) -> bool {
    let registered = unsafe {
        (&raw mut CONSOLE_REGISTRY)
            .as_mut()
            .unwrap()
            .register_serial8250_console(preferred_from_stdout)
    };
    if registered && console_handoff_complete() && earlycon::disable_after_handoff().is_err() {
        panic!("earlycon disable failed during console handoff");
    }
    registered
}

#[allow(dead_code)]
#[cfg(checkpoint_handler_console_handoff)]
pub fn set_keep_bootcon(enabled: bool) {
    unsafe {
        (&raw mut CONSOLE_REGISTRY)
            .as_mut()
            .unwrap()
            .set_keep_bootcon(enabled);
    }
}

pub fn boot_console_registered() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .boot_console_registered
    }
}

pub fn boot_console_online() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .boot_console_online
    }
}

pub fn boot_console_unregistered() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .boot_console_unregistered
    }
}

pub fn boot_console_removed_from_registry() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .boot_console_removed_from_registry
    }
}

pub fn serial8250_console_registered() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .serial8250_console_registered
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_consdev() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .serial8250_consdev
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_write_ready() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .serial8250_write_ready
    }
}

pub fn preferred_console_from_stdout() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .preferred_console_from_stdout
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn keep_bootcon() -> bool {
    unsafe { (&raw const CONSOLE_REGISTRY).as_ref().unwrap().keep_bootcon }
}

pub fn console_handoff_complete() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .handoff_complete
    }
}

pub fn route() -> PrintkRoute {
    unsafe { (&raw const CONSOLE_REGISTRY).as_ref().unwrap().route }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn boot_pending_flushed_before_serial_handoff() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .boot_pending_flushed_before_serial_handoff
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn legacy_earlycon_drain_blocked_after_handoff() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .legacy_earlycon_drain_blocked_after_handoff
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_online_trace_emitted() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .serial8250_online_trace_emitted
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn boot_console_offline_trace_emitted() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .boot_console_offline_trace_emitted
    }
}

#[cfg(checkpoint_handler_console_handoff)]
pub fn serial8250_delivered_records_not_replayed() -> bool {
    unsafe {
        (&raw const CONSOLE_REGISTRY)
            .as_ref()
            .unwrap()
            .serial8250_delivered_records_not_replayed
    }
}

pub fn earlycon_drain_allowed() -> bool {
    let registry = unsafe { (&raw const CONSOLE_REGISTRY).as_ref().unwrap() };
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
    unsafe {
        (&raw mut PRINTK_BUFFER).as_mut().unwrap().setup(
            memblock,
            per_cpu_storage,
            boot_param,
            boot_cpu_local_interrupt,
        )
    }
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
    unsafe { (&raw const PRINTK_BUFFER).as_ref().unwrap().is_prepared() }
}

pub fn is_ready() -> bool {
    unsafe { (&raw const PRINTK_BUFFER).as_ref().unwrap().is_ready() }
}

pub fn setup_local_irq_save_restore_used() -> bool {
    unsafe {
        (&raw const PRINTK_BUFFER)
            .as_ref()
            .unwrap()
            .setup_local_irq_save_restore_used()
    }
}

pub fn setup_local_irq_guard_used_by(boot_cpu_local_interrupt: &InterruptType) -> bool {
    unsafe {
        (&raw const PRINTK_BUFFER)
            .as_ref()
            .unwrap()
            .setup_local_irq_guard_used_by(boot_cpu_local_interrupt)
    }
}

#[allow(dead_code)]
pub fn drain_to(sink: impl FnMut(u8)) {
    drain_buffer_to(sink);
}

#[allow(dead_code)]
fn drain_buffer_to(sink: impl FnMut(u8)) {
    unsafe {
        (&raw mut PRINTK_BUFFER).as_mut().unwrap().drain_to(sink);
    }
}

#[allow(dead_code)]
struct PrintkWriter;

impl Write for PrintkWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_str(s);
        Ok(())
    }
}
