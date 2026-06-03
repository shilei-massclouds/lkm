use crate::{
    apps::smoke::SmokeResult,
    arch::riscv64::csr,
    context::context,
    objects::{printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();

    if ctx.scheduler.state() != State::Online || !ctx.scheduler.scheduler_running() {
        printk::write_str("scheduler is not online\n");
        return SmokeResult::Failed;
    }
    let interrupts_enabled = csr::supervisor_interrupts_enabled();
    if !interrupts_enabled {
        printk::write_str("interrupts not enabled before scheduler smoke\n");
        return SmokeResult::Failed;
    }

    let before = ctx.scheduler.preempt_disabled_passes();
    csr::disable_supervisor_interrupts();
    if ctx.scheduler.schedule_preempt_disabled().is_err() {
        printk::write_str("schedule_preempt_disabled failed\n");
        csr::enable_supervisor_interrupts();
        return SmokeResult::Failed;
    }

    if !csr::supervisor_interrupts_enabled() {
        csr::enable_supervisor_interrupts();
    }
    if !csr::supervisor_interrupts_enabled() {
        printk::write_str("scheduler smoke failed to restore interrupts\n");
        return SmokeResult::Failed;
    }
    if ctx.scheduler.state() != State::Online
        || ctx.scheduler.preempt_disabled_passes() != before.wrapping_add(1)
        || ctx.scheduler.boot_runqueue().curr_task_id() != ctx.scheduler.boot_idle_task().task_id()
        || ctx.scheduler.boot_runqueue().idle_task_id() != ctx.scheduler.boot_idle_task().task_id()
    {
        printk::write_str("scheduler single-task pass facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "preempt_disabled_passes={} boot_cpu={} current_task={}\n",
        ctx.scheduler.preempt_disabled_passes(),
        ctx.scheduler.boot_runqueue().cpu_id(),
        ctx.scheduler.boot_idle_task().task_id()
    ));
    SmokeResult::Passed
}
