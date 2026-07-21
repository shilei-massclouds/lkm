use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{cpu_control::CurrentTaskRef, printk, state::State},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let Some(boot_scheduler_view) = ctx.scheduler.boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        printk::write_str("boot CPU scheduler view missing\n");
        return SmokeResult::Failed;
    };
    let boot_idle_setup_state = boot_scheduler_view.idle_task();

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
        || !ctx.scheduler.boot_idle_preemption().disabled()
    {
        printk::write_str("current CPU or idle task control facts invalid\n");
        return SmokeResult::Failed;
    }
    if !ctx.boot_cpu_local_interrupt.enabled() {
        printk::write_str("interrupts not enabled before scheduler smoke\n");
        return SmokeResult::Failed;
    }

    if !ctx.softirq.rcu_action_registered()
        || !ctx.rcu_core.softirq_registered()
        || !ctx.rcu_core.node_tree_ready()
        || !ctx.rcu_core.node_locks_ready()
        || !ctx.rcu_core.node_waitqueues_ready()
        || !ctx.rcu_core.node_poll_work_ready()
        || !ctx.rcu_core.percpu_data_ready()
        || !ctx.rcu_core.kfree_batch_ready()
        || !ctx.rcu_core.kfree_shrinker_registered()
        || !ctx.rcu_core.pm_notifier_registered()
        || !ctx.rcu_core.runtime_read_side_full_semantics_deferred()
        || !ctx.rcu_core.tasks_rcu().percpu_arrays_ready()
        || !ctx.rcu_core.tasks_rcu().percpu_locks_ready()
        || !ctx.rcu_core.tasks_rcu().percpu_work_ready()
        || !ctx.rcu_core.tasks_rcu().barrier_heads_ready()
    {
        printk::write_str("sched init RCU or softirq facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.sched_init_prelude_trimmed_paths.state() != State::Ready
        || !ctx
            .sched_init_prelude_trimmed_paths
            .poking_init_trimmed_noop()
        || !ctx
            .sched_init_prelude_trimmed_paths
            .ftrace_init_trimmed_noop()
        || !ctx
            .sched_init_prelude_trimmed_paths
            .ftrace_trimmed_because_mcount_record_disabled()
        || !ctx
            .sched_init_prelude_trimmed_paths
            .early_trace_init_deferred()
        || !ctx.sched_init_prelude_trimmed_paths.position_preserved()
        || ctx.sched_init_trace_context_boundaries.state() != State::Ready
        || !ctx
            .sched_init_trace_context_boundaries
            .trace_init_deferred()
        || !ctx
            .sched_init_trace_context_boundaries
            .context_tracking_init_trimmed_noop()
        || !ctx
            .sched_init_trace_context_boundaries
            .context_tracking_trimmed_because_user_force_disabled()
        || !ctx.sched_init_trace_context_boundaries.position_preserved()
    {
        printk::write_str("sched init boundary facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.scheduler.state() != State::Online
        || ctx.scheduler.schedule_passes() == 0
        || ctx.scheduler.current_runqueue_resolve_passes() == 0
        || ctx.scheduler.pick_next_task_passes() == 0
        || ctx.scheduler.switch_to_passes() == 0
        || ctx.scheduler.pick_next_task_exit_count() == 0
        || ctx.scheduler.switch_to_entry_count() == 0
        || ctx.scheduler.schedule_preemption_disable_count() == 0
        || ctx.scheduler.schedule_preemption_enable_no_resched_count() == 0
        || ctx.scheduler.scheduler_rcu_context_switch_count() == 0
        || ctx.scheduler.scheduler_rq_lock_mb_after_spinlock_count() == 0
        || ctx.scheduler.scheduler_rq_clock_update_count() == 0
        || ctx.scheduler.scheduler_need_resched_clear_count() == 0
        || ctx.scheduler.scheduler_rq_curr_publish_rcu_count() == 0
        || ctx.scheduler.scheduler_trace_sched_switch_count() == 0
        || ctx.scheduler.scheduler_prepare_task_switch_count() == 0
        || ctx.scheduler.scheduler_finish_task_switch_count() == 0
        || ctx.scheduler.scheduler_finish_released_rq_lock_count() == 0
        || ctx.scheduler.scheduler_finish_preempt_count_restore_count() == 0
        || !ctx.scheduler.scheduler_switch_mm_or_lazy_tlb_deferred()
        || !ctx.scheduler.scheduler_membarrier_switch_barrier_deferred()
        || ctx.scheduler.boot_runqueue_lock().irqsave_entered_count() == 0
        || ctx.scheduler.boot_runqueue_lock().irqrestore_exited_count() == 0
        || ctx.scheduler.pick_next_task_exit_prev_ref() != CurrentTaskRef::BootTask
        || ctx.scheduler.pick_next_task_exit_next_ref() != CurrentTaskRef::KernelInit
        || ctx.boot_cpu_current_task.switch_committed_count() == 0
        || !ctx.boot_cpu_current_task.current_is_kernel_init()
        || ctx.boot_cpu_current_task.current() != CurrentTaskRef::KernelInit
        || !boot_cpu_owned_scheduler_view_matches()
        || !scheduler_possible_runqueues_match_cpu_group()
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
            .covers_cpu_ref(boot_scheduler_view.runqueue().cpu_ref())
        || !boot_idle_setup_state.switch_ctx_initialized()
        || boot_idle_setup_state.core_saved_count() == 0
        || boot_idle_setup_state.core_restored_count() == 0
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
        boot_scheduler_view.runqueue().cpu_id(),
        boot_idle_setup_state.task_id()
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

fn scheduler_possible_runqueues_match_cpu_group() -> bool {
    let ctx = context();
    if ctx.scheduler.cpu_runqueue_count() != ctx.cpu_group.possible_cpu_count()
        || !ctx.scheduler.possible_cpu_runqueues_ready(&ctx.cpu_group)
        || !ctx.scheduler.boot_runqueue_matches_metadata()
    {
        return false;
    }

    let mut logical_id = 0usize;
    while logical_id < ctx.cpu_group.possible_cpu_count() {
        let Some(runqueue) = ctx.scheduler.cpu_runqueue(logical_id) else {
            return false;
        };
        let Some(cpu) = ctx.cpu_group.cpu(logical_id) else {
            return false;
        };
        let Some(cpu_ref) = ctx.cpu_group.possible_cpu_ref_at(logical_id) else {
            return false;
        };
        if runqueue.state() != State::Ready
            || runqueue.cpu_ref() != cpu_ref
            || runqueue.cpu_ref() != cpu.cpu_ref()
            || runqueue.cpu_id() != logical_id
            || runqueue.cpu_hartid() != cpu.hartid()
            || !runqueue.class_queues_ready()
            || !runqueue.attached_to_root_domain()
            || runqueue.balance_push_enabled()
            || !ctx
                .scheduler
                .default_root_domain()
                .covers_cpu_ref(runqueue.cpu_ref())
        {
            return false;
        }
        if logical_id == 0 {
            if !runqueue.is_boot_backed()
                || runqueue.cpu_ref() != cpu.cpu_ref()
                || runqueue.cpu_hartid() != cpu.hartid()
            {
                return false;
            }
        } else if runqueue.is_boot_backed() {
            return false;
        }
        logical_id += 1;
    }

    ctx.scheduler
        .cpu_runqueue(ctx.cpu_group.possible_cpu_count())
        .is_none()
}

fn boot_cpu_owned_scheduler_view_matches() -> bool {
    let ctx = context();
    if !ctx
        .scheduler
        .boot_cpu_owned_scheduler_view_ready(&ctx.cpu_group)
    {
        return false;
    }

    let Some(view) = ctx.scheduler.boot_cpu_owned_scheduler_view(&ctx.cpu_group) else {
        return false;
    };
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return false;
    };
    let runqueue = view.runqueue();
    let idle_task = view.idle_task();

    view.cpu_ref() == boot_cpu.cpu_ref()
        && view.cpu_id() == boot_cpu.logical_id()
        && view.cpu_hartid() == boot_cpu.hartid()
        && runqueue.cpu_ref() == boot_cpu.cpu_ref()
        && runqueue.cpu_hartid() == boot_cpu.hartid()
        && runqueue.is_boot_backed()
        && idle_task.cpu_ref() == boot_cpu.cpu_ref()
        && idle_task.cpu_id() == boot_cpu.logical_id()
        && view.runqueue_current_task_id() == idle_task.task_id()
        && view.runqueue_idle_task_id() == idle_task.task_id()
        && view.runqueue_task_count() == ctx.scheduler.boot_runqueue().task_count()
        && view.runqueue_idle_task_matches()
        && idle_task.uses_current_init_task()
        && idle_task.lazy_tlb_mm_ready()
        && idle_task.no_set_affinity()
}
