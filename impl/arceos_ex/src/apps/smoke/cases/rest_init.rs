use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        printk,
        rest_init::SystemStateValue,
        state::State,
        task::{TaskEntry, TaskKind, TaskRef},
        task_flow::TaskFlowRef,
    },
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();
    if ctx
        .kernel_init_flow
        .start_initial(&ctx.kernel_init_task)
        .is_ok()
        || ctx
            .kthreadd_flow
            .start_initial(&mut ctx.kthreadd_task)
            .is_ok()
        || ctx.kernel_init_flow.state() != State::Online
        || !ctx.kernel_init_flow.active()
        || ctx.kthreadd_flow.state() != State::Base
        || ctx.kthreadd_flow.active()
    {
        printk::write_str("task flow strict dispatch facts invalid\n");
        return SmokeResult::Failed;
    }
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        printk::write_str("boot CPU facts missing\n");
        return SmokeResult::Failed;
    };
    let Some(boot_scheduler_view) = ctx.scheduler.boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        printk::write_str("boot CPU scheduler view missing\n");
        return SmokeResult::Failed;
    };
    let boot_idle_setup_state = boot_scheduler_view.idle_task();

    if ctx.boot_task.task_ref() != TaskRef::BOOT
        || ctx.boot_task.pid() != 0
        || ctx.boot_task.carrier_address()
            != core::ptr::addr_of!(crate::objects::boot_task::init_task_storage) as usize
        || !ctx.boot_task.idle_role_bound()
        || ctx.boot_init_flow.cpu_ref() != Some(boot_cpu.cpu_ref())
        || !ctx.boot_task.switch_context().initialized()
        || ctx.boot_idle_flow.state() != State::Ready
        || !ctx.boot_idle_flow.active()
        || ctx.boot_idle_flow.owner() != TaskRef::BOOT
        || ctx.boot_idle_flow.flow_ref() != TaskFlowRef::BOOT_IDLE
        || ctx.boot_task.task().initial_flow() != TaskFlowRef::BOOT_INIT
        || ctx.boot_task.task().active_flow() != TaskFlowRef::BOOT_IDLE
    {
        printk::write_str("boot task/flow carrier facts invalid\n");
        return SmokeResult::Failed;
    }

    if !crate::flows::boot_init_flow::rest_init_is_online()
        || !crate::flows::boot_init_flow::schedule_handoff_is_online()
        || crate::flows::boot_idle_flow::entry_is_online()
        || !crate::flows::boot_init_flow::is_online()
    {
        printk::write_str("rest init phase is not online\n");
        return SmokeResult::Failed;
    }

    if !ctx.rcu_core.scheduler_starting_ready()
        || !ctx.rcu_core.scheduler_active_init()
        || !ctx.rcu_core.scheduler_start_single_online_cpu()
        || !ctx.rcu_core.scheduler_start_local_irq_guarded()
        || ctx.rcu_core.scheduler_start_local_irq_save_count() == 0
        || ctx.rcu_core.scheduler_start_local_irq_restore_count() == 0
        || !ctx.rcu_core.gp_seq_baseline_synced()
        || !ctx.rcu_core.gp_threads_deferred()
    {
        printk::write_str("rcu scheduler start action facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.kernel_init_task.state() != State::OnCpu
        || ctx.kernel_init_task.pid() != 1
        || !ctx.task_creation_core.entry_contract_ready()
        || !ctx.task_creation_core.kernel_init_created()
        || ctx.kernel_init_task.entry() != TaskEntry::KernelInit
        || ctx.kernel_init_task.kind() != TaskKind::UserModeThread
        || !ctx.kernel_init_task.running()
        || !ctx.kernel_init_task.enqueued()
        || ctx.kernel_init_flow.cpu_id() != boot_cpu.logical_id()
        || !ctx.kernel_init_task.pid_lookup_under_rcu_read()
        || !ctx.kernel_init_task.pid_lookup_rcu_guard_balanced()
        || ctx.kernel_init_task.waiting_for_kthreadd_done()
        || !ctx.kernel_init_task.observed_kthreadd_done_release()
        || !ctx.kernel_init_task.released_for_pre_smp_init()
        || !boot_scheduler_view.runqueue_contains_task_id(ctx.kernel_init_task.pid())
        || ctx.kernel_init_task_pi_lock.state() != State::Ready
        || ctx.kernel_init_task_pi_lock.locked()
        || ctx.kernel_init_task_pi_lock.irqsave_entered_count() == 0
        || ctx.kernel_init_task_pi_lock.irqrestore_exited_count() == 0
        || !ctx
            .kernel_init_task_pi_lock
            .irqrestore_restored_before_preemption_enabled()
        || ctx.kernel_init_task.task_ref() != TaskRef::KERNEL_INIT
        || ctx.kernel_init_flow.state() != State::Online
        || !ctx.kernel_init_flow.active()
        || ctx.kernel_init_flow.flow_ref() != TaskFlowRef::KERNEL_INIT
        || ctx.kernel_init_task.task().active_flow() != TaskFlowRef::KERNEL_INIT
    {
        printk::write_str("kernel_init task facts invalid\n");
        return SmokeResult::Failed;
    }

    if !phases::smp_runtime::runtime_core::is_online()
        && (!ctx.kernel_init_task.pinned_to_boot_cpu() || !ctx.kernel_init_task.pf_no_setaffinity())
    {
        printk::write_str("kernel_init pre-runtime affinity facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.kthreadd_task.state() != State::Online
        || ctx.kthreadd_task.pid() != 2
        || !ctx.task_creation_core.kthreadd_created()
        || ctx.kthreadd_task.entry() != TaskEntry::Kthreadd
        || ctx.kthreadd_task.kind() != TaskKind::KernelThread
        || !ctx.kthreadd_task.clone_fs()
        || !ctx.kthreadd_task.clone_files()
        || !ctx.kthreadd_task.clone_vm()
        || !ctx.kthreadd_task.clone_untraced()
        || !ctx.kthreadd_task.kernel_thread_flag()
        || !ctx.kthreadd_task.running()
        || !ctx.kthreadd_task.enqueued()
        || ctx.kthreadd_flow.cpu_id() != boot_cpu.logical_id()
        || ctx.scheduler.selected_runqueue_task_id() != ctx.kthreadd_task.pid()
        || !boot_scheduler_view.runqueue_contains_task_id(ctx.kthreadd_task.pid())
        || !ctx.kthreadd_task.global_ref_bound()
        || !ctx.kthreadd_task.provider_ready()
        || !ctx.kthreadd_task.pid_lookup_under_rcu_read()
        || !ctx.kthreadd_task.pid_lookup_rcu_guard_balanced()
        || ctx.kthreadd_task.schedule_loop_active()
        || ctx.kthreadd_task_pi_lock.state() != State::Ready
        || ctx.kthreadd_task_pi_lock.locked()
        || ctx.kthreadd_task_pi_lock.irqsave_entered_count() == 0
        || ctx.kthreadd_task_pi_lock.irqrestore_exited_count() == 0
        || !ctx
            .kthreadd_task_pi_lock
            .irqrestore_restored_before_preemption_enabled()
        || ctx.kthreadd_task.task_ref() != TaskRef::KTHREADD
        || ctx.kthreadd_flow.state() != State::Base
        || ctx.kthreadd_flow.active()
        || ctx.kthreadd_flow.owner() != TaskRef::KTHREADD
        || ctx.kthreadd_flow.flow_ref() != TaskFlowRef::KTHREADD
        || ctx.kthreadd_task.task().initial_flow() != TaskFlowRef::KTHREADD
        || ctx.kthreadd_task.task().active_flow().is_valid()
    {
        printk::write_str("kthreadd task facts invalid\n");
        return SmokeResult::Failed;
    }

    let system_state_valid = (ctx.system_state.state() == State::Ready
        && ctx.system_state.value() == SystemStateValue::Scheduling)
        || (ctx.system_state.state() == State::Online
            && ctx.system_state.value() == SystemStateValue::Running);

    if !system_state_valid
        || ctx.kthreadd_ready_gate.state() != State::Online
        || !ctx.kthreadd_ready_gate.completion().complete_committed()
        || !ctx.kthreadd_ready_gate.complete_wait_lock_guard_used()
        || ctx.kthreadd_ready_gate.complete_wait_lock_irqsave_count() == 0
        || ctx
            .kthreadd_ready_gate
            .complete_wait_lock_irqrestore_count()
            == 0
        || !ctx.kthreadd_ready_gate.complete_done_increment_guarded()
        || !ctx.kthreadd_ready_gate.complete_wake_guarded()
        || ctx.kthreadd_ready_gate_wait_lock.state() != State::Ready
        || ctx.kthreadd_ready_gate_wait_lock.locked()
        || ctx.kthreadd_ready_gate_wait_lock.irqsave_entered_count() == 0
        || ctx.kthreadd_ready_gate_wait_lock.irqrestore_exited_count() == 0
        || !ctx
            .kthreadd_ready_gate_wait_lock
            .irqrestore_restored_before_preemption_enabled()
        || !ctx.kthreadd_ready_gate.pending()
    {
        printk::write_str("system state or kthreadd gate facts invalid\n");
        return SmokeResult::Failed;
    }
    let kthreadd_completion = ctx.kthreadd_ready_gate.completion();
    if kthreadd_completion.state() != State::Online
        || kthreadd_completion.done_count() != 0
        || !kthreadd_completion.storage_bound()
        || !kthreadd_completion.owns_wait_queue()
        || !kthreadd_completion.handle_published()
        || !kthreadd_completion.complete_committed()
        || kthreadd_completion.token_available()
        || !kthreadd_completion.wakes_one_waiter()
        || !kthreadd_completion.waiter_enqueued()
        || !kthreadd_completion.waiter_finished()
        || kthreadd_completion.wait_queue().state() != State::Ready
        || !kthreadd_completion.wait_queue().waiter_enqueued()
        || !kthreadd_completion.wait_queue().waiter_finished()
    {
        printk::write_str("kthreadd completion facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.scheduler.schedule_passes() == 0
        || ctx.scheduler.current_runqueue_resolve_passes() == 0
        || ctx.scheduler.pick_next_task_passes() == 0
        || ctx.scheduler.switch_to_passes() == 0
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
        || ctx.scheduler.identity_switch_passes() == 0
        || ctx.scheduler.boot_idle_preemption().state() != State::Ready
        || ctx.boot_cpu_current_task().switch_committed_count() == 0
        || !ctx.boot_cpu_current_task().current_is_kernel_init()
        || !boot_idle_setup_state.switch_ctx_initialized()
        || boot_idle_setup_state.core_saved_count() == 0
        || boot_idle_setup_state.core_restored_count() != 0
        || ctx
            .kernel_init_task
            .task()
            .thread_context()
            .core_restored_count()
            == 0
        || !ctx.kernel_init_task.released_for_pre_smp_init()
    {
        printk::write_str("scheduler dispatch facts invalid\n");
        return SmokeResult::Failed;
    }

    let idle_schedule_passes = ctx.scheduler.idle_schedule_passes();
    if idle_schedule_passes != 0
        || ctx.scheduler.idle_schedule_returned_passes() != 0
        || ctx.scheduler.idle_schedule_identity_passes() != 0
        || ctx.scheduler.identity_switch_passes() == 0
    {
        printk::write_str("idle schedule relation facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.boot_idle_flow.state() != State::Ready
        || ctx.boot_idle_flow.first_schedule_committed()
        || ctx.boot_idle_flow.idle_entry_prepared()
        || ctx.boot_idle_flow.cpu_startup_entry_ready()
        || ctx.boot_idle_flow.idle_loop_entered()
        || ctx.boot_idle_flow.idle_cycle_committed()
        || ctx.boot_idle_flow.idle_cycle_started()
        || ctx.boot_idle_flow.need_resched_clear_before_wait()
        || ctx.boot_idle_flow.observed_no_need_resched()
        || ctx.boot_idle_flow.idle_polling_set()
        || ctx.boot_idle_flow.idle_polling_rmb_before_sleep_check()
        || ctx.boot_idle_flow.nohz_idle_entered()
        || ctx.boot_idle_flow.local_irq_save_count_for_sleep() != 0
        || ctx.boot_idle_flow.local_irq_restore_count_for_sleep() != 0
        || ctx.boot_idle_flow.rcu_nocb_deferred_wakeup_flushed()
        || ctx.boot_idle_flow.cpu_offline_dead_path_not_taken()
        || ctx.boot_idle_flow.poll_or_cpuidle_path_deferred()
        || ctx.boot_idle_flow.idle_wait_committed()
        || ctx.boot_idle_flow.idle_wait_path_deferred()
        || ctx.boot_idle_flow.need_resched_set_for_schedule()
        || ctx.boot_idle_flow.observed_need_resched()
        || ctx.boot_idle_flow.idle_polling_cleared()
        || ctx.boot_idle_flow.preempt_need_resched_set()
        || ctx.boot_idle_flow.nohz_idle_exited()
        || ctx.boot_idle_flow.polling_clear_mb_before_flush()
        || ctx.boot_idle_flow.idle_schedule_requested()
        || ctx.boot_idle_flow.idle_schedule_returned()
        || ctx.boot_idle_flow.need_resched_drained()
        || ctx.boot_idle_flow.livepatch_state_update_deferred()
        || ctx.boot_idle_flow.idle_loop_continues()
        || ctx
            .boot_idle_flow
            .representative_need_resched_cycle_committed()
        || ctx.boot_idle_flow.boot_init_handoff_complete()
        || !ctx.boot_idle_flow.secondary_cpus_not_started()
        || !ctx.boot_idle_flow.kernel_init_task_switch_handoff_ready()
        || ctx.scheduler.kernel_init_stack_switch_started_count() != 1
        || ctx.scheduler.kernel_init_stack_switch_returned_count() != 0
        || ctx.kernel_init_task.entry_started_count() != 1
        || !ctx.kernel_init_task.entry_stack_verified()
        || !ctx
            .kernel_init_task
            .stack_pointer_in_range(ctx.kernel_init_task.entry_stack_pointer())
        || !ctx.kernel_init_task.current_stack_pointer_in_range()
        || !ctx.boot_cpu_current_task().current_is_kernel_init()
        || ctx.workqueue.workers_running()
    {
        printk::write_str("boot idle runtime facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "rest_init init_pid={} kthreadd_pid={} kernel_init_sp={:#x} schedule_passes={} switch_to_passes={} boot_idle_entry={}\n",
        ctx.kernel_init_task.pid(),
        ctx.kthreadd_task.pid(),
        ctx.kernel_init_task.entry_stack_pointer(),
        ctx.scheduler.schedule_passes(),
        ctx.scheduler.switch_to_passes(),
        crate::flows::boot_idle_flow::entry_is_online()
    ));
    SmokeResult::Passed
}
