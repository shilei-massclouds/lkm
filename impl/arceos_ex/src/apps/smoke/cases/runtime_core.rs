use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::smp_runtime::runtime_core::is_ready() || !phases::smp_runtime::is_ready() {
        printk::write_str("runtime core phase is not ready\n");
        return SmokeResult::Failed;
    }

    if ctx.scheduler.state() != State::Online
        || !ctx.scheduler.smp_initialized()
        || !ctx.scheduler.sched_domains_ready()
        || !ctx.scheduler.kernel_init_affinity_released()
        || !ctx.scheduler.rt_dl_smp_ready()
        || !ctx.scheduler.granularity_refreshed()
    {
        printk::write_str("scheduler SMP runtime facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.kernel_init_task.pinned_to_boot_cpu()
        || ctx.kernel_init_task.pf_no_setaffinity()
        || ctx.kernel_init_task.cpu_id() != usize::MAX
    {
        printk::write_str("kernel_init affinity release facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.workqueue.state() != State::Ready
        || !ctx.workqueue.topology_ready()
        || !ctx.workqueue.pod_types_ready()
        || !ctx.workqueue.unbound_pools_rebound()
        || !ctx.workqueue.max_active_topology_ready()
        || ctx.workqueue.smp_topology_deferred()
        || ctx.workqueue.workers_running()
    {
        printk::write_str("workqueue topology facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.async_core_deferred.state() != State::Ready
        || !ctx.async_core_deferred.setup_deferred()
        || !ctx.async_core_deferred.workqueue_creation_deferred()
        || ctx.padata_core_deferred.state() != State::Ready
        || !ctx.padata_core_deferred.setup_deferred()
        || !ctx.padata_core_deferred.hotplug_steps_deferred()
        || !ctx.padata_core_deferred.work_array_deferred()
    {
        printk::write_str("runtime core deferred facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.page_allocator.state() != State::Ready
        || !ctx.page_allocator.late_ready()
        || !ctx.page_allocator.memory_stats_ready()
        || !ctx.page_allocator.buffer_init_ready()
        || !ctx.page_allocator.memblock_private_discarded()
        || !ctx.page_allocator.zone_contiguous_ready()
        || !ctx.page_allocator.sysctl_ready()
        || !ctx.page_allocator.deferred_struct_page_init_trimmed()
        || !ctx.page_allocator.page_extension_late_trimmed()
        || !ctx.page_allocator.shuffle_late_trimmed()
    {
        printk::write_str("page allocator late facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.runtime_core_boundary.state() != State::Ready
        || !ctx.runtime_core_boundary.do_basic_setup_next_boundary()
    {
        printk::write_str("runtime core boundary facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "runtime_core sched_smp={} workqueue_topology={} page_late={}\n",
        ctx.scheduler.smp_initialized(),
        ctx.workqueue.topology_ready(),
        ctx.page_allocator.late_ready()
    ));
    SmokeResult::Passed
}
