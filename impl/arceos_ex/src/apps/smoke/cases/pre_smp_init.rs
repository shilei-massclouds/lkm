use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::up_multitask::pre_smp_init::is_ready()
        || !phases::up_multitask::rest_init::dispatch_ready()
        || !phases::up_multitask::is_ready()
    {
        printk::write_str("pre-smp init phase is not ready\n");
        return SmokeResult::Failed;
    }

    if ctx.kernel_init_dispatch_gate.state() != State::Ready
        || !ctx.kernel_init_dispatch_gate.boot_idle_tail_pending()
        || ctx.boot_idle_runtime.state() != State::Ready
    {
        printk::write_str("pre-smp fork boundary invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.page_allocator.state() != State::Ready || !ctx.page_allocator.full_gfp_mask_open() {
        printk::write_str("page allocator full GFP mask not open\n");
        return SmokeResult::Failed;
    }

    if ctx.cpu_group.state() != State::Ready
        || !ctx.cpu_group.pre_smp_topology_ready()
        || !ctx.cpu_group.boot_cpu_topology_recorded()
        || !ctx.pre_smp_boundary.secondary_cpus_present_not_online()
    {
        printk::write_str("pre-smp cpu topology facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.workqueue.state() != State::Ready
        || !ctx.workqueue.worker_creation_open()
        || !ctx.workqueue.rescuers_ready()
        || !ctx.workqueue.initial_workers_created()
        || !ctx.workqueue.watchdog_ready()
        || ctx.workqueue.workers_running()
    {
        printk::write_str("workqueue pre-smp facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.vmstat_core.state() != State::Prepared
        || !ctx.vmstat_core.mm_percpu_workqueue_ready()
        || !ctx.vmstat_core.cpuhp_state_registered()
        || !ctx.vmstat_core.shepherd_work_started()
        || !ctx.vmstat_core.proc_exports_deferred()
    {
        printk::write_str("vmstat core facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.rcu_core.tasks_rcu().state() != State::Ready
        || !ctx.rcu_core.tasks_rcu().gp_threads_ready()
        || ctx.pre_smp_initcalls.state() != State::Ready
        || !ctx.pre_smp_initcalls.early_level_ran()
        || !ctx.pre_smp_initcalls.rcu_gp_kthread_ready()
        || !ctx.pre_smp_initcalls.softirq_ksoftirqd_ready()
        || !ctx.pre_smp_initcalls.scheduler_migration_ready()
        || !ctx.pre_smp_initcalls.cpu_stopper_prepared()
        || !ctx.pre_smp_initcalls.zero_page_bound()
        || !ctx.pre_smp_initcalls.address_space_id_ready()
    {
        printk::write_str("tasks RCU or pre-smp initcall facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.pre_smp_boundary.state() != State::Ready
        || !ctx.pre_smp_boundary.mems_allowed_trimmed()
        || !ctx.pre_smp_boundary.cad_pid_deferred()
        || !ctx.pre_smp_boundary.lockup_detector_deferred()
        || !ctx.pre_smp_boundary.smp_init_not_called()
        || !ctx.pre_smp_boundary.secondary_cpus_present_not_online()
    {
        printk::write_str("pre-smp boundary facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "pre_smp_init possible_cpus={} full_gfp={} early_initcalls={}\n",
        ctx.cpu_group.possible_cpu_count(),
        ctx.page_allocator.full_gfp_mask_open(),
        ctx.pre_smp_initcalls.early_level_ran()
    ));
    SmokeResult::Passed
}
