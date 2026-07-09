use core::sync::atomic::{AtomicU8, Ordering};

use crate::{
    arch::riscv64::{csr, SUPERVISOR_EXTERNAL_IRQ, SUPERVISOR_TIMER_IRQ},
    checkpoint::Checkpoint,
};

use super::{
    cpu_control::LocalInterruptControl,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};

const SCAUSE_INTERRUPT_BIT: usize = 1usize << (usize::BITS as usize - 1);
const INTERRUPT_HANDLER_COUNT: usize = 16;

const HANDLER_FALLBACK: u8 = 0;
const HANDLER_TIMER: u8 = 1;
const HANDLER_EXTERNAL: u8 = 2;

static INTERRUPT_HANDLER_POLICY: [AtomicU8; INTERRUPT_HANDLER_COUNT] =
    [const { AtomicU8::new(HANDLER_FALLBACK) }; INTERRUPT_HANDLER_COUNT];

#[derive(Clone, Copy)]
struct InterruptPolicy(u8);

const FALLBACK_POLICY: InterruptPolicy = InterruptPolicy(HANDLER_FALLBACK);

pub struct InterruptStream {
    lifecycle: Lifecycle,
    timer_handler_ready: bool,
    external_handler_ready: bool,
    supervisor_external_input_gate_defined: bool,
    supervisor_external_input_gate_closed: bool,
    supervisor_external_input_enable_deferred: bool,
    supervisor_external_input_gate_open: bool,
    boot_cpu_local_interrupts_enabled: bool,
    early_boot_irqs_disabled: bool,
}

impl InterruptStream {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            timer_handler_ready: false,
            external_handler_ready: false,
            supervisor_external_input_gate_defined: false,
            supervisor_external_input_gate_closed: false,
            supervisor_external_input_enable_deferred: false,
            supervisor_external_input_gate_open: false,
            boot_cpu_local_interrupts_enabled: false,
            early_boot_irqs_disabled: false,
        }
    }

    pub fn adopt_head_preset(&mut self) -> EventResult {
        if csr::read_sie() != 0 {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        csr::clear_supervisor_interrupt_pending();
        reset_interrupt_handlers();
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub const fn timer_handler_ready(&self) -> bool {
        self.timer_handler_ready
    }

    pub const fn external_handler_ready(&self) -> bool {
        self.external_handler_ready
    }

    pub const fn supervisor_external_input_gate_defined(&self) -> bool {
        self.supervisor_external_input_gate_defined
    }

    pub const fn supervisor_external_input_gate_closed(&self) -> bool {
        self.supervisor_external_input_gate_closed
    }

    pub const fn supervisor_external_input_enable_deferred(&self) -> bool {
        self.supervisor_external_input_enable_deferred
    }

    pub const fn supervisor_external_input_gate_open(&self) -> bool {
        self.supervisor_external_input_gate_open
    }

    pub const fn boot_cpu_local_interrupts_enabled(&self) -> bool {
        self.boot_cpu_local_interrupts_enabled
    }

    pub const fn early_boot_irqs_disabled(&self) -> bool {
        self.early_boot_irqs_disabled
    }

    pub fn setup(&mut self, local_interrupt: &mut LocalInterruptControl) -> EventResult {
        if self.lifecycle.state() != State::Prepared || local_interrupt.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        local_interrupt.disable()?;
        self.boot_cpu_local_interrupts_enabled = false;
        self.early_boot_irqs_disabled = true;
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::InterruptStreamReady,
        )
    }

    pub fn bind_timer_handler(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        bind_interrupt_policy(SUPERVISOR_TIMER_IRQ, InterruptPolicy(HANDLER_TIMER));
        self.timer_handler_ready = true;
        Ok(())
    }

    pub fn bind_external_handler(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Ready,
                State::Ready,
            );
        }

        bind_interrupt_policy(SUPERVISOR_EXTERNAL_IRQ, InterruptPolicy(HANDLER_EXTERNAL));
        self.external_handler_ready = true;
        self.supervisor_external_input_gate_defined = true;
        self.supervisor_external_input_gate_closed = true;
        self.supervisor_external_input_enable_deferred = true;
        Ok(())
    }

    pub fn enable_supervisor_external_input(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Online
            || !self.external_handler_ready
            || !self.supervisor_external_input_gate_defined
            || !self.supervisor_external_input_gate_closed
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        csr::enable_supervisor_external_interrupt();
        if !csr::supervisor_external_interrupt_enabled() {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Online,
                State::Online,
            );
        }

        self.supervisor_external_input_gate_closed = false;
        self.supervisor_external_input_enable_deferred = false;
        self.supervisor_external_input_gate_open = true;
        Ok(())
    }

    pub fn enable(&mut self, local_interrupt: &mut LocalInterruptControl) -> EventResult {
        if self.lifecycle.state() != State::Ready
            || !self.timer_handler_ready
            || !self.external_handler_ready
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.early_boot_irqs_disabled = false;
        if let Err(err) = local_interrupt.enable() {
            self.early_boot_irqs_disabled = true;
            return Err(err);
        }
        if !local_interrupt.enabled() || !csr::supervisor_interrupts_enabled() {
            self.early_boot_irqs_disabled = true;
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.boot_cpu_local_interrupts_enabled = local_interrupt.enabled();
        self.lifecycle.transition(
            LifecycleEvent::Enable,
            State::Ready,
            State::Online,
            Checkpoint::InterruptStreamOnline,
        )
    }
}

pub fn dispatch_scause(scause: usize) {
    dispatch_handler_policy(read_interrupt_handler_policy(scause), scause)
}

fn reset_interrupt_handlers() {
    for cause in 0..INTERRUPT_HANDLER_COUNT {
        bind_interrupt_policy(cause, FALLBACK_POLICY);
    }
}

fn read_interrupt_handler_policy(scause: usize) -> u8 {
    let cause = scause & !SCAUSE_INTERRUPT_BIT;
    if cause >= INTERRUPT_HANDLER_COUNT {
        return HANDLER_FALLBACK;
    }

    INTERRUPT_HANDLER_POLICY[cause].load(Ordering::Relaxed)
}

fn bind_interrupt_policy(cause: usize, policy: InterruptPolicy) {
    if cause >= INTERRUPT_HANDLER_COUNT {
        return;
    }

    INTERRUPT_HANDLER_POLICY[cause].store(policy.0, Ordering::Relaxed);
}

fn dispatch_handler_policy(handler: u8, scause: usize) {
    match handler {
        HANDLER_TIMER => timer_interrupt_handler(),
        HANDLER_EXTERNAL => external_interrupt_handler(),
        _ => default_interrupt_handler(scause),
    }
}

fn timer_interrupt_handler() {
    crate::objects::irq_time::handle_timer_interrupt();
}

fn external_interrupt_handler() {
    crate::objects::irq_time::handle_external_interrupt();
}

fn default_interrupt_handler(scause: usize) -> ! {
    let cause = scause & !SCAUSE_INTERRUPT_BIT;
    crate::arch::riscv64::sbi::putstr("interrupt fallback panic scause=0x");
    print_hex(scause);
    crate::arch::riscv64::sbi::putstr(" cause=0x");
    print_hex(cause);
    crate::arch::riscv64::sbi::putstr(" policy=0x");
    print_hex(read_interrupt_handler_policy(scause) as usize);
    crate::arch::riscv64::sbi::putstr(" gp=0x");
    print_hex(crate::arch::riscv64::csr::read_gp());
    crate::arch::riscv64::sbi::putstr(" tp=0x");
    print_hex(crate::arch::riscv64::csr::read_tp());
    crate::arch::riscv64::sbi::putchar(b'\n');
    crate::arch::riscv64::sbi::system_shutdown()
}

fn print_hex(value: usize) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut shift = usize::BITS as usize;
    while shift != 0 {
        shift -= 4;
        crate::arch::riscv64::sbi::putchar(HEX[(value >> shift) & 0xf]);
    }
}
