use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use crate::{
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    trace::Checkpoint,
};

const ANNOUNCE_MODE_EARLY_BYTE: u8 = 0;
const ANNOUNCE_MODE_NAMED_STRING: u8 = 1;

static ANNOUNCE_MODE: AtomicU8 = AtomicU8::new(ANNOUNCE_MODE_EARLY_BYTE);
static ANNOUNCE_LINE_LOCK: AtomicBool = AtomicBool::new(false);

struct AnnounceLineGuard;

impl AnnounceLineGuard {
    fn acquire() -> Self {
        while ANNOUNCE_LINE_LOCK
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        Self
    }
}

impl Drop for AnnounceLineGuard {
    fn drop(&mut self) {
        ANNOUNCE_LINE_LOCK.store(false, Ordering::Release);
    }
}

pub const HANDLER: Handler = Handler {
    name: "announce",
    priority: 0,
    scope: HandlerScope::All,
    run: HandlerRun::Observe(run),
};

pub fn enable_named_announcements() {
    if ANNOUNCE_MODE.swap(ANNOUNCE_MODE_NAMED_STRING, Ordering::Relaxed) == ANNOUNCE_MODE_EARLY_BYTE
    {
        crate::arch::riscv64::sbi::putchar(b'\n');
    }
}

pub fn emit(checkpoint: Checkpoint) {
    if ANNOUNCE_MODE.load(Ordering::Relaxed) == ANNOUNCE_MODE_EARLY_BYTE {
        crate::arch::riscv64::sbi::putchar(checkpoint.early_byte());
        return;
    }

    announce_name(checkpoint);
}

pub fn emit_with_context(checkpoint: Checkpoint, ctx: &Context) {
    if ANNOUNCE_MODE.load(Ordering::Relaxed) == ANNOUNCE_MODE_EARLY_BYTE {
        crate::arch::riscv64::sbi::putchar(checkpoint.early_byte());
        return;
    }

    announce_name_with_context(checkpoint, ctx);
}

pub fn emit_with_ap_idle_task(checkpoint: Checkpoint, logical_id: usize) {
    if ANNOUNCE_MODE.load(Ordering::Relaxed) == ANNOUNCE_MODE_EARLY_BYTE {
        crate::arch::riscv64::sbi::putchar(checkpoint.early_byte());
        return;
    }

    announce_name_with_ap_idle_task(checkpoint, logical_id);
}

pub fn announce_name(checkpoint: Checkpoint) {
    let _guard = AnnounceLineGuard::acquire();
    crate::arch::riscv64::sbi::putstr("checkpoint: ");
    crate::arch::riscv64::sbi::putstr(checkpoint.name());
    crate::arch::riscv64::sbi::putchar(b'\n');
}

pub fn announce_name_with_context(checkpoint: Checkpoint, ctx: &Context) {
    let _guard = AnnounceLineGuard::acquire();
    crate::arch::riscv64::sbi::putstr("checkpoint: ");
    crate::arch::riscv64::sbi::putstr(checkpoint.name());
    crate::arch::riscv64::sbi::putstr(" task=");
    crate::arch::riscv64::sbi::putstr(ctx.boot_cpu_current_task.current().name());
    crate::arch::riscv64::sbi::putchar(b'\n');
}

pub fn announce_name_with_ap_idle_task(checkpoint: Checkpoint, logical_id: usize) {
    let _guard = AnnounceLineGuard::acquire();
    crate::arch::riscv64::sbi::putstr("checkpoint: ");
    crate::arch::riscv64::sbi::putstr(checkpoint.name());
    crate::arch::riscv64::sbi::putstr(" task=ApIdleTask[");
    put_usize_decimal(logical_id);
    crate::arch::riscv64::sbi::putstr("]\n");
}

fn put_usize_decimal(mut value: usize) {
    let mut digits = [0u8; 20];
    let mut index = digits.len();

    if value == 0 {
        crate::arch::riscv64::sbi::putchar(b'0');
        return;
    }

    while value != 0 && index != 0 {
        index -= 1;
        digits[index] = b'0' + (value % 10) as u8;
        value /= 10;
    }

    crate::arch::riscv64::sbi::putstr(core::str::from_utf8(&digits[index..]).unwrap_or("?"));
}

fn run(checkpoint: Checkpoint, ctx: &Context, _sink: &mut dyn Sink) -> CheckpointOutcome {
    announce_name_with_context(checkpoint, ctx);
    CheckpointOutcome::Continue
}
