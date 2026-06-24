use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::smp_runtime::is_ready() || !phases::smp_runtime::smp_bringup::is_ready() {
        printk::write_str("smp bringup phase is not ready\n");
        return SmokeResult::Failed;
    }

    if ctx.secondary_idle_tasks.state() != State::Prepared
        || ctx.secondary_idle_tasks.prepared_count() != ctx.cpu_group.secondary_count()
        || !ctx.secondary_idle_tasks.inactive()
    {
        printk::write_str("secondary idle task facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.cpu_hotplug_sync.state() != State::Prepared
        || !ctx.cpu_hotplug_sync.cpu_running_ready()
        || !ctx.cpu_hotplug_sync.cpu_running_observed()
        || !ctx.cpu_hotplug_sync.done_up_ready()
        || !ctx.cpu_hotplug_sync.done_up_observed()
        || !ctx.cpu_hotplug_sync.done_down_ready()
        || !ctx.cpu_hotplug_sync.boot_cpu_hotplug_thread_online()
        || !ctx.cpu_hotplug_sync.secondary_hotplug_threads_deferred()
    {
        printk::write_str("cpu hotplug sync facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.cpu_start_provider.state() != State::Ready
        || !ctx.cpu_start_provider.start_requests_issued()
        || !ctx.cpu_start_provider.ap_entry_detail_deferred()
        || ctx.secondary_cpu_startup_ack.state() != State::Ready
        || !ctx.secondary_cpu_startup_ack.acknowledged()
        || !ctx.secondary_cpu_startup_ack.ap_entry_detail_deferred()
        || ctx.secondary_cpu_online_ack.state() != State::Ready
        || !ctx.secondary_cpu_online_ack.acknowledged()
        || !ctx.secondary_cpu_online_ack.ap_idle_detail_deferred()
    {
        printk::write_str("secondary cpu ack facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.cpu_group.state() != State::Ready
        || !ctx.cpu_group.secondary_cpus_online()
        || !ctx.cpu_group.smp_concurrency_open()
        || !ctx.secondary_cpus.all_match_cpu_group_views(&ctx.cpu_group)
        || ctx.smp_bringup_boundary.state() != State::Ready
        || !ctx.smp_bringup_boundary.smp_cpus_done_trimmed()
        || !ctx.smp_bringup_boundary.ap_hotplug_callbacks_deferred()
    {
        printk::write_str("smp bringup boundary facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "smp_bringup online_cpus={} sync=cpu_running+done_up\n",
        ctx.cpu_group.possible_cpu_count()
    ));
    SmokeResult::Passed
}
