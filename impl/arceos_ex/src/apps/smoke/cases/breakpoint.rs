use core::{
    arch::global_asm,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    apps::smoke::SmokeResult,
    objects::{
        event_stream::TrapFrame,
        exception_stream::{self, BreakpointHookResult},
        printk,
    },
};

static BREAKPOINT_PROGRESS: AtomicUsize = AtomicUsize::new(0);
static BREAKPOINT_EXPECTED_SEPC: AtomicUsize = AtomicUsize::new(0);

global_asm!(
    r#"
    .section .text.smoke_breakpoint, "ax"
    .align 2
    .globl smoke_trigger_ebreak
smoke_trigger_ebreak:
    .globl smoke_ebreak_site
smoke_ebreak_site:
    ebreak
    ret

    .align 2
    .globl smoke_trigger_c_ebreak
smoke_trigger_c_ebreak:
    .globl smoke_c_ebreak_site
smoke_c_ebreak_site:
    .2byte 0x9002
    ret
"#
);

unsafe extern "C" {
    fn smoke_trigger_ebreak();
    fn smoke_trigger_c_ebreak();
    static smoke_ebreak_site: u8;
    static smoke_c_ebreak_site: u8;
}

pub fn run() -> SmokeResult {
    BREAKPOINT_PROGRESS.store(0, Ordering::Relaxed);
    BREAKPOINT_EXPECTED_SEPC.store(0, Ordering::Relaxed);

    if !exception_stream::register_breakpoint_hook(smoke_breakpoint_hook) {
        printk::write_str("failed to register breakpoint smoke hook\n");
        return SmokeResult::Failed;
    }

    trigger_ebreak();
    if BREAKPOINT_PROGRESS.load(Ordering::Relaxed) != 1 {
        printk::write_str("32-bit ebreak did not resume\n");
        return SmokeResult::Failed;
    }

    trigger_c_ebreak();
    if BREAKPOINT_PROGRESS.load(Ordering::Relaxed) != 2 {
        printk::write_str("16-bit c.ebreak did not resume\n");
        return SmokeResult::Failed;
    }

    printk::write_str("Breakpoint exception resumed twice\n");
    SmokeResult::Passed
}

#[inline(never)]
fn trigger_ebreak() {
    let site = (&raw const smoke_ebreak_site) as usize;
    BREAKPOINT_EXPECTED_SEPC.store(site, Ordering::Relaxed);
    unsafe {
        smoke_trigger_ebreak();
    }
    BREAKPOINT_PROGRESS.store(1, Ordering::Relaxed);
}

#[inline(never)]
fn trigger_c_ebreak() {
    let site = (&raw const smoke_c_ebreak_site) as usize;
    BREAKPOINT_EXPECTED_SEPC.store(site, Ordering::Relaxed);
    unsafe {
        smoke_trigger_c_ebreak();
    }
    BREAKPOINT_PROGRESS.store(2, Ordering::Relaxed);
}

fn smoke_breakpoint_hook(frame: &mut TrapFrame) -> BreakpointHookResult {
    if frame.sepc != BREAKPOINT_EXPECTED_SEPC.load(Ordering::Relaxed) {
        return BreakpointHookResult::NotHandled;
    }

    exception_stream::resume_after_breakpoint(frame);
    BreakpointHookResult::Resume
}
