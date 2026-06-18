pub mod handlers;
#[cfg(any(
    checkpoint_sbi_char,
    checkpoint_handler_page_allocator,
    checkpoint_handler_earlycon,
    checkpoint_handler_kernel_init_task,
    checkpoint_handler_linux_plic,
    checkpoint_handler_scheduler_action,
    checkpoint_handler_console_handoff,
    checkpoint_handler_uart_irq_chain,
    checkpoint_handler_virtio_bus,
    checkpoint_handler_of_platform
))]
mod kunit;

use core::sync::atomic::{AtomicBool, Ordering};

pub use self::handlers::CheckpointOutcome;
use crate::{context::Context, trace::Checkpoint};

static POST_VM_CHECKPOINTS_ENABLED: AtomicBool = AtomicBool::new(false);
static CHECKPOINT_HANDLER_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn enable_post_vm_checkpoints() {
    POST_VM_CHECKPOINTS_ENABLED.store(true, Ordering::Release);
}

pub fn dispatch_pre_context(_checkpoint: Checkpoint) {
    #[cfg(checkpoint_sbi_char)]
    {
        if POST_VM_CHECKPOINTS_ENABLED.load(Ordering::Acquire) {
            handlers::early_trace::emit_with_context(_checkpoint, crate::context::context_ref());
        } else {
            handlers::early_trace::emit(_checkpoint);
        }
    }
}

pub fn dispatch(checkpoint: Checkpoint, ctx: &Context) {
    if !POST_VM_CHECKPOINTS_ENABLED.load(Ordering::Acquire) || !handlers::has_post_vm_handlers() {
        return;
    }

    if CHECKPOINT_HANDLER_ACTIVE.swap(true, Ordering::AcqRel) {
        checkpoint_reentry_shutdown();
    }

    let outcome = handlers::dispatch(checkpoint, ctx);
    CHECKPOINT_HANDLER_ACTIVE.store(false, Ordering::Release);
    apply_outcome(checkpoint, outcome);
}

fn apply_outcome(checkpoint: Checkpoint, outcome: CheckpointOutcome) {
    match outcome {
        CheckpointOutcome::Continue => {}
        CheckpointOutcome::Passed => {}
        CheckpointOutcome::FailAndShutdown => {
            crate::arch::riscv64::sbi::putstr("# checkpoint fail: ");
            crate::arch::riscv64::sbi::putstr(checkpoint.name());
            crate::arch::riscv64::sbi::putchar(b'\n');
            crate::arch::riscv64::sbi::system_shutdown();
        }
        CheckpointOutcome::StopAndShutdown => {
            crate::arch::riscv64::sbi::putstr("# checkpoint stop: ");
            crate::arch::riscv64::sbi::putstr(checkpoint.name());
            crate::arch::riscv64::sbi::putchar(b'\n');
            crate::arch::riscv64::sbi::system_shutdown();
        }
    }
}

fn checkpoint_reentry_shutdown() -> ! {
    crate::arch::riscv64::sbi::putstr("checkpoint reentry\n");
    crate::arch::riscv64::sbi::system_shutdown()
}
