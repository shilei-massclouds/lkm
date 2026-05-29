use core::sync::atomic::{AtomicU8, Ordering};

use crate::{arch::riscv64::csr, trace::Checkpoint};

use super::state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State};

const SCAUSE_INTERRUPT_BIT: usize = 1usize << (usize::BITS as usize - 1);
const INTERRUPT_HANDLER_COUNT: usize = 16;

const HANDLER_FALLBACK: u8 = 0;

static INTERRUPT_HANDLER_POLICY: [AtomicU8; INTERRUPT_HANDLER_COUNT] =
    [const { AtomicU8::new(HANDLER_FALLBACK) }; INTERRUPT_HANDLER_COUNT];

#[derive(Clone, Copy)]
struct InterruptPolicy(u8);

const FALLBACK_POLICY: InterruptPolicy = InterruptPolicy(HANDLER_FALLBACK);

pub struct InterruptStream {
    lifecycle: Lifecycle,
}

impl InterruptStream {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    pub fn adopt_head_preset(&mut self) -> EventResult {
        if csr::read_sie() != 0 || csr::read_sip() != 0 {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        reset_interrupt_handlers();
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self) -> EventResult {
        if self.lifecycle.state() != State::Prepared {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::InterruptStreamReady,
        )
    }
}

pub fn dispatch_scause(scause: usize) -> ! {
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

fn dispatch_handler_policy(handler: u8, scause: usize) -> ! {
    match handler {
        _ => default_interrupt_handler(scause),
    }
}

fn default_interrupt_handler(_scause: usize) -> ! {
    crate::arch::riscv64::sbi::putstr("interrupt fallback panic\n");
    crate::arch::riscv64::sbi::system_shutdown()
}
