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
static IRQ_TIME_INIT_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::trace::checkpoint(Checkpoint::IrqTimeInitPhaseStarted);
    crate::phases::shutdown_on_error(
        setup_objects(ctx).and_then(|()| checkpoint_ready(ctx)),
        "arceos_ex irq time init event failed\n",
    );
    handoff()
}

fn setup_objects(ctx: &mut Context) -> EventResult {
    ctx.irq_controller.setup(
        &ctx.device_tree,
        &ctx.page_allocator,
        &ctx.slub_allocator,
        &ctx.per_cpu_storage,
        &ctx.cpu_group,
    )?;
    ctx.irq_dispatch_tree.setup(
        &ctx.irq_controller,
        &mut ctx.interrupt_stream,
        &ctx.cpu_group,
    )?;
    ctx.tick.preset(&ctx.cpu_group, &ctx.per_cpu_storage)?;
    ctx.timer_wheel
        .setup(&ctx.per_cpu_storage, &ctx.cpu_group, &mut ctx.softirq)?;
    ctx.hrtimer_core
        .setup(&ctx.per_cpu_storage, &ctx.cpu_group, &mut ctx.softirq)?;
    ctx.timekeeper.setup(&ctx.tick, &ctx.static_branch)?;
    ctx.riscv_timer_provider.setup(
        &ctx.device_tree,
        &ctx.irq_controller,
        &ctx.irq_dispatch_tree,
        &mut ctx.timekeeper,
        &ctx.hrtimer_core,
        &ctx.tick,
        &ctx.sbi,
    )?;
    ctx.tick
        .setup(&ctx.hrtimer_core, &ctx.riscv_timer_provider)?;
    ctx.softirq.setup(&ctx.per_cpu_storage)?;
    let time_seed = crate::arch::riscv64::sbi::read_time();
    ctx.randomness.setup(&ctx.cpu_group, time_seed)?;
    ctx.sbi_ipi
        .setup(&ctx.sbi, &ctx.irq_controller, &ctx.cpu_group)?;
    ctx.smp_call_function
        .setup(&ctx.sbi_ipi, &ctx.cpu_group, &ctx.per_cpu_storage)?;
    ctx.interrupt_stream.enable()
}

fn handoff() -> ! {
    crate::phases::boot::setup_after_children()
}

fn checkpoint_ready(ctx: &Context) -> EventResult {
    if !irq_time_init_phase_ready(ctx) {
        return failed_condition(
            LifecycleEvent::Setup,
            crate::phases::state::load(&IRQ_TIME_INIT_PHASE_STATE),
            State::Base,
            State::Ready,
        );
    }

    crate::phases::state::mark(
        &IRQ_TIME_INIT_PHASE_STATE,
        LifecycleEvent::Setup,
        State::Base,
        State::Ready,
        Checkpoint::IrqTimeInitPhaseReady,
    )
}

pub fn is_ready() -> bool {
    crate::phases::state::load(&IRQ_TIME_INIT_PHASE_STATE) == State::Ready
}

fn irq_time_init_phase_ready(ctx: &Context) -> bool {
    crate::phases::boot::sched_init::is_ready()
        && ctx.irq_controller.state() == State::Ready
        && ctx.irq_controller.descriptors_ready()
        && ctx.irq_controller.domain_ready()
        && ctx.irq_controller.riscv_intc_ready()
        && ctx.irq_controller.boot_cpu_timer_irq_ready()
        && ctx.irq_dispatch_tree.state() == State::Ready
        && ctx.irq_dispatch_tree.fallback_route_ready()
        && ctx.irq_dispatch_tree.timer_route_ready()
        && ctx.irq_dispatch_tree.boot_cpu_route_ready()
        && ctx.tick.state() == State::Ready
        && ctx.tick.control_ready()
        && ctx.tick.nohz_trimmed()
        && ctx.tick.boot_cpu_tick_device_ready()
        && ctx.tick.broadcast().state() == State::Ready
        && ctx.tick.broadcast().masks_ready()
        && ctx.tick.broadcast().clockevent_ready()
        && ctx.timer_wheel.state() == State::Ready
        && ctx.timer_wheel.cpu_timer_bases_ready()
        && ctx.timer_wheel.posix_cpu_timer_work_ready()
        && ctx.timer_wheel.timer_softirq_registered()
        && ctx.hrtimer_core.state() == State::Ready
        && ctx.hrtimer_core.boot_cpu_base_ready()
        && ctx.hrtimer_core.hrtimer_softirq_registered()
        && ctx.timekeeper.state() == State::Ready
        && ctx.timekeeper.clocksource_core().state() == State::Prepared
        && ctx.timekeeper.clocksource_core().registry_ready()
        && ctx
            .timekeeper
            .clocksource_core()
            .riscv_clocksource_registered()
        && ctx.timekeeper.jiffies_clocksource().state() == State::Prepared
        && ctx.timekeeper.jiffies_clocksource().available()
        && ctx.timekeeper.wall_time_ready()
        && ctx.timekeeper.monotonic_time_ready()
        && ctx.timekeeper.raw_time_ready()
        && ctx.riscv_timer_provider.state() == State::Ready
        && ctx.riscv_timer_provider.timebase_hz() != 0
        && ctx.riscv_timer_provider.clocksource_registered()
        && ctx.riscv_timer_provider.clockevent_registered()
        && ctx.riscv_timer_provider.irq_mapping_ready()
        && ctx.riscv_timer_provider.sbi_programming_ready()
        && ctx.interrupt_stream.timer_handler_ready()
        && ctx.softirq.state() == State::Ready
        && ctx.softirq.action_table_ready()
        && ctx.softirq.pending_set_ready()
        && ctx.softirq.timer_action_registered()
        && ctx.softirq.hrtimer_action_registered()
        && ctx.softirq.tasklet_queues_ready()
        && ctx.softirq.tasklet_actions_registered()
        && !ctx.softirq.execution_open()
        && ctx.randomness.state() == State::Ready
        && ctx.randomness.is_fully_ready()
        && ctx.sbi_ipi.state() == State::Ready
        && ctx.sbi_ipi.irq_mapping_ready()
        && ctx.sbi_ipi.enable_deferred()
        && ctx.smp_call_function.state() == State::Ready
        && ctx.smp_call_function.call_single_queue_ready()
        && ctx.smp_call_function.ipi_route_ready()
        && ctx.smp_call_function.possible_cpu_count() == ctx.cpu_group.possible_cpu_count()
        && ctx.interrupt_stream.state() == State::Online
        && ctx.interrupt_stream.boot_cpu_local_interrupts_enabled()
        && csr::supervisor_interrupts_enabled()
        && printk::is_ready()
        && earlycon::is_online()
}
