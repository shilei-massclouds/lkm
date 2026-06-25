use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{
        printk,
        rest_init::{SystemStateValue, TaskEntry, TaskKind},
        state::State,
    },
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();
    let Some(boot_cpu) = ctx.cpu_group.boot_cpu() else {
        printk::write_str("boot CPU facts missing\n");
        return SmokeResult::Failed;
    };
    let Some(boot_scheduler_view) = ctx.scheduler.boot_cpu_owned_scheduler_view(&ctx.cpu_group)
    else {
        printk::write_str("boot CPU scheduler view missing\n");
        return SmokeResult::Failed;
    };
    let boot_idle_task = boot_scheduler_view.idle_task();

    if !phases::up_multitask::rest_init::is_ready()
        || !phases::up_multitask::rest_init::dispatch_ready()
        || !phases::up_multitask::is_ready()
    {
        printk::write_str("rest init phase is not ready\n");
        return SmokeResult::Failed;
    }

    if !ctx.rcu_core.scheduler_starting_ready()
        || !ctx.rcu_core.scheduler_active_init()
        || !ctx.rcu_core.scheduler_start_single_online_cpu()
        || !ctx.rcu_core.gp_seq_baseline_synced()
        || !ctx.rcu_core.gp_threads_deferred()
    {
        printk::write_str("rcu scheduler start action facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.kernel_init_task.state() != State::Online
        || ctx.kernel_init_task.pid() != 1
        || !ctx.task_creation_core.entry_contract_ready()
        || !ctx.task_creation_core.kernel_init_created()
        || ctx.kernel_init_task.entry() != TaskEntry::KernelInit
        || ctx.kernel_init_task.kind() != TaskKind::UserModeThread
        || !ctx.kernel_init_task.running()
        || !ctx.kernel_init_task.enqueued()
        || ctx.kernel_init_task.cpu_id() != boot_cpu.logical_id()
        || !ctx.kernel_init_task.released_for_pre_smp_init()
        || !boot_scheduler_view.runqueue_contains_task_id(ctx.kernel_init_task.pid())
        || ctx.kernel_init_task_pi_lock.state() != State::Ready
        || ctx.kernel_init_task_pi_lock.locked()
        || ctx.kernel_init_task_pi_lock.irqsave_entered_count() == 0
        || ctx.kernel_init_task_pi_lock.irqrestore_exited_count() == 0
        || !ctx
            .kernel_init_task_pi_lock
            .irqrestore_restored_before_preemption_enabled()
    {
        printk::write_str("kernel_init task facts invalid\n");
        return SmokeResult::Failed;
    }

    if !phases::smp_runtime::runtime_core::is_ready()
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
        || ctx.kthreadd_task.cpu_id() != boot_cpu.logical_id()
        || ctx.scheduler.selected_runqueue_task_id() != ctx.kthreadd_task.pid()
        || !boot_scheduler_view.runqueue_contains_task_id(ctx.kthreadd_task.pid())
        || !ctx.kthreadd_task.global_ref_bound()
        || !ctx.kthreadd_task.provider_ready()
        || !ctx.kthreadd_task.schedule_loop_ready()
        || !ctx.kthreadd_task.schedule_loop_requests_schedule()
        || !ctx.kthreadd_task.schedule_loop_deferred()
        || ctx.kthreadd_task_pi_lock.state() != State::Ready
        || ctx.kthreadd_task_pi_lock.locked()
        || ctx.kthreadd_task_pi_lock.irqsave_entered_count() == 0
        || ctx.kthreadd_task_pi_lock.irqrestore_exited_count() == 0
        || !ctx
            .kthreadd_task_pi_lock
            .irqrestore_restored_before_preemption_enabled()
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
        || !ctx.kthreadd_ready_gate.completed()
        || !ctx.kthreadd_ready_gate.release_committed()
        || ctx.kthreadd_ready_gate.pending()
    {
        printk::write_str("system state or kthreadd gate facts invalid\n");
        return SmokeResult::Failed;
    }
    let kthreadd_completion = ctx.kthreadd_ready_gate.completion();
    if kthreadd_completion.state() != State::Online
        || kthreadd_completion.done_count() != 1
        || !kthreadd_completion.storage_bound()
        || !kthreadd_completion.owns_wait_queue()
        || !kthreadd_completion.handle_published()
        || !kthreadd_completion.complete_committed()
        || !kthreadd_completion.token_available()
        || !kthreadd_completion.wakes_one_waiter()
        || kthreadd_completion.wait_queue().state() != State::Ready
    {
        printk::write_str("kthreadd completion facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.scheduler.schedule_passes() == 0
        || ctx.scheduler.current_runqueue_resolve_passes() == 0
        || ctx.scheduler.pick_next_task_passes() == 0
        || ctx.scheduler.switch_to_passes() == 0
        || ctx.scheduler.identity_switch_passes() != 0
        || ctx.boot_cpu_current_task.switch_committed_count() == 0
        || !ctx.boot_cpu_current_task.current_is_kernel_init()
        || !boot_idle_task.thread_context_core_register_set()
        || boot_idle_task.thread_context_core_saved_count() == 0
        || boot_idle_task.thread_context_core_restored_count() == 0
        || !ctx.kernel_init_task.released_for_pre_smp_init()
    {
        printk::write_str("scheduler dispatch facts invalid\n");
        return SmokeResult::Failed;
    }

    let idle_schedule_passes = ctx.scheduler.idle_schedule_passes();
    if idle_schedule_passes != 1
        || ctx.scheduler.idle_schedule_returned_passes() != idle_schedule_passes
        || ctx.scheduler.idle_schedule_identity_passes() != 0
        || ctx.scheduler.schedule_passes() < idle_schedule_passes
        || ctx.scheduler.switch_to_passes() < idle_schedule_passes
        || ctx.boot_cpu_current_task.switch_committed_count() < idle_schedule_passes
        || ctx.scheduler.identity_switch_passes() != 0
    {
        printk::write_str("idle schedule relation facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.boot_idle_runtime.state() != State::Ready
        || !ctx.boot_idle_runtime.first_schedule_committed()
        || !ctx.boot_idle_runtime.idle_entry_prepared()
        || !ctx.boot_idle_runtime.cpu_startup_entry_ready()
        || !ctx.boot_idle_runtime.idle_loop_entered()
        || !ctx.boot_idle_runtime.idle_cycle_committed()
        || !ctx.boot_idle_runtime.idle_cycle_started()
        || !ctx.boot_idle_runtime.need_resched_clear_before_wait()
        || !ctx.boot_idle_runtime.observed_no_need_resched()
        || !ctx.boot_idle_runtime.idle_polling_set()
        || !ctx.boot_idle_runtime.nohz_idle_entered()
        || !ctx.boot_idle_runtime.idle_wait_committed()
        || !ctx.boot_idle_runtime.idle_wait_path_deferred()
        || !ctx.boot_idle_runtime.need_resched_set_for_schedule()
        || !ctx.boot_idle_runtime.observed_need_resched()
        || !ctx.boot_idle_runtime.idle_polling_cleared()
        || !ctx.boot_idle_runtime.nohz_idle_exited()
        || !ctx.boot_idle_runtime.idle_schedule_requested()
        || !ctx.boot_idle_runtime.idle_schedule_returned()
        || !ctx.boot_idle_runtime.need_resched_drained()
        || !ctx.boot_idle_runtime.idle_loop_continues()
        || !ctx
            .boot_idle_runtime
            .representative_need_resched_cycle_committed()
        || !ctx.boot_idle_runtime.boot_init_handoff_complete()
        || !ctx.boot_idle_runtime.secondary_cpus_not_started()
        || !ctx.boot_idle_runtime.real_task_switch_deferred()
        || ctx.workqueue.workers_running()
    {
        printk::write_str("boot idle runtime facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "rest_init init_pid={} kthreadd_pid={} schedule_passes={} switch_to_passes={} idle_schedule_passes={}\n",
        ctx.kernel_init_task.pid(),
        ctx.kthreadd_task.pid(),
        ctx.scheduler.schedule_passes(),
        ctx.scheduler.switch_to_passes(),
        idle_schedule_passes
    ));
    SmokeResult::Passed
}
