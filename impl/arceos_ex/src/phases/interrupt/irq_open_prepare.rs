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
    ctx.slub_allocator.setup_flush_workqueue(&ctx.workqueue)?;
    ctx.console.preset(&ctx.static_objects)?;
    checkpoint_panic_later_clear()?;
    checkpoint_lockdep_noop()?;
    checkpoint_locking_selftest_noop()?;
    checkpoint_initrd_bounds_trimmed()?;
    checkpoint_page_allocator_percpu_pagesets_deferred()?;
    checkpoint_numa_policy_noop()?;
    checkpoint_acpi_early_noop()?;
    checkpoint_late_time_init_noop()?;
    ctx.sched_clock.setup(
        &ctx.hrtimer_core,
        &ctx.timekeeper,
        &ctx.riscv_timer_provider,
        &ctx.static_branch,
    )?;
    ctx.delay_loop
        .setup(&ctx.riscv_timer_provider, &ctx.cpu_group)?;
    checkpoint_arch_cpu_finalize_noop()
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
    crate::phases::interrupt::irq_time_init::is_ready()
        && ctx.interrupt_stream.state() == State::Online
        && ctx.interrupt_stream.boot_cpu_local_interrupts_enabled()
        && csr::supervisor_interrupts_enabled()
        && ctx.irq_dispatch_tree.state() == State::Ready
        && ctx.sbi_ipi.state() == State::Ready
        && ctx.tick.state() == State::Ready
        && ctx.timer_wheel.state() == State::Ready
        && ctx.hrtimer_core.state() == State::Ready
        && ctx.softirq.state() == State::Ready
        && !ctx.softirq.execution_open()
        && ctx.timekeeper.state() == State::Ready
        && ctx.riscv_timer_provider.state() == State::Ready
        && ctx.smp_call_function.state() == State::Ready
        && ctx.workqueue.state() == State::Prepared
        && !ctx.workqueue.workers_running()
        && ctx.slub_allocator.state() == State::Ready
        && ctx.slub_allocator.kmalloc_caches().state() == State::Ready
        && irq_open::slub_flush_workqueue_ready(
            &ctx.slub_allocator,
            ctx.slub_allocator.kmalloc_caches(),
        )
        && ctx.page_allocator.state() == State::Ready
        && ctx.per_cpu_storage.state() == State::Ready
        && printk::is_ready()
        && earlycon::is_online()
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
        && ctx.sched_clock.state() == State::Ready
        && ctx.sched_clock.running_key_enabled()
        && ctx.sched_clock.reader_ready()
        && ctx.sched_clock.timer_ready()
        && ctx.sched_clock.timer_period() != 0
        && ctx.delay_loop.state() == State::Ready
        && ctx.delay_loop.lpj_fine() != 0
        && ctx.delay_loop.boot_cpu_loops_per_jiffy() == ctx.delay_loop.lpj_fine()
        && ctx.delay_loop.global_loops_per_jiffy() == ctx.delay_loop.lpj_fine()
        && ctx.delay_loop.delay_actions_ready()
}

fn checkpoint_panic_later_clear() -> EventResult {
    crate::trace::checkpoint(Checkpoint::PanicLaterClearCheckpoint);
    Ok(())
}

fn checkpoint_lockdep_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::LockdepInitNoop);
    Ok(())
}

fn checkpoint_locking_selftest_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::LockingSelftestNoop);
    Ok(())
}

fn checkpoint_initrd_bounds_trimmed() -> EventResult {
    crate::trace::checkpoint(Checkpoint::InitrdBoundsTrimmed);
    Ok(())
}

fn checkpoint_page_allocator_percpu_pagesets_deferred() -> EventResult {
    crate::trace::checkpoint(Checkpoint::PageAllocatorPerCpuPagesetsDeferred);
    Ok(())
}

fn checkpoint_numa_policy_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::NumaPolicyNoop);
    Ok(())
}

fn checkpoint_acpi_early_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::AcpiEarlyNoop);
    Ok(())
}

fn checkpoint_late_time_init_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::LateTimeInitNoop);
    Ok(())
}

fn checkpoint_arch_cpu_finalize_noop() -> EventResult {
    crate::trace::checkpoint(Checkpoint::ArchCpuFinalizeNoop);
    Ok(())
}
