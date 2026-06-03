use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    context::Context,
    objects::{earlycon, printk},
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::PrintkBufferReady];

pub const HANDLER: Handler = Handler {
    name: "earlycon",
    priority: 100,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Write(run),
};

fn run(_checkpoint: Checkpoint, _ctx: &mut Context) -> CheckpointOutcome {
    if !earlycon::is_online() || !printk::is_ready() {
        putstr("kunit earlycon: early console or printk unavailable\n");
        return CheckpointOutcome::FailAndShutdown;
    }

    printk::write_fmt(format_args!(
        "kunit earlycon: formatted value={} hex={:#x}\n",
        42usize, 42usize
    ));
    earlycon::drain_printk();
    putstr("kunit earlycon: passed\n");
    CheckpointOutcome::Passed
}

fn putstr(message: &str) {
    crate::arch::riscv64::sbi::putstr(message);
}
