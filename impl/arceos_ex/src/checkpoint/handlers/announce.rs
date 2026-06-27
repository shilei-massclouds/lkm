use core::sync::atomic::{AtomicU8, Ordering};

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

pub fn announce_name(checkpoint: Checkpoint) {
    crate::arch::riscv64::sbi::putstr("checkpoint: ");
    crate::arch::riscv64::sbi::putstr(checkpoint.name());
    crate::arch::riscv64::sbi::putchar(b'\n');
}

pub fn announce_name_with_context(checkpoint: Checkpoint, ctx: &Context) {
    crate::arch::riscv64::sbi::putstr("checkpoint: ");
    crate::arch::riscv64::sbi::putstr(checkpoint.name());
    crate::arch::riscv64::sbi::putstr(" task=");
    crate::arch::riscv64::sbi::putstr(ctx.boot_cpu_current_task.current().name());
    crate::arch::riscv64::sbi::putchar(b'\n');
}

fn run(checkpoint: Checkpoint, ctx: &Context, _sink: &mut dyn Sink) -> CheckpointOutcome {
    announce_name_with_context(checkpoint, ctx);
    CheckpointOutcome::Continue
}
