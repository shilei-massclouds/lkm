use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{cpu_control::CurrentTaskRef, printk, state::State},
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
        || !ctx.boot_cpu_current_task.current_is_kernel_init()
        || ctx.boot_cpu_current_task.current() != CurrentTaskRef::KernelInit
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

    if ctx.scheduler.state() != State::Online
        || ctx.scheduler.schedule_passes() == 0
        || ctx.scheduler.current_runqueue_resolve_passes() == 0
        || ctx.scheduler.pick_next_task_passes() == 0
        || ctx.scheduler.switch_to_passes() == 0
        || ctx.scheduler.pick_next_task_exit_count() == 0
        || ctx.scheduler.switch_to_entry_count() == 0
        || ctx.scheduler.pick_next_task_exit_prev_ref() != CurrentTaskRef::BootIdle
        || ctx.scheduler.pick_next_task_exit_next_ref() != CurrentTaskRef::KernelInit
        || ctx.boot_cpu_current_task.switch_committed_count() == 0
        || !ctx.boot_cpu_current_task.current_is_kernel_init()
        || ctx.boot_cpu_current_task.current() != CurrentTaskRef::KernelInit
        || ctx.scheduler.boot_runqueue().curr_task_id() != ctx.scheduler.boot_idle_task().task_id()
        || ctx.scheduler.boot_runqueue().idle_task_id() != ctx.scheduler.boot_idle_task().task_id()
        || ctx.scheduler.boot_idle_task().cpu_ref() != ctx.scheduler.boot_runqueue().cpu_ref()
        || ctx.scheduler.default_root_domain().covered_cpu_count()
            != ctx.cpu_group.possible_cpu_count()
        || ctx.scheduler.default_root_domain().covered_cpu_ref(0) != ctx.cpu_group.boot_cpu_ref()
        || !default_root_domain_entries_match_cpu_group()
        || !ctx
            .scheduler
            .default_root_domain()
            .covers_cpu_group_possible(&ctx.cpu_group)
        || !ctx
            .scheduler
            .default_root_domain()
            .covers_cpu_ref(ctx.scheduler.boot_runqueue().cpu_ref())
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
            == 0
        || ctx
            .scheduler
            .boot_idle_task()
            .thread_context()
            .core_restored_count()
            == 0
        || ctx.scheduler.idle_schedule_passes() == 0
        || ctx.scheduler.idle_schedule_returned_passes() != ctx.scheduler.idle_schedule_passes()
        || ctx.scheduler.idle_schedule_identity_passes() != 0
        || ctx.scheduler.identity_switch_passes() != 0
        || ctx.scheduler.switch_to_passes() <= ctx.scheduler.idle_schedule_passes()
        || ctx.boot_cpu_current_task.switch_committed_count()
            <= ctx.scheduler.idle_schedule_passes()
        || ctx.boot_cpu_local_interrupt.saved_and_disabled_count() == 0
        || ctx.boot_cpu_local_interrupt.restored_count() == 0
    {
        printk::write_str("scheduler first-switch facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "schedule_passes={} switch_to_passes={} idle_schedule_passes={} boot_cpu={} current_task={}\n",
        ctx.scheduler.schedule_passes(),
        ctx.scheduler.switch_to_passes(),
        ctx.scheduler.idle_schedule_passes(),
        ctx.scheduler.boot_runqueue().cpu_id(),
        ctx.scheduler.boot_idle_task().task_id()
    ));
    SmokeResult::Passed
}

fn default_root_domain_entries_match_cpu_group() -> bool {
    let ctx = context();
    let root_domain = ctx.scheduler.default_root_domain();
    let mut logical_id = 0usize;
    while logical_id < ctx.cpu_group.possible_cpu_count() {
        if root_domain.covered_cpu_ref(logical_id) != ctx.cpu_group.possible_cpu_ref_at(logical_id)
        {
            return false;
        }
        logical_id += 1;
    }
    true
}
