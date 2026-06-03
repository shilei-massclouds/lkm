use crate::{
    apps::smoke::SmokeResult,
    arch::riscv64::{csr, sbi},
    context::context,
    objects::{irq_time, printk, state::State},
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::boot::irq_time_init::is_ready()
        || ctx.interrupt_stream.state() != State::Online
        || !csr::supervisor_interrupts_enabled()
    {
        printk::write_str("irq time phase did not open boot CPU interrupts\n");
        return SmokeResult::Failed;
    }

    if ctx.irq_controller.state() != State::Ready
        || ctx.irq_dispatch_tree.state() != State::Ready
        || ctx.tick.state() != State::Ready
        || ctx.timer_wheel.state() != State::Ready
        || ctx.hrtimer_core.state() != State::Ready
        || ctx.timekeeper.state() != State::Ready
        || ctx.riscv_timer_provider.state() != State::Ready
        || ctx.softirq.state() != State::Ready
        || ctx.randomness.state() != State::Ready
        || ctx.sbi_ipi.state() != State::Ready
        || ctx.smp_call_function.state() != State::Ready
    {
        printk::write_str("irq time objects are not ready\n");
        return SmokeResult::Failed;
    }

    let before = irq_time::timer_interrupt_count();
    let delta = (ctx.riscv_timer_provider.timebase_hz() / 1000).max(1);
    let timeout = (ctx.riscv_timer_provider.timebase_hz() / 10).max(delta * 2);
    let deadline = sbi::read_time().wrapping_add(timeout);

    if !ctx.riscv_timer_provider.program_delta(delta) {
        printk::write_str("failed to program timer interrupt\n");
        return SmokeResult::Failed;
    }

    while irq_time::timer_interrupt_count() == before {
        if sbi::read_time().wrapping_sub(deadline) < (1u64 << 63) {
            printk::write_str("timer interrupt did not fire\n");
            csr::disable_supervisor_timer_interrupt();
            return SmokeResult::Failed;
        }
        core::hint::spin_loop();
    }

    if !csr::supervisor_interrupts_enabled() {
        printk::write_str("timer interrupt returned with interrupts disabled\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "timer_interrupts={} timebase_hz={}\n",
        irq_time::timer_interrupt_count(),
        ctx.riscv_timer_provider.timebase_hz()
    ));
    SmokeResult::Passed
}
