use crate::{
    apps::smoke::SmokeResult,
    arch::riscv64::csr,
    context::context,
    objects::{printk, state::State},
};

const UDELAY_USECS: u64 = 25;

pub fn run() -> SmokeResult {
    let ctx = context();

    if ctx.delay_loop.state() != State::Ready || !ctx.delay_loop.delay_actions_ready() {
        printk::write_str("delay loop is not ready\n");
        return SmokeResult::Failed;
    }

    let expected_ticks = expected_delay_ticks(ctx.riscv_timer_provider.timebase_hz(), UDELAY_USECS);
    if expected_ticks == 0 {
        printk::write_str("delay loop expected tick count is zero\n");
        return SmokeResult::Failed;
    }

    let Some(elapsed_ticks) = ctx
        .delay_loop
        .udelay(&ctx.riscv_timer_provider, UDELAY_USECS)
    else {
        printk::write_str("udelay action unavailable\n");
        return SmokeResult::Failed;
    };

    if elapsed_ticks < expected_ticks {
        printk::write_str("udelay returned before target duration\n");
        return SmokeResult::Failed;
    }
    if !csr::supervisor_interrupts_enabled() {
        printk::write_str("udelay returned with interrupts disabled\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "udelay_usecs={} expected_ticks={} elapsed_ticks={}\n",
        UDELAY_USECS, expected_ticks, elapsed_ticks
    ));
    SmokeResult::Passed
}

fn expected_delay_ticks(timebase_hz: u64, usecs: u64) -> u64 {
    timebase_hz
        .saturating_mul(usecs)
        .saturating_div(1_000_000)
        .max(1)
}
