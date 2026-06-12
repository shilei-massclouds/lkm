use crate::{
    apps::smoke::SmokeResult,
    arch::riscv64::{csr, sbi},
    context::context,
    objects::{irq_time, printk, state::State},
    phases,
};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

static CLOCKEVENT_CALLBACKS: AtomicUsize = AtomicUsize::new(0);
static CLOCKEVENT_DEADLINE: AtomicU64 = AtomicU64::new(0);
const TIME_ADVANCE_SPIN_LIMIT: usize = 1_000_000;

pub fn run() -> SmokeResult {
    let ctx = context();

    if !phases::interrupt::irq_time_init::is_ready()
        || ctx.interrupt_stream.state() != State::Online
        || !ctx
            .interrupt_stream
            .supervisor_external_input_gate_defined()
        || !ctx.interrupt_stream.supervisor_external_input_gate_open()
        || ctx.boot_cpu_local_interrupt.state() != State::Ready
        || !ctx.boot_cpu_local_interrupt.enabled()
        || !csr::supervisor_interrupts_enabled()
    {
        printk::write_str("irq time phase did not open boot CPU interrupts\n");
        return SmokeResult::Failed;
    }

    if ctx.irq_controller.state() != State::Ready
        || ctx.riscv_intc.state() != State::Ready
        || ctx.irqchip_init_table.state() != State::Ready
        || ctx.plic_driver.state() != State::Prepared
        || ctx.irq_dispatch_tree.state() != State::Ready
        || ctx.plic.state() != State::Ready
        || ctx.plic_irq_domain.state() != State::Ready
        || ctx.irq_handler_registry.state() != State::Ready
        || ctx.tick.state() != State::Ready
        || ctx.timer_wheel.state() != State::Ready
        || ctx.hrtimer_core.state() != State::Ready
        || ctx.timekeeper.state() != State::Ready
        || ctx.riscv_timer_provider.state() != State::Ready
        || ctx.softirq.state() != State::Ready
        || ctx.randomness.state() != State::Ready
        || ctx.ipi_mux.state() != State::Ready
        || ctx.sbi_ipi.state() != State::Ready
        || ctx.smp_call_function.state() != State::Ready
    {
        printk::write_str("irq time objects are not ready\n");
        return SmokeResult::Failed;
    }

    if ctx.irqchip_init_table.entry_count() == 0
        || ctx.irqchip_init_table.run_count() == 0
        || !ctx.irqchip_init_table.contains_entry(b"sifive,plic-1.0.0")
        || !ctx.irqchip_init_table.lds_section_ready()
        || !ctx.irqchip_init_table.init_irq_called_irqchip_init()
        || !ctx.irqchip_init_table.irqchip_init_called_of_irq_init()
        || !ctx.irqchip_init_table.of_irq_init_traversed_lds_section()
        || !ctx.irqchip_init_table.plic_callback_invoked()
        || !ctx.plic_driver.registered_in_lds_section()
        || !ctx.plic_driver.init_callback_bound()
        || !ctx.plic_driver.not_platform_bus_probe()
        || !ctx.plic.matched_compatible()
        || !ctx.plic.setup_called_by_of_irq_init()
        || !ctx.plic.output_connected_to_riscv_intc_external_input()
        || !ctx.plic.mmio_resource_ready()
        || !ctx.plic.ioremapped()
        || !ctx.plic.vm_ioremap()
        || ctx.plic.mapbase() == 0
        || ctx.plic.mapsize() == 0
        || ctx.plic.membase() == 0
        || ctx.plic.source_count() == 0
        || !ctx.plic.external_input_context_ready()
        || !ctx.plic.threshold_ready()
        || !ctx.plic.priority_ready()
        || !ctx.plic.source_enable_ready()
        || !ctx.plic.chained_handler_ready()
        || !ctx.plic.claim_action_ready()
        || !ctx.plic.complete_action_ready()
        || !ctx.plic.claim_reads_claim_register()
        || !ctx.plic.claim_zero_means_no_pending()
        || !ctx.plic.complete_writes_claimed_source()
        || !ctx.plic.claim_loop_until_zero()
        || !ctx.plic.zero_claim_stops_dispatch()
        || !ctx.plic.completes_each_claimed_source()
        || !ctx.plic.claim_before_dispatch()
        || !ctx.plic.complete_after_handler()
        || !ctx.plic_irq_domain.owner_bound()
        || !ctx.plic_irq_domain.hwirq_valid_range_ready()
        || !ctx.plic_irq_domain.logical_irq_allocator_ready()
        || !ctx.plic_irq_domain.mapping_table_ready()
        || !ctx.plic_irq_domain.translate_specifier_ready()
        || !ctx.plic_irq_domain.dispatch_ops_ready()
        || !ctx.plic_irq_domain.source_zero_reserved()
        || !ctx.plic_irq_domain.one_cell_specifier()
        || !ctx.plic_irq_domain.enable_deferred()
        || ctx.plic_irq_domain.source_count() != ctx.plic.source_count()
        || !ctx.irq_handler_registry.action_table_ready()
        || !ctx.irq_handler_registry.owner_irq_core()
        || !ctx.irq_handler_registry.requires_mapped_logical_irq()
        || !ctx.irq_handler_registry.duplicate_policy_ready()
        || !ctx.irq_handler_registry.unmapped_reject_ready()
        || !ctx.irq_handler_registry.hardirq_context_guard_ready()
        || !ctx.irq_handler_registry.source_enable_deferred()
        || !ctx.irq_handler_registry.dispatch_ready()
        || !ctx.irq_handler_registry.dispatch_requires_hardirq_context()
        || ctx
            .plic_irq_domain
            .translate_one_cell_specifier(&[0])
            .is_some()
        || ctx
            .plic_irq_domain
            .translate_one_cell_specifier(&[ctx.plic.source_count().saturating_add(1)])
            .is_some()
        || !plic_irqchip_callback_recorded(ctx)
    {
        printk::write_str("plic irqchip section traversal facts invalid\n");
        return SmokeResult::Failed;
    }

    let Some(first_time) = ctx.riscv_timer_provider.read_time() else {
        printk::write_str("time read action unavailable\n");
        return SmokeResult::Failed;
    };
    let mut second_time = first_time;
    let mut time_spins = 0usize;
    while second_time == first_time {
        if time_spins == TIME_ADVANCE_SPIN_LIMIT {
            printk::write_str("time source did not advance\n");
            return SmokeResult::Failed;
        }
        time_spins += 1;

        let Some(now) = ctx.riscv_timer_provider.read_time() else {
            printk::write_str("time read action failed after first read\n");
            return SmokeResult::Failed;
        };
        second_time = now;
    }

    CLOCKEVENT_CALLBACKS.store(0, Ordering::Relaxed);
    CLOCKEVENT_DEADLINE.store(0, Ordering::Relaxed);

    let interrupts_before = irq_time::timer_interrupt_count();
    let delta = (ctx.riscv_timer_provider.timebase_hz() / 1000).max(1);
    let timeout = (ctx.riscv_timer_provider.timebase_hz() / 10).max(delta * 2);
    let timeout_deadline = sbi::read_time().wrapping_add(timeout);

    let Some(clockevent_deadline) = ctx
        .riscv_timer_provider
        .schedule_oneshot(delta, clockevent_callback)
    else {
        printk::write_str("failed to schedule clockevent callback\n");
        return SmokeResult::Failed;
    };

    while CLOCKEVENT_CALLBACKS.load(Ordering::Relaxed) == 0 {
        if sbi::read_time().wrapping_sub(timeout_deadline) < (1u64 << 63) {
            printk::write_str("clockevent callback did not fire\n");
            csr::disable_supervisor_timer_interrupt();
            return SmokeResult::Failed;
        }
        core::hint::spin_loop();
    }

    if !csr::supervisor_interrupts_enabled() {
        printk::write_str("timer interrupt returned with interrupts disabled\n");
        return SmokeResult::Failed;
    }
    let callback_count = CLOCKEVENT_CALLBACKS.load(Ordering::Relaxed);
    let callback_deadline = CLOCKEVENT_DEADLINE.load(Ordering::Relaxed);
    if irq_time::timer_interrupt_count() != interrupts_before.wrapping_add(1)
        || callback_count != 1
        || callback_deadline != clockevent_deadline
    {
        printk::write_str("clockevent callback facts invalid\n");
        return SmokeResult::Failed;
    }

    printk::write_fmt(format_args!(
        "time_delta={} clockevent_callbacks={} timer_interrupts={} timebase_hz={}\n",
        second_time.wrapping_sub(first_time),
        callback_count,
        irq_time::timer_interrupt_count(),
        ctx.riscv_timer_provider.timebase_hz()
    ));
    SmokeResult::Passed
}

fn clockevent_callback(deadline: u64) {
    CLOCKEVENT_DEADLINE.store(deadline, Ordering::Relaxed);
    CLOCKEVENT_CALLBACKS.fetch_add(1, Ordering::Relaxed);
}

fn plic_irqchip_callback_recorded(ctx: &crate::context::Context) -> bool {
    let mut index = 0usize;
    while index < ctx.irqchip_init_table.run_count() {
        let Some(record) = ctx.irqchip_init_table.run_record(index) else {
            return false;
        };
        if (record.name() == "sifive_plic" || record.name() == "riscv_plic0")
            && (record.compatible() == b"sifive,plic-1.0.0"
                || record.compatible() == b"riscv,plic0")
            && record.matched()
            && record.callback_invoked()
            && record.return_ok()
        {
            return true;
        }
        index += 1;
    }
    false
}
