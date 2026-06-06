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
        || ctx.kernel_init_task.entry() != TaskEntry::KernelInit
        || ctx.kernel_init_task.kind() != TaskKind::UserModeThread
        || !ctx.kernel_init_task.running()
        || !ctx.kernel_init_task.enqueued()
        || !ctx.kernel_init_task.released_for_pre_smp_init()
        || ctx.scheduler.selected_runqueue_task_id() != ctx.kernel_init_task.pid()
        || ctx.scheduler.boot_runqueue().enqueued_task_id() != ctx.kernel_init_task.pid()
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
        || ctx.kthreadd_task.entry() != TaskEntry::Kthreadd
        || ctx.kthreadd_task.kind() != TaskKind::KernelThread
        || !ctx.kthreadd_task.clone_fs()
        || !ctx.kthreadd_task.clone_files()
        || !ctx.kthreadd_task.global_ref_bound()
        || !ctx.kthreadd_task.enqueued()
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

    if ctx.kernel_init_dispatch_gate.state() != State::Ready
        || !ctx.kernel_init_dispatch_gate.schedule_committed()
        || !ctx.kernel_init_dispatch_gate.kernel_init_dispatched()
        || !ctx.kernel_init_dispatch_gate.boot_idle_tail_pending()
        || ctx.scheduler.preempt_disabled_passes() == 0
    {
        printk::write_str("kernel init dispatch gate facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.boot_idle_runtime.state() != State::Ready
        || !ctx.boot_idle_runtime.first_schedule_committed()
        || !ctx.boot_idle_runtime.cpu_startup_entry_ready()
        || !ctx.boot_idle_runtime.boot_init_handoff_complete()
        || !ctx.boot_idle_runtime.secondary_cpus_not_started()
        || !ctx.boot_idle_runtime.real_task_switch_deferred()
        || ctx.workqueue.workers_running()
    {
        printk::write_str("boot idle runtime facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "rest_init init_pid={} kthreadd_pid={} schedule_passes={}\n",
        ctx.kernel_init_task.pid(),
        ctx.kthreadd_task.pid(),
        ctx.scheduler.preempt_disabled_passes()
    ));
    SmokeResult::Passed
}
