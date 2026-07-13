use crate::{
    apps::smoke::SmokeResult,
    context::context,
    objects::{printk, rest_init::SystemStateValue, state::State},
    phases,
};

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::smp_runtime::finalize::is_online() || !phases::smp_runtime::is_online() {
        printk::write_str("finalize phase is not ready\n");
        return SmokeResult::Failed;
    }

    if ctx.async_full_sync_deferred.state() != State::Ready
        || !ctx.async_full_sync_deferred.synchronize_full_deferred()
        || !ctx
            .async_full_sync_deferred
            .init_work_drain_boundary_preserved()
        || !ctx.async_full_sync_deferred.waitqueue_deferred()
        || !ctx.async_full_sync_deferred.async_lock_irqsave_deferred()
        || !ctx.async_full_sync_deferred.entry_count_atomic_deferred()
        || !ctx
            .async_full_sync_deferred
            .global_cookie_boundary_preserved()
    {
        printk::write_str("async full sync deferred facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.init_memory_cleanup_deferred.state() != State::Ready
        || !ctx
            .init_memory_cleanup_deferred
            .system_state_freeing_window_entered()
        || !ctx.init_memory_cleanup_deferred.kprobe_trimmed_noop()
        || !ctx.init_memory_cleanup_deferred.ftrace_cleanup_deferred()
        || !ctx.init_memory_cleanup_deferred.kgdb_trimmed_noop()
        || !ctx
            .init_memory_cleanup_deferred
            .bootconfig_exit_trimmed_noop()
        || !ctx.init_memory_cleanup_deferred.free_deferred()
    {
        printk::write_str("init memory cleanup facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.kernel_mapping_protection_deferred.state() != State::Ready
        || !ctx.kernel_mapping_protection_deferred.enable_deferred()
        || !ctx
            .kernel_mapping_protection_deferred
            .strict_kernel_rwx_position_preserved()
        || !ctx
            .kernel_mapping_protection_deferred
            .rodata_debug_test_trimmed_or_deferred()
    {
        printk::write_str("kernel mapping protection facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.pti_finalize_trimmed.state() != State::Ready || !ctx.pti_finalize_trimmed.trimmed_noop()
    {
        printk::write_str("pti finalize facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.system_state.state() != State::Online
        || ctx.system_state.value() != SystemStateValue::Running
    {
        printk::write_str("system state running facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.numa_default_policy_trimmed.state() != State::Ready
        || !ctx.numa_default_policy_trimmed.trimmed_noop()
        || !ctx.numa_default_policy_trimmed.config_numa_disabled()
    {
        printk::write_str("numa default policy facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.rcu_boot_end.state() != State::Ready
        || !ctx.rcu_boot_end.rcu_boot_ended()
        || !ctx.rcu_core.inkernel_boot_ended()
        || !ctx.rcu_core.unexpedite_gp_atomic_decrement_recorded()
        || !ctx.rcu_core.async_relax_config_lazy_trimmed()
        || !ctx
            .rcu_core
            .normal_after_boot_write_once_trimmed_or_recorded()
        || !ctx.rcu_core.boot_ended_publish_recorded()
    {
        printk::write_str("rcu boot end facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.sysctl_args_deferred.state() != State::Ready
        || !ctx.sysctl_args_deferred.apply_deferred()
        || !ctx.sysctl_args_deferred.command_line_position_preserved()
    {
        printk::write_str("sysctl args deferred facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.finalize_boundary.state() != State::Ready
        || !ctx.finalize_boundary.payload_next_boundary()
    {
        printk::write_str("finalize boundary facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_str("finalize system=running next=payload\n");
    SmokeResult::Passed
}
