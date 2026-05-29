use core::sync::atomic::{AtomicU8, Ordering};

use crate::trace::Checkpoint;

use super::{
    event_stream::EventStream,
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

const HANDLER_NONE: u8 = 0;
const HANDLER_PAGE_FAULT: u8 = 1;
const HANDLER_SYSCALL: u8 = 2;
const HANDLER_BREAKPOINT: u8 = 3;
const HANDLER_UNEXPECTED: u8 = 4;

static DISPATCH_READY: AtomicU8 = AtomicU8::new(0);
static PAGE_FAULT_HANDLER: AtomicU8 = AtomicU8::new(HANDLER_NONE);
static SYSCALL_HANDLER: AtomicU8 = AtomicU8::new(HANDLER_NONE);
static BREAKPOINT_HANDLER: AtomicU8 = AtomicU8::new(HANDLER_NONE);
static UNEXPECTED_HANDLER: AtomicU8 = AtomicU8::new(HANDLER_NONE);

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
            &PAGE_FAULT_HANDLER,
            HANDLER_PAGE_FAULT,
        )
    }

    pub fn breakpoint_setup(&mut self) -> EventResult {
        self.breakpoint.setup(
            self.lifecycle.state(),
            &BREAKPOINT_HANDLER,
            HANDLER_BREAKPOINT,
        )
    }

    pub fn unexpected_setup(&mut self) -> EventResult {
        self.unexpected.setup(
            self.lifecycle.state(),
            &UNEXPECTED_HANDLER,
            HANDLER_UNEXPECTED,
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

    #[allow(dead_code)]
    pub fn dispatch_kind_for_scause(&self, scause: usize) -> Option<ExceptionDispatchKind> {
        if !self.dispatch_ready {
            return None;
        }
        ExceptionDispatchKind::from_scause(scause)
    }
}

struct ExceptionKind {
    lifecycle: Lifecycle,
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
        handler_slot: &AtomicU8,
        handler: u8,
    ) -> EventResult {
        if exception_stream_state != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Prepared,
                State::Ready,
            );
        }

        handler_slot.store(handler, Ordering::Relaxed);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Prepared, State::Ready)
    }

    fn state(&self) -> State {
        self.lifecycle.state()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ExceptionDispatchKind {
    PageFault,
    Syscall,
    Breakpoint,
    Unexpected,
}

impl ExceptionDispatchKind {
    pub const fn from_scause(scause: usize) -> Option<Self> {
        if scause & SCAUSE_INTERRUPT_BIT != 0 {
            return None;
        }

        match scause {
            EXC_INSTRUCTION_PAGE_FAULT | EXC_LOAD_PAGE_FAULT | EXC_STORE_PAGE_FAULT => {
                Some(Self::PageFault)
            }
            EXC_BREAKPOINT => Some(Self::Breakpoint),
            EXC_USER_ECALL | EXC_SUPERVISOR_ECALL => Some(Self::Syscall),
            _ => Some(Self::Unexpected),
        }
    }
}

pub fn dispatch_scause(scause: usize) -> ! {
    let Some(kind) = ExceptionDispatchKind::from_scause(scause) else {
        panic_dispatch("interrupt reached exception stream\n");
    };

    if DISPATCH_READY.load(Ordering::Relaxed) == 0 {
        panic_dispatch("exception dispatch not ready\n");
    }

    match kind {
        ExceptionDispatchKind::PageFault => dispatch_registered(
            PAGE_FAULT_HANDLER.load(Ordering::Relaxed),
            HANDLER_PAGE_FAULT,
            "page fault exception\n",
        ),
        ExceptionDispatchKind::Breakpoint => dispatch_registered(
            BREAKPOINT_HANDLER.load(Ordering::Relaxed),
            HANDLER_BREAKPOINT,
            "breakpoint exception\n",
        ),
        ExceptionDispatchKind::Unexpected => dispatch_registered(
            UNEXPECTED_HANDLER.load(Ordering::Relaxed),
            HANDLER_UNEXPECTED,
            "unexpected exception\n",
        ),
        ExceptionDispatchKind::Syscall => dispatch_registered(
            SYSCALL_HANDLER.load(Ordering::Relaxed),
            HANDLER_SYSCALL,
            "syscall exception not enabled\n",
        ),
    }
}

fn dispatch_registered(actual: u8, expected: u8, message: &str) -> ! {
    if actual != expected {
        panic_dispatch("exception handler not registered\n");
    }

    panic_dispatch(message)
}

fn panic_dispatch(message: &str) -> ! {
    crate::arch::riscv64::sbi::putstr(message);
    crate::arch::riscv64::sbi::system_shutdown()
}
