use core::sync::atomic::{AtomicU8, Ordering};

use crate::trace::Checkpoint;

const TRACE_MODE_EARLY_BYTE: u8 = 0;
const TRACE_MODE_NAMED_STRING: u8 = 1;

static TRACE_MODE: AtomicU8 = AtomicU8::new(TRACE_MODE_EARLY_BYTE);

pub fn enable_named_checkpoints() {
    if TRACE_MODE.swap(TRACE_MODE_NAMED_STRING, Ordering::Relaxed) == TRACE_MODE_EARLY_BYTE {
        crate::arch::riscv64::sbi::putchar(b'\n');
    }
}

pub fn emit(checkpoint: Checkpoint) {
    if TRACE_MODE.load(Ordering::Relaxed) == TRACE_MODE_EARLY_BYTE {
        crate::arch::riscv64::sbi::putchar(checkpoint.early_byte());
        return;
    }

    trace_name(checkpoint);
}

pub fn trace_name(checkpoint: Checkpoint) {
    crate::arch::riscv64::sbi::putstr("trace: ");
    crate::arch::riscv64::sbi::putstr(checkpoint.name());
    crate::arch::riscv64::sbi::putchar(b'\n');
}
