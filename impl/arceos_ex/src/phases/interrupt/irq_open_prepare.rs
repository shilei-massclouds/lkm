use crate::{
    arch::riscv64::csr,
    context::Context,
    objects::{
        earlycon, irq_open, printk,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static IRQ_OPEN_PREPARE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::IrqOpenPreparePhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex irq open prepare event failed\n",
    );
    handoff()
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    ctx.slub_subsystem.setup_flush_workqueue(&ctx.workqueue)?;
    ctx.console.preset(&ctx.static_objects)?;
    ctx.irq_open_prepare_trimmed_paths
        .preset(&ctx.config, &ctx.console, &ctx.page_allocator)?;
    ctx.sched_clock.setup(
        &ctx.hrtimer_core,
        &ctx.timekeeper,
        &ctx.riscv_timer_provider,
        &ctx.static_branch,
        &mut ctx.boot_cpu_local_interrupt,
    )?;
    ctx.delay_loop
        .setup(&ctx.riscv_timer_provider, &ctx.cpu_group)?;
    ctx.irq_open_prepare_trimmed_paths
        .setup(&ctx.config, &ctx.sched_clock, &ctx.delay_loop)
}

fn handoff() -> ! {
    crate::phases::interrupt::process_prepare::setup(crate::context::context())
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !irq_open_prepare_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&IRQ_OPEN_PREPARE_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &IRQ_OPEN_PREPARE_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::IrqOpenPreparePhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&IRQ_OPEN_PREPARE_PHASE_STATE) == State::Ready
}

fn irq_open_prepare_phase_ready(ctx: &Context) -> bool {
    crate::phases::interrupt::local_irq_enable::is_ready()
        && ctx.interrupt_stream.state() == State::Online
        && ctx.interrupt_stream.boot_cpu_local_interrupts_enabled()
        && !ctx.interrupt_stream.early_boot_irqs_disabled()
        && ctx.boot_cpu_local_interrupt.state() == State::Ready
        && ctx.boot_cpu_local_interrupt.enabled()
        && csr::supervisor_interrupts_enabled()
        && ctx.irq_dispatch_tree.state() == State::Ready
        && ctx.sbi_ipi.state() == State::Ready
        && ctx.sbi_ipi.enable_deferred()
        && ctx.tick.state() == State::Ready
        && ctx.timer_wheel.state() == State::Ready
        && ctx.hrtimer_core.state() == State::Ready
        && ctx.softirq.state() == State::Ready
        && !ctx.softirq.execution_open()
        && ctx.timekeeper.state() == State::Ready
        && ctx.riscv_timer_provider.state() == State::Ready
        && ctx.smp_call_function.state() == State::Ready
        && ctx.smp_call_function.runtime_ipi_delivery_deferred()
        && ctx.workqueue.state() == State::Prepared
        && !ctx.workqueue.workers_running()
        && ctx.slub_subsystem.state() == State::Ready
        && ctx.slub_subsystem.kmalloc_caches().state() == State::Ready
        && irq_open::slub_flush_workqueue_ready(
            &ctx.slub_subsystem,
            ctx.slub_subsystem.kmalloc_caches(),
        )
        && ctx.page_allocator.state() == State::Ready
        && ctx.per_cpu_storage.state() == State::Ready
        && printk::is_ready()
        && (earlycon::is_online() || printk::console_handoff_complete())
        && ctx.console.state() == State::Prepared
        && ctx.console.line_discipline_registry().state() == State::Prepared
        && ctx.console.line_discipline_registry().n_tty_registered()
        && ctx.console.line_discipline_registry().registered_slot() == 0
        && ctx.console.driver_set().state() == State::Prepared
        && ctx.console.driver_set().early_registered()
        && ctx.console.driver_set().serial_probe_deferred()
        && ctx.console.driver_set().boot_console_unregister_deferred()
        && ctx.console.initcall_table_scanned()
        && ctx.console.real_device_probe_deferred()
        && ctx.console.earlycon_handoff_conditional()
        && ctx.irq_open_prepare_trimmed_paths.state() == State::Ready
        && ctx.irq_open_prepare_trimmed_paths.panic_later_clear()
        && ctx
            .irq_open_prepare_trimmed_paths
            .lockdep_init_trimmed_noop()
        && ctx
            .irq_open_prepare_trimmed_paths
            .lockdep_trimmed_because_config_debug_lock_alloc_disabled()
        && ctx
            .irq_open_prepare_trimmed_paths
            .locking_selftest_trimmed_noop()
        && ctx
            .irq_open_prepare_trimmed_paths
            .locking_selftest_trimmed_because_config_debug_locking_api_selftests_disabled()
        && ctx.irq_open_prepare_trimmed_paths.initrd_bounds_trimmed()
        && ctx
            .irq_open_prepare_trimmed_paths
            .initrd_trimmed_because_config_blk_dev_initrd_disabled()
        && ctx
            .irq_open_prepare_trimmed_paths
            .page_allocator_per_cpu_pagesets_deferred()
        && ctx
            .irq_open_prepare_trimmed_paths
            .page_allocator_deferred_bound()
        && ctx
            .irq_open_prepare_trimmed_paths
            .numa_policy_trimmed_noop()
        && ctx
            .irq_open_prepare_trimmed_paths
            .numa_policy_trimmed_because_config_numa_disabled()
        && ctx.irq_open_prepare_trimmed_paths.acpi_early_trimmed_noop()
        && ctx
            .irq_open_prepare_trimmed_paths
            .acpi_early_trimmed_because_config_acpi_disabled()
        && ctx
            .irq_open_prepare_trimmed_paths
            .late_time_init_hook_trimmed_noop()
        && ctx
            .irq_open_prepare_trimmed_paths
            .late_time_init_hook_unset_on_riscv()
        && ctx
            .irq_open_prepare_trimmed_paths
            .arch_cpu_finalize_init_trimmed_noop()
        && ctx
            .irq_open_prepare_trimmed_paths
            .arch_cpu_finalize_trimmed_because_config_arch_has_cpu_finalize_init_disabled()
        && ctx.irq_open_prepare_trimmed_paths.position_preserved()
        && ctx.sched_clock.state() == State::Ready
        && ctx.sched_clock.running_key_enabled()
        && ctx.sched_clock.reader_ready()
        && ctx.sched_clock.timer_ready()
        && ctx.sched_clock.timer_period() != 0
        && ctx.sched_clock.setup_local_irq_disable_enable_used()
        && ctx
            .sched_clock
            .setup_local_irq_guard_used_by(&ctx.boot_cpu_local_interrupt)
        && ctx.delay_loop.state() == State::Ready
        && ctx.delay_loop.lpj_fine() != 0
        && ctx.delay_loop.boot_cpu_loops_per_jiffy() == ctx.delay_loop.lpj_fine()
        && ctx.delay_loop.global_loops_per_jiffy() == ctx.delay_loop.lpj_fine()
        && ctx.delay_loop.delay_actions_ready()
}
