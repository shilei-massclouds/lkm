use core::sync::atomic::{AtomicU8, Ordering};

use crate::trace::{self, Checkpoint};

use super::{
    event_stream::{EventStream, TrapFrame},
    init_stack::InitStack,
    state::{failed_condition, EventResult, Lifecycle, LifecycleEvent, State},
};

const SCAUSE_INTERRUPT_BIT: usize = 1usize << (usize::BITS as usize - 1);
const EXC_INSTRUCTION_PAGE_FAULT: usize = 12;
const EXC_LOAD_PAGE_FAULT: usize = 13;
const EXC_STORE_PAGE_FAULT: usize = 15;
const EXC_BREAKPOINT: usize = 3;
const EXC_USER_ECALL: usize = 8;
const EXC_SUPERVISOR_ECALL: usize = 9;
const EXCEPTION_HANDLER_COUNT: usize = 16;

const HANDLER_FALLBACK: u8 = 0;
const HANDLER_PAGE_FAULT: u8 = 1;
const HANDLER_SYSCALL_DISABLED: u8 = 2;
const HANDLER_BREAKPOINT: u8 = 3;
const HANDLER_UNEXPECTED: u8 = 4;

const PAGE_FAULT_CAUSES: [usize; 3] = [
    EXC_INSTRUCTION_PAGE_FAULT,
    EXC_LOAD_PAGE_FAULT,
    EXC_STORE_PAGE_FAULT,
];
const SYSCALL_CAUSES: [usize; 2] = [EXC_USER_ECALL, EXC_SUPERVISOR_ECALL];
const BREAKPOINT_CAUSES: [usize; 1] = [EXC_BREAKPOINT];

static DISPATCH_READY: AtomicU8 = AtomicU8::new(0);
static EXCEPTION_HANDLER_POLICY: [AtomicU8; EXCEPTION_HANDLER_COUNT] =
    [const { AtomicU8::new(HANDLER_FALLBACK) }; EXCEPTION_HANDLER_COUNT];

#[derive(Clone, Copy)]
struct ExceptionPolicy(u8);

const FALLBACK_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_FALLBACK);
const PAGE_FAULT_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_PAGE_FAULT);
const SYSCALL_DISABLED_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_SYSCALL_DISABLED);
const BREAKPOINT_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_BREAKPOINT);
const UNEXPECTED_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_UNEXPECTED);

pub struct ExceptionStream {
    lifecycle: Lifecycle,
    page_fault: ExceptionKind,
    syscall: ExceptionKind,
    breakpoint: ExceptionKind,
    unexpected: ExceptionKind,
    dispatch_ready: bool,
}

impl ExceptionStream {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            page_fault: ExceptionKind::new(),
            syscall: ExceptionKind::new(),
            breakpoint: ExceptionKind::new(),
            unexpected: ExceptionKind::new(),
            dispatch_ready: false,
        }
    }

    pub fn preset(&mut self, event_stream: &EventStream, init_stack: &InitStack) -> EventResult {
        if self.lifecycle.state() != State::Base
            || event_stream.state() != State::Prepared
            || init_stack.state() != State::Prepared
        {
            return failed_condition(
                LifecycleEvent::Preset,
                self.lifecycle.state(),
                State::Base,
                State::Prepared,
            );
        }

        self.page_fault.preset()?;
        self.syscall.preset()?;
        self.breakpoint.preset()?;
        self.unexpected.preset()?;
        reset_exception_handlers();
        bind_exception_policy(
            ExceptionHandlerBinding::Causes(&SYSCALL_CAUSES),
            SYSCALL_DISABLED_POLICY,
        );

        self.lifecycle.transition(
            LifecycleEvent::Preset,
            State::Base,
            State::Prepared,
            Checkpoint::ExceptionStreamPrepared,
        )
    }

    pub fn state(&self) -> State {
        self.lifecycle.state()
    }

    pub fn setup(&mut self, event_stream: &EventStream) -> EventResult {
        if self.lifecycle.state() != State::Prepared || event_stream.state() != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        self.page_fault_setup()?;
        self.breakpoint_setup()?;
        self.unexpected_setup()?;
        self.dispatch_ready = true;
        DISPATCH_READY.store(1, Ordering::Relaxed);
        self.lifecycle.transition(
            LifecycleEvent::Setup,
            State::Prepared,
            State::Ready,
            Checkpoint::ExceptionStreamReady,
        )
    }

    pub fn page_fault_setup(&mut self) -> EventResult {
        self.page_fault.setup(
            self.lifecycle.state(),
            ExceptionHandlerBinding::Causes(&PAGE_FAULT_CAUSES),
            PAGE_FAULT_POLICY,
        )
    }

    pub fn breakpoint_setup(&mut self) -> EventResult {
        self.breakpoint.setup(
            self.lifecycle.state(),
            ExceptionHandlerBinding::Causes(&BREAKPOINT_CAUSES),
            BREAKPOINT_POLICY,
        )
    }

    pub fn unexpected_setup(&mut self) -> EventResult {
        self.unexpected.setup(
            self.lifecycle.state(),
            ExceptionHandlerBinding::RemainingKnown,
            UNEXPECTED_POLICY,
        )
    }

    pub fn page_fault_state(&self) -> State {
        self.page_fault.state()
    }

    pub fn syscall_state(&self) -> State {
        self.syscall.state()
    }

    pub fn breakpoint_state(&self) -> State {
        self.breakpoint.state()
    }

    pub fn unexpected_state(&self) -> State {
        self.unexpected.state()
    }

    #[allow(dead_code)]
    pub const fn dispatch_ready(&self) -> bool {
        self.dispatch_ready
    }
}

struct ExceptionKind {
    lifecycle: Lifecycle,
}

enum ExceptionHandlerBinding {
    Causes(&'static [usize]),
    RemainingKnown,
}

impl ExceptionKind {
    const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
        }
    }

    fn preset(&mut self) -> EventResult {
        self.lifecycle
            .adopt_transition(LifecycleEvent::Preset, State::Base, State::Prepared)
    }

    fn setup(
        &mut self,
        exception_stream_state: State,
        binding: ExceptionHandlerBinding,
        policy: ExceptionPolicy,
    ) -> EventResult {
        if exception_stream_state != State::Prepared || self.lifecycle.state() != State::Prepared {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        bind_exception_policy(binding, policy);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    fn state(&self) -> State {
        self.lifecycle.state()
    }
}

pub fn dispatch_trap(frame: &mut TrapFrame) {
    if frame.scause & SCAUSE_INTERRUPT_BIT != 0 {
        panic_dispatch("interrupt reached exception stream\n");
    }

    if DISPATCH_READY.load(Ordering::Relaxed) == 0 {
        panic_dispatch("exception dispatch not ready\n");
    }

    dispatch_handler_frame(read_exception_handler_policy(frame.scause), frame);
}

fn reset_exception_handlers() {
    for cause in 0..EXCEPTION_HANDLER_COUNT {
        set_exception_policy(cause, FALLBACK_POLICY);
    }
}

fn bind_exception_policy(binding: ExceptionHandlerBinding, policy: ExceptionPolicy) {
    match binding {
        ExceptionHandlerBinding::Causes(causes) => {
            for &cause in causes {
                set_exception_policy(cause, policy);
            }
        }
        ExceptionHandlerBinding::RemainingKnown => {
            for cause in 0..EXCEPTION_HANDLER_COUNT {
                if !known_mechanism_cause(cause) {
                    set_exception_policy(cause, policy);
                }
            }
        }
    }
}

fn read_exception_handler_policy(scause: usize) -> u8 {
    let cause = scause & !SCAUSE_INTERRUPT_BIT;
    if cause >= EXCEPTION_HANDLER_COUNT {
        return HANDLER_UNEXPECTED;
    }

    EXCEPTION_HANDLER_POLICY[cause].load(Ordering::Relaxed)
}

fn known_mechanism_cause(cause: usize) -> bool {
    PAGE_FAULT_CAUSES.contains(&cause)
        || SYSCALL_CAUSES.contains(&cause)
        || BREAKPOINT_CAUSES.contains(&cause)
}

fn set_exception_policy(cause: usize, policy: ExceptionPolicy) {
    if cause >= EXCEPTION_HANDLER_COUNT {
        return;
    }

    EXCEPTION_HANDLER_POLICY[cause].store(policy.0, Ordering::Relaxed);
}

fn dispatch_handler_frame(handler: u8, frame: &mut TrapFrame) {
    match handler {
        HANDLER_BREAKPOINT => breakpoint_exception_handler(frame),
        HANDLER_PAGE_FAULT => page_fault_exception_handler(frame.scause),
        HANDLER_SYSCALL_DISABLED => syscall_disabled_exception_handler(frame.scause),
        HANDLER_UNEXPECTED => unexpected_exception_handler(frame.scause),
        _ => default_exception_handler(frame.scause),
    }
}

fn default_exception_handler(_scause: usize) -> ! {
    panic_dispatch("exception fallback panic\n")
}

fn page_fault_exception_handler(_scause: usize) -> ! {
    panic_dispatch("page fault exception\n")
}

fn syscall_disabled_exception_handler(_scause: usize) -> ! {
    panic_dispatch("syscall exception not enabled\n")
}

fn breakpoint_exception_handler(frame: &mut TrapFrame) {
    trace::checkpoint(Checkpoint::BreakpointExceptionHandled);
    frame.sepc = frame
        .sepc
        .wrapping_add(breakpoint_instruction_length(frame.sepc));
}

fn unexpected_exception_handler(_scause: usize) -> ! {
    panic_dispatch("unexpected exception\n")
}

fn breakpoint_instruction_length(sepc: usize) -> usize {
    let insn = unsafe { core::ptr::read_unaligned(sepc as *const u16) };
    if insn & 0b11 == 0b11 {
        4
    } else {
        2
    }
}

fn panic_dispatch(message: &str) -> ! {
    crate::arch::riscv64::sbi::putstr(message);
    crate::arch::riscv64::sbi::system_shutdown()
}
