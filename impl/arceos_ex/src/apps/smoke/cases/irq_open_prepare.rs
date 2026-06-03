use crate::{
    apps::smoke::SmokeResult,
    arch::riscv64::csr,
    context::context,
    objects::{printk, state::State},
    phases,
};

const TIME_ADVANCE_SPIN_LIMIT: usize = 1_000_000;

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::interrupt::irq_open_prepare::is_ready()
        || !phases::interrupt::is_ready()
        || !csr::supervisor_interrupts_enabled()
    {
        printk::write_str("irq open prepare phase is not ready\n");
        return SmokeResult::Failed;
    }

    if ctx.slub_allocator.state() != State::Ready
        || !ctx.slub_allocator.flush_workqueue_ready()
        || ctx.workqueue.state() != State::Prepared
        || ctx.workqueue.workers_running()
    {
        printk::write_str("irq open SLUB/workqueue facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.console.state() != State::Prepared
        || ctx.console.line_discipline_registry().state() != State::Prepared
        || !ctx.console.line_discipline_registry().n_tty_registered()
        || ctx.console.line_discipline_registry().registered_slot() != 0
        || ctx.console.driver_set().state() != State::Prepared
        || !ctx.console.driver_set().early_registered()
        || !ctx.console.driver_set().serial_probe_deferred()
        || !ctx.console.driver_set().boot_console_unregister_deferred()
    {
        printk::write_str("console prepared facts invalid\n");
        return SmokeResult::Failed;
    }

    let Some(first_sched_clock) = ctx.sched_clock.read(&ctx.riscv_timer_provider) else {
        printk::write_str("sched clock read unavailable\n");
        return SmokeResult::Failed;
    };
    let mut second_sched_clock = first_sched_clock;
    let mut spins = 0usize;
    while second_sched_clock == first_sched_clock {
        if spins == TIME_ADVANCE_SPIN_LIMIT {
            printk::write_str("sched clock did not advance\n");
            return SmokeResult::Failed;
        }
        spins += 1;
        let Some(now) = ctx.sched_clock.read(&ctx.riscv_timer_provider) else {
            printk::write_str("sched clock read failed after first read\n");
            return SmokeResult::Failed;
        };
        second_sched_clock = now;
    }

    printk::write_fmt(format_args!(
        "sched_clock_delta={} lpj_fine={}\n",
        second_sched_clock.wrapping_sub(first_sched_clock),
        ctx.delay_loop.lpj_fine()
    ));
    SmokeResult::Passed
}
