use crate::{
    checkpoint::Checkpoint,
    context::Context,
    objects::{
        earlycon, printk,
        state::{failed_condition, EventResult, LifecycleEvent, State},
    },
};
use core::sync::atomic::AtomicU8;

#[unsafe(link_section = ".data.phase")]
static IRQ_TIME_INIT_PHASE_STATE: AtomicU8 =
    AtomicU8::new(crate::phases::state::encode(State::Base));

pub fn setup(ctx: &mut Context) -> ! {
    crate::checkpoint::checkpoint(Checkpoint::IrqTimeInitPhaseStarted);
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
        &ctx.slub_subsystem,
        &ctx.per_cpu_storage,
        &ctx.cpu_group,
    )?;
    ctx.riscv_irq_stack_set.setup(
        &ctx.config,
        &ctx.cpu_group,
        &ctx.vmalloc_allocator,
        &ctx.page_table_caches,
        &ctx.page_allocator,
    )?;
    ctx.riscv_intc
        .setup(&ctx.device_tree, &ctx.irq_controller, &ctx.cpu_group)?;
    ctx.irqchip_init_table
        .preset(&ctx.irq_controller, &ctx.device_tree)?;
    ctx.plic_driver
        .preset(&ctx.irqchip_init_table, &ctx.device_tree)?;
    ctx.irqchip_init_table.setup(
        &ctx.plic_driver,
        &ctx.device_tree,
        &ctx.riscv_intc,
        &ctx.cpu_group,
        &mut ctx.vmalloc_allocator,
        &mut ctx.page_table_caches,
        &mut ctx.page_allocator,
        &ctx.page_metadata_map,
        &ctx.config,
        &mut ctx.ioremap,
        &mut ctx.plic,
    )?;
    ctx.plic_irq_domain.preset(&ctx.plic, &ctx.irq_controller)?;
    ctx.plic_irq_domain.setup(&ctx.plic, &ctx.irq_controller)?;
    ctx.irq_handler_registry
        .setup(&ctx.irq_controller, &ctx.plic_irq_domain)?;
    ctx.irq_dispatch_tree.setup(
        &ctx.irq_controller,
        &ctx.riscv_intc,
        &mut ctx.interrupt_stream,
        &ctx.cpu_group,
        &ctx.plic,
        &ctx.plic_irq_domain,
        &ctx.irq_handler_registry,
    )?;
    ctx.sbi_ipi
        .setup(&ctx.sbi, &ctx.riscv_intc, &ctx.cpu_group)?;
    ctx.ipi_mux
        .setup(&ctx.sbi_ipi, &ctx.per_cpu_storage, &ctx.cpu_group)?;
    ctx.tick.preset(&ctx.cpu_group, &ctx.per_cpu_storage)?;
    ctx.irq_time_trimmed_paths
        .preset(&ctx.config, &ctx.rcu_core, &ctx.tick)?;
    ctx.timer_wheel
        .setup(&ctx.per_cpu_storage, &ctx.cpu_group, &mut ctx.softirq)?;
    ctx.srcu_core
        .setup(&ctx.rcu_core, &ctx.timer_wheel, &ctx.workqueue)?;
    ctx.hrtimer_core
        .setup(&ctx.per_cpu_storage, &ctx.cpu_group, &mut ctx.softirq)?;
    ctx.softirq.setup(&ctx.per_cpu_storage)?;
    ctx.timekeeper.setup(&ctx.tick, &ctx.static_branch)?;
    ctx.riscv_timer_provider.setup(
        &ctx.device_tree,
        &ctx.riscv_intc,
        &ctx.irq_dispatch_tree,
        &mut ctx.timekeeper,
        &ctx.hrtimer_core,
        &ctx.tick,
        &ctx.sbi,
    )?;
    ctx.tick
        .setup(&ctx.hrtimer_core, &ctx.riscv_timer_provider)?;
    let time_seed = crate::arch::riscv64::sbi::read_time();
    ctx.randomness
        .setup(&ctx.cpu_group, &ctx.timekeeper, time_seed)?;
    ctx.irq_time_trimmed_paths
        .setup(&ctx.config, &ctx.randomness)?;
    ctx.boot_stack_canary
        .setup(&ctx.randomness, &ctx.init_stack)?;
    ctx.perf_event_core.setup(&ctx.srcu_core, &ctx.cpu_group)?;
    ctx.profile_core
        .setup(&ctx.randomness, &ctx.perf_event_core)?;
    ctx.smp_call_function
        .setup(&ctx.ipi_mux, &ctx.cpu_group, &ctx.per_cpu_storage)
}

fn handoff() -> ! {
    crate::phases::interrupt::local_irq_enable::setup(crate::context::context())
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
        && ctx.irq_controller.allocator_minimal_ready()
        && ctx.irq_controller.desc_locks_ready()
        && ctx.irq_controller.sparse_irq_tree_lock_deferred()
        && ctx.irq_controller.irq_domain_mutex_deferred()
        && ctx.riscv_intc.state() == State::Ready
        && ctx.riscv_intc.domain_ready()
        && ctx.riscv_intc.boot_cpu_local_causes_ready()
        && ctx.riscv_intc.timer_pin_ready()
        && ctx.riscv_intc.software_pin_ready()
        && ctx.riscv_intc.external_pin_ready()
        && ctx.riscv_intc.boot_cpu_timer_irq_ready()
        && ctx.riscv_intc.boot_cpu_software_irq_ready()
        && ctx.riscv_intc.boot_cpu_external_irq_reserved()
        && ctx.riscv_irq_stack_set.state() == State::Ready
        && ctx.riscv_irq_stack_set.irq_stacks_enabled()
        && ctx.riscv_irq_stack_set.vmap_stack_enabled()
        && ctx.riscv_irq_stack_set.possible_cpu_stacks_ready()
        && ctx.riscv_irq_stack_set.runtime_switch_deferred()
        && ctx.riscv_irq_stack_set.scs_trimmed_noop()
        && ctx
            .riscv_irq_stack_set
            .scs_trimmed_because_shadow_call_stack_disabled()
        && ctx.riscv_irq_stack_set.possible_cpu_count() == ctx.cpu_group.possible_cpu_count()
        && ctx.irqchip_init_table.state() == State::Ready
        && ctx.irqchip_init_table.static_entries_ready()
        && ctx.irqchip_init_table.lds_section_ready()
        && ctx.irqchip_init_table.entry_view_ready()
        && ctx.irqchip_init_table.interrupt_controller_scan_ready()
        && ctx.irqchip_init_table.parent_first_order_ready()
        && ctx.irqchip_init_table.init_irq_called_irqchip_init()
        && ctx.irqchip_init_table.irqchip_init_called_of_irq_init()
        && ctx.irqchip_init_table.of_irq_init_traversed_lds_section()
        && ctx.irqchip_init_table.plic_callback_invoked()
        && ctx.plic_driver.state() == State::Prepared
        && ctx.plic_driver.entry_registered()
        && ctx.plic_driver.registered_in_lds_section()
        && ctx.plic_driver.init_callback_bound()
        && ctx.plic_driver.compatible_covers_qemu_virt()
        && ctx.plic_driver.probe_depends_on_device_tree()
        && ctx.plic_driver.probe_runs_in_irq_time_init()
        && ctx.plic_driver.not_platform_bus_probe()
        && ctx.irq_dispatch_tree.state() == State::Ready
        && ctx.irq_dispatch_tree.fallback_route_ready()
        && ctx.irq_dispatch_tree.timer_route_ready()
        && ctx.irq_dispatch_tree.software_route_reserved()
        && ctx.irq_dispatch_tree.external_route_ready()
        && ctx
            .irq_dispatch_tree
            .external_route_uses_plic_chained_handler()
        && ctx.irq_dispatch_tree.external_route_uses_plic_irq_domain()
        && ctx
            .irq_dispatch_tree
            .external_route_claims_before_dispatch()
        && ctx
            .irq_dispatch_tree
            .external_route_completes_after_handler()
        && ctx.irq_dispatch_tree.boot_cpu_route_ready()
        && ctx.plic.state() == State::Ready
        && ctx.plic.matched_compatible()
        && ctx.plic.setup_called_by_of_irq_init()
        && ctx.plic.interrupt_controller_node_ready()
        && ctx.plic.provider_discovery_reserved()
        && ctx.plic.external_parent_reserved()
        && ctx.plic.output_connected_to_riscv_intc_external_input()
        && ctx.plic.mmio_resource_ready()
        && ctx.plic.ioremapped()
        && ctx.plic.vm_ioremap()
        && ctx.plic.mapbase() != 0
        && ctx.plic.mapsize() != 0
        && ctx.plic.membase() != 0
        && ctx.plic.source_count() != 0
        && ctx.plic.external_input_context_ready()
        && ctx.plic.threshold_ready()
        && ctx.plic.priority_ready()
        && ctx.plic.source_enable_ready()
        && ctx.plic.chained_handler_ready()
        && ctx.plic.claim_action_ready()
        && ctx.plic.complete_action_ready()
        && ctx.plic.claim_reads_claim_register()
        && ctx.plic.claim_zero_means_no_pending()
        && ctx.plic.complete_writes_claimed_source()
        && ctx.plic.claim_before_dispatch()
        && ctx.plic.complete_after_handler()
        && ctx.plic.uart_source_trigger_deferred()
        && ctx.plic.source_observation_counters_ready()
        && ctx.plic_irq_domain.state() == State::Ready
        && ctx.plic_irq_domain.owner_bound()
        && ctx.plic_irq_domain.hwirq_valid_range_ready()
        && ctx.plic_irq_domain.logical_irq_allocator_ready()
        && ctx.plic_irq_domain.mapping_table_ready()
        && ctx.plic_irq_domain.translate_specifier_ready()
        && ctx.plic_irq_domain.dispatch_ops_ready()
        && ctx.plic_irq_domain.source_zero_reserved()
        && ctx.plic_irq_domain.one_cell_specifier()
        && ctx.plic_irq_domain.enable_deferred()
        && ctx.plic_irq_domain.source_count() == ctx.plic.source_count()
        && ctx.irq_handler_registry.state() == State::Ready
        && ctx.irq_handler_registry.action_table_ready()
        && ctx.irq_handler_registry.owner_irq_core()
        && ctx.irq_handler_registry.requires_mapped_logical_irq()
        && ctx.irq_handler_registry.duplicate_policy_ready()
        && ctx.irq_handler_registry.unmapped_reject_ready()
        && ctx.irq_handler_registry.hardirq_context_guard_ready()
        && ctx.irq_handler_registry.source_enable_deferred()
        && ctx.irq_handler_registry.dispatch_ready()
        && ctx.irq_handler_registry.dispatch_requires_hardirq_context()
        && ctx.tick.state() == State::Ready
        && ctx.tick.control_ready()
        && ctx.tick.nohz_trimmed()
        && ctx.tick.boot_cpu_tick_device_ready()
        && ctx.tick.broadcast().state() == State::Ready
        && ctx.tick.broadcast().masks_ready()
        && ctx.tick.broadcast().clockevent_ready()
        && ctx.irq_time_trimmed_paths.state() == State::Ready
        && ctx.irq_time_trimmed_paths.rcu_init_nohz_trimmed_noop()
        && ctx
            .irq_time_trimmed_paths
            .rcu_nohz_trimmed_because_config_rcu_nocb_cpu_disabled()
        && ctx.irq_time_trimmed_paths.rcu_nohz_position_preserved()
        && ctx.irq_time_trimmed_paths.kfence_init_trimmed_noop()
        && ctx
            .irq_time_trimmed_paths
            .kfence_trimmed_because_config_kfence_disabled()
        && ctx.irq_time_trimmed_paths.kfence_position_preserved()
        && ctx.timer_wheel.state() == State::Ready
        && ctx.timer_wheel.cpu_timer_bases_ready()
        && ctx.timer_wheel.base_locks_ready()
        && ctx.timer_wheel.pending_maps_ready()
        && ctx.timer_wheel.vectors_empty()
        && ctx.timer_wheel.posix_cpu_timer_work_ready()
        && ctx.timer_wheel.timer_softirq_registered()
        && ctx.srcu_core.state() == State::Ready
        && ctx.srcu_core.tree_srcu_enabled()
        && ctx.srcu_core.init_done()
        && ctx.srcu_core.boot_list_drained()
        && ctx.srcu_core.per_struct_locks_deferred()
        && ctx.srcu_core.delayed_work_queueing_ready()
        && ctx.hrtimer_core.state() == State::Ready
        && ctx.hrtimer_core.boot_cpu_base_ready()
        && ctx.hrtimer_core.base_locks_ready()
        && ctx.hrtimer_core.clock_bases_ready()
        && ctx.hrtimer_core.active_queues_empty()
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
        && ctx.timekeeper.tk_core_seqcount_ready()
        && ctx.timekeeper.tk_core_write_seqcount_used()
        && ctx.timekeeper.timekeeper_lock_ready()
        && ctx.timekeeper.raw_spinlock_irqsave_used()
        && ctx.timekeeper.irqsave_flags_restored()
        && ctx.timekeeper.shadow_timekeeper_ready()
        && ctx.riscv_timer_provider.state() == State::Ready
        && ctx.riscv_timer_provider.timebase_hz() != 0
        && ctx.riscv_timer_provider.clocksource_registered()
        && ctx.riscv_timer_provider.clockevent_registered()
        && ctx.riscv_timer_provider.irq_mapping_ready()
        && ctx.riscv_timer_provider.sbi_programming_ready()
        && ctx.interrupt_stream.timer_handler_ready()
        && ctx.interrupt_stream.external_handler_ready()
        && ctx
            .interrupt_stream
            .supervisor_external_input_gate_defined()
        && ctx.interrupt_stream.supervisor_external_input_gate_closed()
        && ctx
            .interrupt_stream
            .supervisor_external_input_enable_deferred()
        && ctx.softirq.state() == State::Ready
        && ctx.softirq.action_table_ready()
        && ctx.softirq.pending_set_ready()
        && ctx.softirq.rcu_action_registered()
        && ctx.softirq.timer_action_registered()
        && ctx.softirq.hrtimer_action_registered()
        && ctx.softirq.tasklet_queues_ready()
        && ctx.softirq.tasklet_actions_registered()
        && !ctx.softirq.execution_open()
        && ctx.randomness.state() == State::Ready
        && ctx.randomness.is_fully_ready()
        && ctx.randomness.timekeeping_required()
        && ctx.randomness.cycle_entropy_mixed()
        && ctx.randomness.input_pool_lock_deferred()
        && ctx.randomness.base_crng_lock_deferred()
        && ctx.randomness.pm_notifier_deferred()
        && ctx.boot_stack_canary.state() == State::Ready
        && ctx.boot_stack_canary.stackprotector_enabled()
        && ctx.boot_stack_canary.per_task_canary_ready()
        && ctx.boot_stack_canary.randomness_dependency_used()
        && ctx.boot_stack_canary.boot_init_task_canary_seeded()
        && ctx.perf_event_core.state() == State::Ready
        && ctx.perf_event_core.enabled_by_config()
        && ctx.perf_event_core.pmu_idr_ready()
        && ctx.perf_event_core.pmus_srcu_ready()
        && ctx.perf_event_core.pmu_registry_ready()
        && ctx.perf_event_core.cpu_context_locks_ready()
        && ctx.perf_event_core.swevent_pmus_registered()
        && ctx.perf_event_core.reboot_notifier_registered()
        && ctx.perf_event_core.event_cache_deferred()
        && ctx.perf_event_core.hw_breakpoint_deferred()
        && ctx.profile_core.state() == State::Ready
        && ctx.profile_core.profiling_enabled_by_config()
        && ctx.profile_core.profile_param_absent()
        && ctx.profile_core.buffer_allocation_trimmed()
        && ctx.profile_core.proc_export_deferred()
        && ctx.ipi_mux.state() == State::Ready
        && ctx.ipi_mux.domain_ready()
        && ctx.ipi_mux.per_cpu_bits_ready()
        && ctx.ipi_mux.virtual_ipi_range_ready()
        && ctx.ipi_mux.parent_software_irq_ready()
        && ctx.ipi_mux.secondary_enable_deferred()
        && ctx.sbi_ipi.state() == State::Ready
        && ctx.sbi_ipi.irq_mapping_ready()
        && ctx.sbi_ipi.send_action_ready()
        && ctx.sbi_ipi.enable_deferred()
        && ctx.smp_call_function.state() == State::Ready
        && ctx.smp_call_function.call_single_queue_ready()
        && ctx.smp_call_function.call_single_queue_locks_ready()
        && ctx.smp_call_function.ipi_route_ready()
        && ctx.smp_call_function.ipi_mux_ready()
        && ctx.smp_call_function.runtime_ipi_delivery_deferred()
        && ctx.smp_call_function.possible_cpu_count() == ctx.cpu_group.possible_cpu_count()
        && ctx.interrupt_stream.state() == State::Ready
        && !ctx.interrupt_stream.boot_cpu_local_interrupts_enabled()
        && ctx.interrupt_stream.early_boot_irqs_disabled()
        && ctx.boot_cpu_local_interrupt.state() == State::Ready
        && ctx.boot_cpu_local_interrupt.disabled()
        && !crate::arch::riscv64::csr::supervisor_interrupts_enabled()
        && printk::is_ready()
        && (earlycon::is_online() || printk::console_handoff_complete())
}
