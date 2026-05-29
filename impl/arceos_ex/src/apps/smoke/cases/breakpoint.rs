use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{apps::smoke::SmokeResult, objects::printk};

static BREAKPOINT_PROGRESS: AtomicUsize = AtomicUsize::new(0);

pub fn run() -> SmokeResult {
    BREAKPOINT_PROGRESS.store(0, Ordering::Relaxed);

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
    unsafe {
        core::arch::asm!("ebreak", options(nostack));
    }
    BREAKPOINT_PROGRESS.store(1, Ordering::Relaxed);
}

#[inline(never)]
fn trigger_c_ebreak() {
    unsafe {
        core::arch::asm!(".2byte 0x9002", options(nostack));
    }
    BREAKPOINT_PROGRESS.store(2, Ordering::Relaxed);
}
