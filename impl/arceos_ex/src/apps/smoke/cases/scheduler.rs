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
    if ctx.boot_current_cpu.state() != State::Online
        || !ctx.boot_current_cpu.owns_boot_cpu()
        || !ctx.boot_current_cpu.registered_in_cpu_group()
        || ctx.boot_cpu_current_task.state() != State::Ready
        || !ctx.boot_cpu_current_task.current_is_boot_idle()
        || ctx.boot_cpu_current_task.current()
            != crate::objects::cpu_control::CurrentTaskRef::BootIdle
        || ctx.scheduler.boot_idle_preemption().state() != State::Ready
        || !ctx.scheduler.boot_idle_preemption().enabled()
    {
        printk::write_str("current CPU or idle task control facts invalid\n");
        return SmokeResult::Failed;
    }
    let interrupts_enabled = csr::supervisor_interrupts_enabled();
    if !interrupts_enabled {
        printk::write_str("interrupts not enabled before scheduler smoke\n");
        return SmokeResult::Failed;
    }

    let before = ctx.scheduler.preempt_disabled_passes();
    if ctx.boot_cpu_local_interrupt.save_and_disable().is_err()
        || ctx.scheduler.boot_idle_preemption_mut().disable().is_err()
    {
        printk::write_str("failed to enter scheduler smoke critical state\n");
        return SmokeResult::Failed;
    }
    if ctx.scheduler.schedule_preempt_disabled().is_err() {
        printk::write_str("schedule_preempt_disabled failed\n");
        let _ = ctx.scheduler.boot_idle_preemption_mut().enable();
        let _ = ctx.boot_cpu_local_interrupt.restore();
        return SmokeResult::Failed;
    }
    if ctx.scheduler.boot_idle_preemption_mut().enable().is_err()
        || ctx.boot_cpu_local_interrupt.restore().is_err()
    {
        printk::write_str("failed to leave scheduler smoke critical state\n");
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
        || ctx.boot_cpu_local_interrupt.saved_and_disabled_count() == 0
        || ctx.boot_cpu_local_interrupt.restored_count() == 0
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
