use crate::{
    apps::smoke::SmokeResult,
    arch::riscv64::csr,
    context::context,
    objects::{printk, state::State},
    phases,
};

const TIME_ADVANCE_SPIN_LIMIT: usize = 1_000_000;

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::interrupt::irq_open_prepare::is_online()
        || !phases::interrupt::is_online()
        || !csr::supervisor_interrupts_enabled()
    {
        printk::write_str("irq open prepare phase is not online\n");
        return SmokeResult::Failed;
    }

    if ctx.slub_subsystem.state() != State::Ready
        || !ctx.slub_subsystem.flush_workqueue_ready()
        || (ctx.workqueue.state() != State::Prepared && ctx.workqueue.state() != State::Ready)
        || ctx.workqueue.workers_running()
    {
        printk::write_str("irq open SLUB/workqueue facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.console.state() != State::Prepared
        || ctx.console.line_discipline_registry().state() != State::Prepared
        || !ctx.console.line_discipline_registry().n_tty_registered()
        || ctx.console.line_discipline_registry().registered_slot() != 0
        || ctx.console.driver_set().state() != State::Prepared
        || !ctx.console.driver_set().early_registered()
        || !ctx.console.driver_set().serial_probe_deferred()
        || !ctx.console.driver_set().boot_console_unregister_deferred()
        || (!ctx.console.real_device_probe_deferred() && !ctx.console.handoff_complete())
        || (!ctx.console.earlycon_handoff_conditional() && !ctx.console.handoff_complete())
    {
        printk::write_str("console prepared facts invalid\n");
        return SmokeResult::Failed;
    }

    if ctx.irq_open_prepare_trimmed_paths.state() != State::Ready
        || !ctx.irq_open_prepare_trimmed_paths.panic_later_clear()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .lockdep_init_trimmed_noop()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .lockdep_trimmed_because_config_debug_lock_alloc_disabled()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .locking_selftest_trimmed_noop()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .locking_selftest_trimmed_because_config_debug_locking_api_selftests_disabled()
        || !ctx.irq_open_prepare_trimmed_paths.initrd_bounds_trimmed()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .initrd_trimmed_because_config_blk_dev_initrd_disabled()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .page_allocator_per_cpu_pagesets_deferred()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .page_allocator_deferred_bound()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .numa_policy_trimmed_noop()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .numa_policy_trimmed_because_config_numa_disabled()
        || !ctx.irq_open_prepare_trimmed_paths.acpi_early_trimmed_noop()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .acpi_early_trimmed_because_config_acpi_disabled()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .late_time_init_hook_trimmed_noop()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .late_time_init_hook_unset_on_riscv()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .arch_cpu_finalize_init_trimmed_noop()
        || !ctx
            .irq_open_prepare_trimmed_paths
            .arch_cpu_finalize_trimmed_because_config_arch_has_cpu_finalize_init_disabled()
        || !ctx.irq_open_prepare_trimmed_paths.position_preserved()
    {
        printk::write_str("irq open trimmed/deferred path facts invalid\n");
        return SmokeResult::Failed;
    }

    if !ctx.sched_clock.setup_local_irq_disable_enable_used()
        || !ctx
            .sched_clock
            .setup_local_irq_guard_used_by(&ctx.boot_cpu_local_interrupt)
    {
        printk::write_str("sched clock local irq guard facts invalid\n");
        return SmokeResult::Failed;
    }

    let Some(first_sched_clock) = ctx.sched_clock.read(&ctx.riscv_timer_provider) else {
        printk::write_str("sched clock read unavailable\n");
        return SmokeResult::Failed;
    };
    let mut second_sched_clock = first_sched_clock;
    let mut spins = 0usize;
    while second_sched_clock == first_sched_clock {
        if spins == TIME_ADVANCE_SPIN_LIMIT {
            printk::write_str("sched clock did not advance\n");
            return SmokeResult::Failed;
        }
        spins += 1;
        let Some(now) = ctx.sched_clock.read(&ctx.riscv_timer_provider) else {
            printk::write_str("sched clock read failed after first read\n");
            return SmokeResult::Failed;
        };
        second_sched_clock = now;
    }

    printk::write_fmt(format_args!(
        "sched_clock_delta={} lpj_fine={}\n",
        second_sched_clock.wrapping_sub(first_sched_clock),
        ctx.delay_loop.lpj_fine()
    ));
    SmokeResult::Passed
}
