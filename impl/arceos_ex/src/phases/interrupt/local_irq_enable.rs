use crate::{
    arch::riscv64::csr,
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        earlycon, printk,
        state::{EventResult, LifecycleEvent, State, failed_condition},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static LOCAL_IRQ_ENABLE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn preset(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        preset_start(ctx),
        "arceos_ex local irq enable preset start failed\n",
    );
    crate::checkpoint::checkpoint(Checkpoint::LocalIrqEnablePhaseStarted);
    crate::phases::shutdown_on_error(
        preset_objects(ctx).and_then(|()| adopt_prepared_with_check(ctx)),
        "arceos_ex local irq enable preset failed\n",
    );
    setup(ctx)
}

fn preset_start(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&LOCAL_IRQ_ENABLE_PHASE_STATE);
    if state != State::Base
        || !crate::phases::interrupt::irq_time_init::is_online()
        || ctx.interrupt_stream.state() != State::Ready
        || !ctx.interrupt_stream.early_boot_irqs_disabled()
        || ctx.boot_cpu_local_interrupt.state() != State::Ready
        || !ctx.boot_cpu_local_interrupt.disabled()
        || csr::supervisor_interrupts_enabled()
        || ctx.cpu_group.smp_concurrency_open()
        || ctx.task_creation_core.state() != State::Base
    {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }
    Ok(())
}

fn preset_objects(ctx: &mut Context) -> EventResult {
    ctx.interrupt_stream
        .enable(&mut ctx.boot_cpu_local_interrupt)
}

fn adopt_prepared_with_check(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&LOCAL_IRQ_ENABLE_PHASE_STATE);
    if state != State::Base || !local_irq_enable_phase_ready(ctx) {
        return failed_condition(LifecycleEvent::Preset, state, State::Base, State::Prepared);
    }

    crate::phases::state::mark_checked(
        &LOCAL_IRQ_ENABLE_PHASE_STATE,
        LifecycleEvent::Preset,
        State::Base,
        State::Prepared,
        Checkpoint::LocalIrqEnablePhasePrepared,
    )
}

fn setup(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        adopt_ready(ctx),
        "arceos_ex local irq enable setup failed\n",
    );
    enable(ctx)
}

fn adopt_ready(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&LOCAL_IRQ_ENABLE_PHASE_STATE);
    if state != State::Prepared || !local_irq_enable_phase_ready(ctx) {
        return failed_condition(LifecycleEvent::Setup, state, State::Prepared, State::Ready);
    }

    crate::phases::state::mark_checked(
        &LOCAL_IRQ_ENABLE_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Prepared,
        State::Ready,
        Checkpoint::LocalIrqEnablePhaseReady,
    )
}

fn enable(ctx: &mut Context) -> ! {
    crate::phases::shutdown_on_error(
        enable_event(ctx),
        "arceos_ex local irq enable event failed\n",
    );
    crate::phases::interrupt::preset_after_local_irq_enable()
}

fn enable_event(ctx: &Context) -> EventResult {
    let state = crate::phases::state::load(&LOCAL_IRQ_ENABLE_PHASE_STATE);
    if state != State::Ready || !local_irq_enable_phase_ready(ctx) {
        return failed_condition(LifecycleEvent::Enable, state, State::Ready, State::Online);
    }

    crate::phases::state::mark_checked(
        &LOCAL_IRQ_ENABLE_PHASE_STATE,
        LifecycleEvent::Enable,
        State::Ready,
        State::Online,
        Checkpoint::LocalIrqEnablePhaseOnline,
    )
}

pub fn is_online() -> bool {
    crate::phases::state::load(&LOCAL_IRQ_ENABLE_PHASE_STATE) == State::Online
}

fn local_irq_enable_phase_ready(ctx: &Context) -> bool {
    crate::phases::interrupt::irq_time_init::is_online()
        && ctx.irq_controller.state() == State::Ready
        && ctx.riscv_intc.state() == State::Ready
        && ctx.irq_dispatch_tree.state() == State::Ready
        && ctx.tick.state() == State::Ready
        && ctx.timer_wheel.state() == State::Ready
        && ctx.hrtimer_core.state() == State::Ready
        && ctx.softirq.state() == State::Ready
        && !ctx.softirq.execution_open()
        && ctx.timekeeper.state() == State::Ready
        && ctx.riscv_timer_provider.state() == State::Ready
        && ctx.randomness.state() == State::Ready
        && ctx.sbi_ipi.state() == State::Ready
        && ctx.sbi_ipi.enable_deferred()
        && ctx.ipi_mux.state() == State::Ready
        && ctx.ipi_mux.secondary_enable_deferred()
        && ctx.smp_call_function.state() == State::Ready
        && ctx.smp_call_function.runtime_ipi_delivery_deferred()
        && ctx.interrupt_stream.state() == State::Online
        && ctx.interrupt_stream.timer_handler_ready()
        && ctx.interrupt_stream.external_handler_ready()
        && ctx
            .interrupt_stream
            .supervisor_external_input_gate_defined()
        && ctx.interrupt_stream.supervisor_external_input_gate_closed()
        && ctx
            .interrupt_stream
            .supervisor_external_input_enable_deferred()
        && ctx.interrupt_stream.boot_cpu_local_interrupts_enabled()
        && !ctx.interrupt_stream.early_boot_irqs_disabled()
        && ctx.boot_cpu_local_interrupt.state() == State::Ready
        && ctx.boot_cpu_local_interrupt.enabled()
        && csr::supervisor_interrupts_enabled()
        && ctx.plic_irq_domain.state() == State::Ready
        && ctx.plic_irq_domain.enable_deferred()
        && ctx.irq_handler_registry.state() == State::Ready
        && ctx.irq_handler_registry.source_enable_deferred()
        && ctx.workqueue.state() == State::Prepared
        && !ctx.workqueue.workers_running()
        && ctx.rcu_core.state() == State::Ready
        && ctx.rcu_core.gp_threads_deferred()
        && ctx.rcu_core.tasks_rcu().gp_threads_deferred()
        && ctx.task_creation_core.state() == State::Base
        && !ctx.cpu_group.smp_concurrency_open()
        && printk::is_ready()
        && (earlycon::is_online() || printk::console_handoff_complete())
}
