use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State, task::TaskRef},
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let Some(boot_scheduler_view) = ctx
        .scheduler()
        .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        printk::write_str("boot CPU scheduler view missing\n");
        return SmokeResult::Failed;
    };
    let boot_idle_setup_state = boot_scheduler_view.idle_task();

    if ctx.scheduler().state() != State::Online || !ctx.scheduler().scheduler_running() {
        printk::write_str("scheduler is not online\n");
        return SmokeResult::Failed;
    }
    if ctx.cpu_group.boot_cpu_state() != State::Online
        || !ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref == TaskRef::KERNEL_INIT)
        || ctx.scheduler().boot_idle_preemption().state() != State::Ready
        || ctx.scheduler().boot_idle_preemption().disabled()
    {
        printk::write_fmt(format_args!(
            "current CPU or idle task control facts invalid: cpu_online={} current_is_kernel_init={} idle_preemption_ready={} idle_preemption_disabled={}\n",
            ctx.cpu_group.boot_cpu_state() == State::Online,
            ctx.current_task_ref()
                .is_ok_and(|task_ref| task_ref == TaskRef::KERNEL_INIT),
            ctx.scheduler().boot_idle_preemption().state() == State::Ready,
            ctx.scheduler().boot_idle_preemption().disabled(),
        ));
        return SmokeResult::Failed;
    }
    if !ctx.boot_cpu_local_interrupt().enabled() {
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

    if ctx.scheduler().state() != State::Online
        || ctx.scheduler().schedule_passes() == 0
        || ctx.scheduler().current_runqueue_resolve_passes() == 0
        || ctx.scheduler().pick_next_task_passes() == 0
        || ctx.scheduler().switch_to_passes() == 0
        || ctx.scheduler().pick_next_task_exit_count() == 0
        || ctx.scheduler().switch_to_entry_count() == 0
        || ctx.scheduler().schedule_preemption_disable_count() == 0
        || ctx
            .scheduler()
            .schedule_preemption_enable_no_resched_count()
            == 0
        || ctx.scheduler().scheduler_rcu_context_switch_count() == 0
        || ctx.scheduler().scheduler_rq_lock_mb_after_spinlock_count() == 0
        || ctx.scheduler().scheduler_rq_clock_update_count() == 0
        || ctx.scheduler().scheduler_need_resched_clear_count() == 0
        || ctx.scheduler().scheduler_rq_curr_publish_rcu_count() == 0
        || ctx.scheduler().scheduler_trace_sched_switch_count() == 0
        || ctx.scheduler().scheduler_prepare_task_switch_count() == 0
        || ctx.scheduler().scheduler_finish_task_switch_count() == 0
        || ctx.scheduler().scheduler_finish_released_rq_lock_count() == 0
        || ctx
            .scheduler()
            .scheduler_finish_preempt_count_restore_count()
            == 0
        || !ctx.scheduler().scheduler_switch_mm_or_lazy_tlb_deferred()
        || !ctx
            .scheduler()
            .scheduler_membarrier_switch_barrier_deferred()
        || ctx.scheduler().boot_runqueue_lock().irqsave_entered_count() == 0
        || ctx
            .scheduler()
            .boot_runqueue_lock()
            .irqrestore_exited_count()
            == 0
        || ctx.scheduler().pick_next_task_exit_prev_ref() != TaskRef::BOOT
        || ctx.scheduler().pick_next_task_exit_next_ref() != TaskRef::KERNEL_INIT
        || !ctx
            .current_task_ref()
            .is_ok_and(|task_ref| task_ref == TaskRef::KERNEL_INIT)
        || !boot_cpu_owned_scheduler_view_matches()
        || !scheduler_possible_runqueues_match_cpu_group()
        || ctx
            .scheduler_shared
            .default_root_domain()
            .covered_cpu_count()
            != ctx.cpu_group.possible_cpu_count()
        || ctx
            .scheduler_shared
            .default_root_domain()
            .covered_cpu_ref(0)
            != ctx.cpu_group.boot_cpu_ref()
        || !default_root_domain_entries_match_cpu_group()
        || !ctx
            .scheduler_shared
            .default_root_domain()
            .covers_cpu_group_possible(&ctx.cpu_group)
        || !ctx
            .scheduler_shared
            .default_root_domain()
            .covers_cpu_ref(boot_scheduler_view.runqueue().cpu_ref())
        || !boot_idle_setup_state.switch_ctx_initialized()
        || boot_idle_setup_state.core_saved_count() == 0
        || boot_idle_setup_state.core_restored_count() != 0
        || ctx
            .kernel_init_task
            .task()
            .thread_context()
            .core_restored_count()
            == 0
        || ctx.scheduler().idle_schedule_passes() != 0
        || ctx.scheduler().idle_schedule_returned_passes() != 0
        || ctx.scheduler().idle_schedule_identity_passes() != 0
        || ctx.scheduler().identity_switch_passes() != 0
        || crate::flows::boot_idle_flow::entry_is_online()
        || ctx.scheduler().kernel_init_stack_switch_started_count() != 1
        || ctx.scheduler().kernel_init_stack_switch_returned_count() != 0
        || ctx.boot_cpu_local_interrupt().saved_and_disabled_count() == 0
        || ctx.boot_cpu_local_interrupt().restored_count() == 0
    {
        printk::write_fmt(format_args!(
            "scheduler first-switch snapshot schedule={} resolve={} pick={} switch={} pick_exit={} switch_entry={} disable={} enable={} rcu={} lock_mb={} clock={} need_resched={} curr_publish={} trace={} prepare={} finish={} rq_unlock={} preempt_restore={} irq_enter={} irq_exit={} boot_saved={} boot_restored={} next_restored={} idle={} idle_return={} idle_identity={} identity={} boot_entry={} stack_start={} stack_return={} saved_irq={} restored_irq={}\n",
            ctx.scheduler().schedule_passes(),
            ctx.scheduler().current_runqueue_resolve_passes(),
            ctx.scheduler().pick_next_task_passes(),
            ctx.scheduler().switch_to_passes(),
            ctx.scheduler().pick_next_task_exit_count(),
            ctx.scheduler().switch_to_entry_count(),
            ctx.scheduler().schedule_preemption_disable_count(),
            ctx.scheduler()
                .schedule_preemption_enable_no_resched_count(),
            ctx.scheduler().scheduler_rcu_context_switch_count(),
            ctx.scheduler().scheduler_rq_lock_mb_after_spinlock_count(),
            ctx.scheduler().scheduler_rq_clock_update_count(),
            ctx.scheduler().scheduler_need_resched_clear_count(),
            ctx.scheduler().scheduler_rq_curr_publish_rcu_count(),
            ctx.scheduler().scheduler_trace_sched_switch_count(),
            ctx.scheduler().scheduler_prepare_task_switch_count(),
            ctx.scheduler().scheduler_finish_task_switch_count(),
            ctx.scheduler().scheduler_finish_released_rq_lock_count(),
            ctx.scheduler()
                .scheduler_finish_preempt_count_restore_count(),
            ctx.scheduler().boot_runqueue_lock().irqsave_entered_count(),
            ctx.scheduler()
                .boot_runqueue_lock()
                .irqrestore_exited_count(),
            boot_idle_setup_state.core_saved_count(),
            boot_idle_setup_state.core_restored_count(),
            ctx.kernel_init_task
                .task()
                .thread_context()
                .core_restored_count(),
            ctx.scheduler().idle_schedule_passes(),
            ctx.scheduler().idle_schedule_returned_passes(),
            ctx.scheduler().idle_schedule_identity_passes(),
            ctx.scheduler().identity_switch_passes(),
            crate::flows::boot_idle_flow::entry_is_online(),
            ctx.scheduler().kernel_init_stack_switch_started_count(),
            ctx.scheduler().kernel_init_stack_switch_returned_count(),
            ctx.boot_cpu_local_interrupt().saved_and_disabled_count(),
            ctx.boot_cpu_local_interrupt().restored_count(),
        ));
        printk::write_fmt(format_args!(
            "scheduler first-switch refs pick_prev_boot={} pick_next_kernel={} current_kernel={} boot_view={} possible={} root_entries={} root_covers={}\n",
            ctx.scheduler().pick_next_task_exit_prev_ref() == TaskRef::BOOT,
            ctx.scheduler().pick_next_task_exit_next_ref() == TaskRef::KERNEL_INIT,
            ctx.current_task_ref()
                .is_ok_and(|task_ref| task_ref == TaskRef::KERNEL_INIT),
            boot_cpu_owned_scheduler_view_matches(),
            scheduler_possible_runqueues_match_cpu_group(),
            default_root_domain_entries_match_cpu_group(),
            ctx.scheduler_shared
                .default_root_domain()
                .covers_cpu_group_possible(&ctx.cpu_group),
        ));
        printk::write_str("scheduler first-switch facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "schedule_passes={} switch_to_passes={} idle_schedule_passes={} boot_cpu={} current_task={}\n",
        ctx.scheduler().schedule_passes(),
        ctx.scheduler().switch_to_passes(),
        ctx.scheduler().idle_schedule_passes(),
        boot_scheduler_view.runqueue().cpu_id(),
        boot_idle_setup_state.task_id()
    ));
    SmokeResult::Passed
}

fn default_root_domain_entries_match_cpu_group() -> bool {
    let ctx = context();
    let root_domain = ctx.scheduler_shared.default_root_domain();
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
    let root_domain = ctx.scheduler_shared.default_root_domain();
    if !ctx.cpu_group.possible_schedulers_ready(root_domain) {
        return false;
    }

    let mut logical_id = 0usize;
    while logical_id < ctx.cpu_group.possible_cpu_count() {
        let Some(cpu) = ctx.cpu_group.cpu(logical_id) else {
            return false;
        };
        let scheduler = cpu.scheduler();
        let runqueue = scheduler;
        let Some(cpu_ref) = ctx.cpu_group.possible_cpu_ref_at(logical_id) else {
            return false;
        };
        if runqueue.runqueue_state() != State::Ready
            || runqueue.cpu_ref() != cpu_ref
            || runqueue.cpu_ref() != cpu.cpu_ref()
            || runqueue.cpu_id() != logical_id
            || runqueue.cpu_hartid() != cpu.hartid()
            || !runqueue.class_queues_ready()
            || !runqueue.attached_to_root_domain()
            || runqueue.balance_push_enabled()
            || !root_domain.covers_cpu_ref(runqueue.cpu_ref())
        {
            return false;
        }
        if logical_id == 0 {
            if scheduler.state() != State::Online
                || runqueue.cpu_ref() != cpu.cpu_ref()
                || runqueue.cpu_hartid() != cpu.hartid()
            {
                return false;
            }
        } else if scheduler.state()
            != if cpu.is_online() {
                State::Online
            } else {
                State::Ready
            }
        {
            return false;
        }
        logical_id += 1;
    }

    ctx.cpu_group
        .possible_scheduler(ctx.cpu_group.possible_cpu_count())
        .is_none()
}

fn boot_cpu_owned_scheduler_view_matches() -> bool {
    let ctx = context();
    if !ctx
        .scheduler()
        .boot_cpu_owned_scheduler_view_ready(&ctx.cpu_group)
    {
        return false;
    }

    let Some(view) = ctx
        .scheduler()
        .boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        return false;
    };
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        return false;
    };
    let runqueue = view.runqueue();
    let idle_task = view.idle_task();
    let matches = view.cpu_ref() == boot_cpu.cpu_ref()
        && view.cpu_id() == boot_cpu.logical_id()
        && view.cpu_hartid() == boot_cpu.hartid()
        && runqueue.cpu_ref() == boot_cpu.cpu_ref()
        && runqueue.cpu_hartid() == boot_cpu.hartid()
        && runqueue.is_boot_backed()
        && idle_task.cpu_ref() == boot_cpu.cpu_ref()
        && idle_task.cpu_id() == boot_cpu.logical_id()
        && ctx
            .current_task_ref()
            .is_ok_and(|task_ref| view.runqueue_current_task_ref().same_identity(task_ref))
        && view.runqueue_current_task_id() != usize::MAX
        && view.runqueue_idle_task_id() == idle_task.task_id()
        && view.runqueue_task_count() == ctx.scheduler().task_count()
        && view.runqueue_idle_task_matches()
        && idle_task.uses_current_init_task()
        && idle_task.lazy_tlb_mm_ready()
        && idle_task.no_set_affinity();
    if !matches {
        printk::write_fmt(format_args!(
            "boot scheduler view snapshot view_cpu={} boot_cpu={} view_hart={} boot_hart={} rq_cpu={} rq_hart={} boot_backed={} idle_cpu={} current_ref={} current_id={} idle_id={} idle_view_id={} count_view={} count_scheduler={} idle_match={} uses_init={} lazy_tlb={} no_affinity={} ready={}\n",
            view.cpu_id(),
            boot_cpu.logical_id(),
            view.cpu_hartid(),
            boot_cpu.hartid(),
            runqueue.cpu_ref().logical_id(),
            runqueue.cpu_hartid(),
            runqueue.is_boot_backed(),
            idle_task.cpu_id(),
            view.runqueue_current_task_ref() == TaskRef::KERNEL_INIT,
            view.runqueue_current_task_id(),
            view.runqueue_idle_task_id(),
            idle_task.task_id(),
            view.runqueue_task_count(),
            ctx.scheduler().task_count(),
            view.runqueue_idle_task_matches(),
            idle_task.uses_current_init_task(),
            idle_task.lazy_tlb_mm_ready(),
            idle_task.no_set_affinity(),
            ctx.scheduler()
                .boot_cpu_owned_scheduler_view_ready(&ctx.cpu_group),
        ));
    }
    matches
}
