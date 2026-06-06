use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        cpu_control::{LocalInterruptControl, PreemptionControl, RawSpinLock},
        printk,
        state::State,
    },
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let mut local_interrupt = LocalInterruptControl::new();
    let mut preemption = PreemptionControl::new();
    let mut lock = RawSpinLock::new();

    if ctx.init_task.state() != State::Online
        || local_interrupt.setup().is_err()
        || preemption.setup(&ctx.init_task).is_err()
        || lock.setup().is_err()
    {
        printk::write_str("raw spinlock smoke setup failed\n");
        return SmokeResult::Failed;
    }

    if lock.state() != State::Ready
        || lock.locked()
        || !local_interrupt.disabled()
        || !preemption.enabled()
    {
        printk::write_str("raw spinlock initial facts invalid\n");
        return SmokeResult::Failed;
    }

    if local_interrupt.enable().is_err() || !local_interrupt.enabled() {
        printk::write_str("raw spinlock failed to open interrupt precondition\n");
        return SmokeResult::Failed;
    }

    if lock
        .lock_irqsave(&mut local_interrupt, &mut preemption)
        .is_err()
        || !lock.locked()
        || lock.irqsave_entered_count() != 1
        || !local_interrupt.disabled()
        || !preemption.disabled()
    {
        printk::write_str("raw spinlock irqsave facts invalid\n");
        return SmokeResult::Failed;
    }

    if lock
        .lock_irqsave(&mut local_interrupt, &mut preemption)
        .is_ok()
    {
        printk::write_str("raw spinlock contended lock should block\n");
        return SmokeResult::Failed;
    }

    if lock
        .unlock_irqrestore(&mut local_interrupt, &mut preemption)
        .is_err()
        || lock.locked()
        || lock.irqrestore_exited_count() != 1
        || !lock.irqrestore_restored_before_preemption_enabled()
        || !local_interrupt.enabled()
        || !preemption.enabled()
    {
        printk::write_str("raw spinlock irqrestore facts invalid\n");
        return SmokeResult::Failed;
    }

    if lock
        .unlock_irqrestore(&mut local_interrupt, &mut preemption)
        .is_ok()
    {
        printk::write_str("raw spinlock unlocked restore should fail\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "raw_spinlock irqsave={} irqrestore={} saved={} restored={}\n",
        lock.irqsave_entered_count(),
        lock.irqrestore_exited_count(),
        local_interrupt.saved_and_disabled_count(),
        local_interrupt.restored_count()
    ));
    SmokeResult::Passed
}
