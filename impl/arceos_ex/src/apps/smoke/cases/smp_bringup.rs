use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, state::State},
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::smp_runtime::smp_bringup::is_online()
        || !ctx.kernel_init_flow.payload_handoff_committed()
    {
        printk::write_str("smp bringup phase is not ready\n");
        return SmokeResult::Failed;
    }

    if ctx.secondary_idle_tasks.state() != State::Prepared
        || ctx.secondary_idle_tasks.prepared_count() != ctx.cpu_group.secondary_count()
        || !ctx.secondary_idle_tasks.inactive()
        || !ctx.secondary_idle_tasks.per_secondary_idle_task()
        || !ctx.secondary_idle_tasks.dedicated_stack()
        || !ctx.secondary_idle_tasks.pt_regs_stack_pointer()
        || !ctx.secondary_idle_tasks.unified_task_flow_carriers()
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
        || !ctx.cpu_hotplug_sync.cpus_read_guard_used()
        || !ctx.cpu_hotplug_sync.smpboot_threads_mutex_guard_used()
        || !ctx.cpu_hotplug_sync.cpu_running_wait_lock_guard_used()
        || !ctx.cpu_hotplug_sync.done_up_wait_lock_guard_used()
    {
        printk::write_str("cpu hotplug sync facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.smpboot_threads_lock.lock_entered_count() == 0
        || ctx.smpboot_threads_lock.unlock_exited_count()
            != ctx.smpboot_threads_lock.lock_entered_count()
        || ctx.cpu_add_remove_lock.lock_entered_count() == 0
        || ctx.cpu_add_remove_lock.unlock_exited_count()
            != ctx.cpu_add_remove_lock.lock_entered_count()
        || ctx.cpu_hotplug_lock.read_lock_count() == 0
        || ctx.cpu_hotplug_lock.read_unlock_count() != ctx.cpu_hotplug_lock.read_lock_count()
        || ctx.cpu_hotplug_lock.write_lock_count() == 0
        || ctx.cpu_hotplug_lock.write_unlock_count() != ctx.cpu_hotplug_lock.write_lock_count()
        || ctx.cpu_running_wait_lock.irqsave_entered_count() == 0
        || ctx.cpu_running_wait_lock.irqrestore_exited_count()
            != ctx.cpu_running_wait_lock.irqsave_entered_count()
        || ctx.done_up_wait_lock.irqsave_entered_count() == 0
        || ctx.done_up_wait_lock.irqrestore_exited_count()
            != ctx.done_up_wait_lock.irqsave_entered_count()
    {
        printk::write_str("smp bringup guard counters invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.cpu_start_provider.state() != State::Ready
        || !ctx.cpu_start_provider.start_requests_issued()
        || !ctx.cpu_start_provider.secondary_start_sbi_selected()
        || !ctx.cpu_start_provider.boot_data_selected()
        || !ctx.cpu_start_provider.hsm_start_requests_issued()
        || !ctx.cpu_start_provider.hsm_start_return_observed()
        || !ctx.cpu_start_provider.boot_data_per_secondary_cpu()
        || !ctx.cpu_start_provider.boot_data_task_ptr_is_idle_task()
        || !ctx
            .cpu_start_provider
            .boot_data_stack_ptr_is_pt_regs_stack()
        || !ctx.cpu_start_provider.cpu_add_remove_mutex_guard_used()
        || !ctx.cpu_start_provider.cpu_hotplug_write_guard_used()
        || !ctx
            .cpu_start_provider
            .sbi_boot_data_publish_barriers_observed()
        || !phases::smp_runtime::ap_entry_prelude::all_online(&ctx.cpu_group)
        || !phases::smp_runtime::ap_entry_prelude::all_adoption_facts(&ctx.cpu_group)
        || !phases::smp_runtime::ap_entry_prelude::all_boot_data_verified(&ctx.cpu_group)
        || !phases::smp_runtime::ap_entry_prelude::all_stacks_verified(&ctx.cpu_group)
        || !phases::smp_runtime::ap_entry_prelude::all_task_pointers_verified(&ctx.cpu_group)
        || !phases::smp_runtime::ap_smp_callin::all_online(&ctx.cpu_group)
        || !phases::smp_runtime::ap_smp_callin::all_callin_facts(&ctx.cpu_group)
        || !phases::smp_runtime::ap_online_idle::all_online(&ctx.cpu_group)
        || !phases::smp_runtime::ap_online_idle::all_online_idle_facts(&ctx.cpu_group)
        || !phases::smp_runtime::ap_online_idle::all_park_loops_entered(&ctx.cpu_group)
        || ctx.secondary_cpu_startup_ack.state() != State::Ready
        || !ctx.secondary_cpu_startup_ack.acknowledged()
        || !ctx
            .secondary_cpu_startup_ack
            .ap_smp_callin_ack_matches_secondary_cpu()
        || ctx.secondary_cpu_online_ack.state() != State::Ready
        || !ctx.secondary_cpu_online_ack.acknowledged()
        || !ctx.secondary_cpu_online_ack.online_after_ap_ack()
        || !ctx.secondary_cpu_online_ack.ap_idle_or_park_loop_entered()
        || !ctx.secondary_cpu_online_ack.ap_local_irq_enable_observed()
        || !ctx
            .secondary_cpu_online_ack
            .ap_cache_tlb_flush_summary_observed()
        || !ctx.secondary_cpu_online_ack.ap_ipi_enable_observed()
        || !ctx
            .secondary_cpu_online_ack
            .ap_hotplug_thread_mb_pair_deferred()
    {
        printk::write_str("secondary cpu ack facts invalid\n");
        return SmokeResult::Failed;
    }

    let mut logical_id = 1usize;
    while logical_id <= ctx.cpu_group.secondary_count() {
        if phases::smp_runtime::ap_entry_prelude::state_for(logical_id) != State::Online
            || phases::smp_runtime::ap_smp_callin::state_for(logical_id) != State::Online
            || phases::smp_runtime::ap_online_idle::state_for(logical_id) != State::Online
        {
            printk::write_str("per-AP phase state invalid\n");
            return SmokeResult::Failed;
        }
        logical_id += 1;
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
        "smp_bringup online_cpus={} sync=cpu_running+done_up guards=hotplug+completion\n",
        ctx.cpu_group.possible_cpu_count()
    ));
    SmokeResult::Passed
}
