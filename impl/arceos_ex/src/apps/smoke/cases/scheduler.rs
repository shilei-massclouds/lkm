use crate::{
    apps::smoke::SmokeResult,
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
    if !ctx.boot_cpu_local_interrupt.enabled() {
        printk::write_str("interrupts not enabled before scheduler smoke\n");
        return SmokeResult::Failed;
    }

    let before = ctx.scheduler.schedule_passes();
    let switch_before = ctx.scheduler.switch_to_passes();
    let identity_switch_before = ctx.scheduler.identity_switch_passes();
    let core_saved_before = ctx
        .scheduler
        .boot_idle_task()
        .thread_context()
        .core_saved_count();
    let core_restored_before = ctx
        .scheduler
        .boot_idle_task()
        .thread_context()
        .core_restored_count();
    let saved_before = ctx.boot_cpu_local_interrupt.saved_and_disabled_count();
    let restored_before = ctx.boot_cpu_local_interrupt.restored_count();
    if ctx.scheduler.boot_idle_preemption_mut().disable().is_err() {
        printk::write_str("failed to enter scheduler smoke critical state\n");
        return SmokeResult::Failed;
    }
    let enable_no_resched = ctx.scheduler.boot_idle_preemption_mut().enable_no_resched();
    let schedule = ctx.scheduler.schedule(&mut ctx.boot_cpu_local_interrupt);
    let disable = ctx.scheduler.boot_idle_preemption_mut().disable();
    if enable_no_resched.is_err() || schedule.is_err() || disable.is_err() {
        printk::write_str("schedule boundary failed\n");
        let _ = ctx.scheduler.boot_idle_preemption_mut().enable();
        return SmokeResult::Failed;
    }
    if ctx.scheduler.boot_idle_preemption_mut().enable().is_err() {
        printk::write_str("failed to leave scheduler smoke critical state\n");
        return SmokeResult::Failed;
    }

    if !ctx.boot_cpu_local_interrupt.enabled() {
        printk::write_str("scheduler smoke failed to restore interrupts\n");
        return SmokeResult::Failed;
    }
    if ctx.scheduler.state() != State::Online
        || ctx.scheduler.schedule_passes() != before.wrapping_add(1)
        || ctx.scheduler.switch_to_passes() != switch_before.wrapping_add(1)
        || ctx.scheduler.identity_switch_passes() != identity_switch_before.wrapping_add(1)
        || ctx.scheduler.boot_runqueue().curr_task_id() != ctx.scheduler.boot_idle_task().task_id()
        || ctx.scheduler.boot_runqueue().idle_task_id() != ctx.scheduler.boot_idle_task().task_id()
        || !ctx
            .scheduler
            .boot_idle_task()
            .thread_context()
            .core_register_set()
        || ctx
            .scheduler
            .boot_idle_task()
            .thread_context()
            .core_saved_count()
            != core_saved_before.wrapping_add(1)
        || ctx
            .scheduler
            .boot_idle_task()
            .thread_context()
            .core_restored_count()
            != core_restored_before.wrapping_add(1)
        || ctx.boot_cpu_local_interrupt.saved_and_disabled_count() != saved_before.wrapping_add(1)
        || ctx.boot_cpu_local_interrupt.restored_count() != restored_before.wrapping_add(1)
    {
        printk::write_str("scheduler single-task pass facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "schedule_passes={} switch_to_passes={} boot_cpu={} current_task={}\n",
        ctx.scheduler.schedule_passes(),
        ctx.scheduler.switch_to_passes(),
        ctx.scheduler.boot_runqueue().cpu_id(),
        ctx.scheduler.boot_idle_task().task_id()
    ));
    SmokeResult::Passed
}
