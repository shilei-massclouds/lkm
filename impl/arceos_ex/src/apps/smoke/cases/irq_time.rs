use crate::{
    apps::smoke::SmokeResult,
    arch::riscv64::{csr, sbi},
    context::context,
    objects::{irq_time, printk, state::State},
    phases,
};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

static CLOCKEVENT_CALLBACKS: AtomicUsize = AtomicUsize::new(0);
static CLOCKEVENT_DEADLINE: AtomicU64 = AtomicU64::new(0);
const TIME_ADVANCE_SPIN_LIMIT: usize = 1_000_000;

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

    let Some(first_time) = ctx.riscv_timer_provider.read_time() else {
        printk::write_str("time read action unavailable\n");
        return SmokeResult::Failed;
    };
    let mut second_time = first_time;
    let mut time_spins = 0usize;
    while second_time == first_time {
        if time_spins == TIME_ADVANCE_SPIN_LIMIT {
            printk::write_str("time source did not advance\n");
            return SmokeResult::Failed;
        }
        time_spins += 1;

        let Some(now) = ctx.riscv_timer_provider.read_time() else {
            printk::write_str("time read action failed after first read\n");
            return SmokeResult::Failed;
        };
        second_time = now;
    }

    CLOCKEVENT_CALLBACKS.store(0, Ordering::Relaxed);
    CLOCKEVENT_DEADLINE.store(0, Ordering::Relaxed);

    let interrupts_before = irq_time::timer_interrupt_count();
    let delta = (ctx.riscv_timer_provider.timebase_hz() / 1000).max(1);
    let timeout = (ctx.riscv_timer_provider.timebase_hz() / 10).max(delta * 2);
    let timeout_deadline = sbi::read_time().wrapping_add(timeout);

    let Some(clockevent_deadline) = ctx
        .riscv_timer_provider
        .schedule_oneshot(delta, clockevent_callback)
    else {
        printk::write_str("failed to schedule clockevent callback\n");
        return SmokeResult::Failed;
    };

    while CLOCKEVENT_CALLBACKS.load(Ordering::Relaxed) == 0 {
        if sbi::read_time().wrapping_sub(timeout_deadline) < (1u64 << 63) {
            printk::write_str("clockevent callback did not fire\n");
            csr::disable_supervisor_timer_interrupt();
            return SmokeResult::Failed;
        }
        core::hint::spin_loop();
    }

    if !csr::supervisor_interrupts_enabled() {
        printk::write_str("timer interrupt returned with interrupts disabled\n");
        return SmokeResult::Failed;
    }
    let callback_count = CLOCKEVENT_CALLBACKS.load(Ordering::Relaxed);
    let callback_deadline = CLOCKEVENT_DEADLINE.load(Ordering::Relaxed);
    if irq_time::timer_interrupt_count() != interrupts_before.wrapping_add(1)
        || callback_count != 1
        || callback_deadline != clockevent_deadline
    {
        printk::write_str("clockevent callback facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "time_delta={} clockevent_callbacks={} timer_interrupts={} timebase_hz={}\n",
        second_time.wrapping_sub(first_time),
        callback_count,
        irq_time::timer_interrupt_count(),
        ctx.riscv_timer_provider.timebase_hz()
    ));
    SmokeResult::Passed
}

fn clockevent_callback(deadline: u64) {
    CLOCKEVENT_DEADLINE.store(deadline, Ordering::Relaxed);
    CLOCKEVENT_CALLBACKS.fetch_add(1, Ordering::Relaxed);
}
