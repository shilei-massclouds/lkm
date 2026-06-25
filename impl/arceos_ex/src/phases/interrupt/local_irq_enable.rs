use crate::{
    arch::riscv64::csr,
    context::Context,
    objects::{
        earlycon, printk,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
    trace::Checkpoint,
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static LOCAL_IRQ_ENABLE_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::LocalIrqEnablePhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex local irq enable event failed\n",
    );
    handoff()
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    ctx.interrupt_stream
        .enable(&mut ctx.boot_cpu_local_interrupt)
}

fn handoff() -> ! {
    crate::phases::interrupt::irq_open_prepare::setup(crate::context::context())
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !local_irq_enable_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&LOCAL_IRQ_ENABLE_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &LOCAL_IRQ_ENABLE_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::LocalIrqEnablePhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&LOCAL_IRQ_ENABLE_PHASE_STATE) == State::Ready
}

fn local_irq_enable_phase_ready(ctx: &Context) -> bool {
    crate::phases::interrupt::irq_time_init::is_ready()
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
        && ctx.ipi_mux.state() == State::Ready
        && ctx.smp_call_function.state() == State::Ready
        && ctx.interrupt_stream.state() == State::Online
        && ctx.interrupt_stream.timer_handler_ready()
        && ctx.interrupt_stream.external_handler_ready()
        && ctx.interrupt_stream.boot_cpu_local_interrupts_enabled()
        && !ctx.interrupt_stream.early_boot_irqs_disabled()
        && ctx.boot_cpu_local_interrupt.state() == State::Ready
        && ctx.boot_cpu_local_interrupt.enabled()
        && csr::supervisor_interrupts_enabled()
        && printk::is_ready()
        && (earlycon::is_online() || printk::console_handoff_complete())
}
