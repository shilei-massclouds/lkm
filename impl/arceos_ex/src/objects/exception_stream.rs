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
const HANDLER_SYSCALL: u8 = 5;

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
#[cfg(app_user_boot)]
const SYSCALL_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_SYSCALL);
#[cfg(not(app_user_boot))]
const SYSCALL_POLICY: ExceptionPolicy = ExceptionPolicy(HANDLER_SYSCALL_DISABLED);
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_EXIT_GROUP: usize = 94;
const USER_COPY_MAX: usize = 256;

static SYSCALL_TABLE_READY: AtomicU8 = AtomicU8::new(0);

pub struct SyscallTable {
    lifecycle: Lifecycle,
    #[allow(dead_code)]
    bound_to_exception: bool,
    write_supported: bool,
    exit_supported: bool,
    exit_group_supported: bool,
    write_usercopy_ready: bool,
    write_routes_to_console: bool,
    exit_records_status: bool,
    write_observed: AtomicU8,
    exit_observed: AtomicU8,
}

impl SyscallTable {
    pub const fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(State::Base),
            bound_to_exception: false,
            write_supported: false,
            exit_supported: false,
            exit_group_supported: false,
            write_usercopy_ready: false,
            write_routes_to_console: false,
            exit_records_status: false,
            write_observed: AtomicU8::new(0),
            exit_observed: AtomicU8::new(0),
        }
    }

    #[allow(dead_code)]
    pub const fn state(&self) -> State {
        self.lifecycle.state()
    }

    #[allow(dead_code)]
    pub const fn bound_to_exception(&self) -> bool {
        self.bound_to_exception
    }

    #[allow(dead_code)]
    pub const fn write_supported(&self) -> bool {
        self.write_supported
    }

    #[allow(dead_code)]
    pub const fn exit_supported(&self) -> bool {
        self.exit_supported
    }

    #[allow(dead_code)]
    pub const fn exit_group_supported(&self) -> bool {
        self.exit_group_supported
    }

    #[allow(dead_code)]
    pub const fn write_usercopy_ready(&self) -> bool {
        self.write_usercopy_ready
    }

    #[allow(dead_code)]
    pub const fn write_routes_to_console(&self) -> bool {
        self.write_routes_to_console
    }

    #[allow(dead_code)]
    pub const fn exit_records_status(&self) -> bool {
        self.exit_records_status
    }

    #[allow(dead_code)]
    pub fn write_observed(&self) -> bool {
        self.write_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn exit_observed(&self) -> bool {
        self.exit_observed.load(Ordering::Acquire) != 0
    }

    #[allow(dead_code)]
    pub fn setup(&mut self, syscall_exception_state: State) -> EventResult {
        if self.lifecycle.state() != State::Base || syscall_exception_state != State::Ready {
            return failed_condition(
                LifecycleEvent::Setup,
                self.lifecycle.state(),
                State::Base,
                State::Ready,
            );
        }

        self.bound_to_exception = true;
        self.write_supported = true;
        self.exit_supported = true;
        self.exit_group_supported = true;
        self.write_usercopy_ready = true;
        self.write_routes_to_console = true;
        self.exit_records_status = true;
        SYSCALL_TABLE_READY.store(1, Ordering::Relaxed);
        self.lifecycle
            .adopt_transition(LifecycleEvent::Setup, State::Base, State::Ready)
    }

    pub fn write(&self, frame: &mut TrapFrame) {
        if self.lifecycle.state() != State::Ready
            || !self.write_supported
            || !self.write_usercopy_ready
            || !self.write_routes_to_console
        {
            complete_unsupported_syscall(frame);
            return;
        }

        syscall_table_write(self, frame);
    }

    pub fn exit(&self, frame: &mut TrapFrame) -> ! {
        if self.lifecycle.state() != State::Ready
            || !self.exit_supported
            || !self.exit_records_status
        {
            panic_dispatch("syscall exit table entry not ready\n");
        }

        syscall_table_exit(self, frame)
    }

    pub fn exit_group(&self, frame: &mut TrapFrame) -> ! {
        if self.lifecycle.state() != State::Ready
            || !self.exit_group_supported
            || !self.exit_records_status
        {
            panic_dispatch("syscall exit_group table entry not ready\n");
        }

        syscall_table_exit(self, frame)
    }
}

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

    pub fn syscall_setup(&mut self, table: &mut SyscallTable) -> EventResult {
        self.syscall.setup(
            self.lifecycle.state(),
            ExceptionHandlerBinding::Causes(&SYSCALL_CAUSES),
            SYSCALL_POLICY,
        )?;
        table.setup(self.syscall.state())
    }

    pub fn syscall_enable(&mut self, table: &SyscallTable) -> EventResult {
        self.syscall.enable(self.lifecycle.state(), table)
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
        if !matches!(exception_stream_state, State::Prepared | State::Ready)
            || self.lifecycle.state() != State::Prepared
        {
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

    fn enable(
        &mut self,
        exception_stream_state: State,
        syscall_table: &SyscallTable,
    ) -> EventResult {
        if exception_stream_state != State::Ready
            || self.lifecycle.state() != State::Ready
            || syscall_table.state() != State::Ready
            || !syscall_table.bound_to_exception()
        {
            return failed_condition(
                LifecycleEvent::Enable,
                self.lifecycle.state(),
                State::Ready,
                State::Online,
            );
        }

        self.lifecycle
            .adopt_transition(LifecycleEvent::Enable, State::Ready, State::Online)
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
        HANDLER_PAGE_FAULT => page_fault_exception_handler(frame),
        HANDLER_SYSCALL_DISABLED => syscall_disabled_exception_handler(frame),
        HANDLER_SYSCALL => syscall_exception_handler(frame),
        HANDLER_UNEXPECTED => unexpected_exception_handler(frame),
        _ => default_exception_handler(frame),
    }
}

fn syscall_table_ref() -> Option<&'static SyscallTable> {
    if SYSCALL_TABLE_READY.load(Ordering::Relaxed) == 0 {
        return None;
    }

    Some(&crate::context::context_ref().syscall_table)
}

fn default_exception_handler(frame: &TrapFrame) -> ! {
    panic_dispatch_frame("exception fallback panic", frame)
}

fn page_fault_exception_handler(frame: &TrapFrame) -> ! {
    panic_dispatch_frame("page fault exception", frame)
}

fn syscall_disabled_exception_handler(frame: &TrapFrame) -> ! {
    panic_dispatch_frame("syscall exception not enabled", frame)
}

fn syscall_exception_handler(frame: &mut TrapFrame) {
    let Some(table) = syscall_table_ref() else {
        panic_dispatch("syscall table not ready\n");
    };

    match frame.reg(17) {
        SYSCALL_WRITE => table.write(frame),
        SYSCALL_EXIT => table.exit(frame),
        SYSCALL_EXIT_GROUP => table.exit_group(frame),
        _ => complete_unsupported_syscall(frame),
    }
}

fn complete_unsupported_syscall(frame: &mut TrapFrame) {
    frame.set_reg(10, usize::MAX);
    frame.sepc = frame.sepc.wrapping_add(4);
}

fn syscall_table_write(table: &SyscallTable, frame: &mut TrapFrame) {
    let fd = frame.reg(10);
    let user_ptr = frame.reg(11);
    let len = frame.reg(12);
    if !(fd == 1 || fd == 2) || len > USER_COPY_MAX {
        complete_unsupported_syscall(frame);
        return;
    }

    let mut buffer = [0u8; USER_COPY_MAX];
    if !copy_from_user(user_ptr, &mut buffer[..len]) {
        complete_unsupported_syscall(frame);
        return;
    }

    table.write_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(Checkpoint::SyscallTableWrite, crate::context::context_ref());
    crate::objects::printk::write_bytes(&buffer[..len]);
    frame.set_reg(10, len);
    frame.sepc = frame.sepc.wrapping_add(4);
}

fn syscall_table_exit(table: &SyscallTable, frame: &mut TrapFrame) -> ! {
    table.exit_observed.store(1, Ordering::Release);
    crate::checkpoint::dispatch(Checkpoint::SyscallTableExit, crate::context::context_ref());
    let status = frame.reg(10);
    crate::arch::riscv64::sbi::putstr("user exit status=");
    print_decimal(status);
    crate::arch::riscv64::sbi::putchar(b'\n');
    crate::arch::riscv64::sbi::system_shutdown()
}

fn copy_from_user(user_ptr: usize, dst: &mut [u8]) -> bool {
    if dst.is_empty() {
        return true;
    }
    if user_ptr == 0 || user_ptr.checked_add(dst.len()).is_none() {
        return false;
    }

    let saved = crate::arch::riscv64::csr::save_and_enable_user_memory_access();
    let mut index = 0usize;
    while index < dst.len() {
        dst[index] = unsafe { core::ptr::read_volatile((user_ptr + index) as *const u8) };
        index += 1;
    }
    crate::arch::riscv64::csr::restore_user_memory_access(saved);
    true
}

fn breakpoint_exception_handler(frame: &mut TrapFrame) {
    if run_breakpoint_hooks(frame) == BreakpointHookResult::Resume {
        return;
    }

    panic_dispatch("unhandled breakpoint exception\n")
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum BreakpointHookResult {
    NotHandled,
    Resume,
}

pub type BreakpointHook = fn(&mut TrapFrame) -> BreakpointHookResult;

static mut BREAKPOINT_HOOKS: [Option<BreakpointHook>; 4] = [None; 4];

pub fn register_breakpoint_hook(hook: BreakpointHook) -> bool {
    unsafe {
        for slot in (&raw mut BREAKPOINT_HOOKS).as_mut().unwrap().iter_mut() {
            if slot.is_none() {
                *slot = Some(hook);
                return true;
            }
        }
    }

    false
}

fn run_breakpoint_hooks(frame: &mut TrapFrame) -> BreakpointHookResult {
    unsafe {
        for hook in (&raw const BREAKPOINT_HOOKS)
            .as_ref()
            .unwrap()
            .iter()
            .flatten()
        {
            match hook(frame) {
                BreakpointHookResult::NotHandled => {}
                BreakpointHookResult::Resume => return BreakpointHookResult::Resume,
            }
        }
    }

    BreakpointHookResult::NotHandled
}

pub fn resume_after_breakpoint(frame: &mut TrapFrame) {
    trace::checkpoint(Checkpoint::BreakpointExceptionHandled);
    frame.sepc = frame
        .sepc
        .wrapping_add(breakpoint_instruction_length(frame.sepc));
}

fn unexpected_exception_handler(frame: &TrapFrame) -> ! {
    panic_dispatch_frame("unexpected exception", frame)
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

fn panic_dispatch_frame(message: &str, frame: &TrapFrame) -> ! {
    crate::arch::riscv64::sbi::putstr(message);
    crate::arch::riscv64::sbi::putstr(" scause=0x");
    print_hex(frame.scause);
    crate::arch::riscv64::sbi::putstr(" sepc=0x");
    print_hex(frame.sepc);
    crate::arch::riscv64::sbi::putstr(" stval=0x");
    print_hex(frame.stval);
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

fn print_decimal(mut value: usize) {
    let mut digits = [0u8; 20];
    let mut len = 0usize;
    if value == 0 {
        crate::arch::riscv64::sbi::putchar(b'0');
        return;
    }
    while value != 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len != 0 {
        len -= 1;
        crate::arch::riscv64::sbi::putchar(digits[len]);
    }
}
