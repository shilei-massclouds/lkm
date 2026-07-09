use crate::{
    checkpoint::Checkpoint,
    checkpoint::{
        handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
        kunit::Sink,
    },
    context::Context,
    objects::{
        linux_plic_shim::{self, LinuxPlicBoundaryFacts},
        ns16550a,
    },
};

const SCOPE: &[Checkpoint] = &[Checkpoint::InitcallBoundaryReady];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "linux_plic.boundary_facts",
    priority: 88,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Observe(run),
};

fn run(checkpoint: Checkpoint, _ctx: &Context, sink: &mut dyn Sink) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    sink.start_case(total, "", HANDLER.name, checkpoint);

    let facts = linux_plic_shim::boundary_facts();
    emit_facts(facts, sink);
    if !facts_valid(facts) {
        sink.fail(total, "", HANDLER.name, "Linux PLIC boundary facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    sink.pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}

fn emit_facts(facts: LinuxPlicBoundaryFacts, sink: &mut dyn Sink) {
    sink.diag_usize("initcall_seq", facts.initcall_seq);
    sink.diag_usize("driver_register_seq", facts.driver_register_seq);
    sink.diag_usize("platform_match_seq", facts.platform_match_seq);
    sink.diag_usize("probe_enter_seq", facts.probe_enter_seq);
    sink.diag_usize("domain_instantiate_seq", facts.domain_instantiate_seq);
    sink.diag_usize("find_parent_domain_seq", facts.find_parent_domain_seq);
    sink.diag_usize("create_mapping_seq", facts.create_mapping_seq);
    sink.diag_usize("set_handler_seq", facts.set_handler_seq);
    sink.diag_usize("cpuhp_seq", facts.cpuhp_seq);
    sink.diag_usize("syscore_seq", facts.syscore_seq);
    sink.diag_usize("probe_return_seq", facts.probe_return_seq);
    sink.diag_usize("driver_ptr", facts.driver_ptr);
    sink.diag_usize("probe_ptr", facts.probe_ptr);
    sink.diag_usize("platform_device_ptr", facts.platform_device_ptr);
    sink.diag_usize("fwnode_ptr", facts.fwnode_ptr);
    sink.diag_usize("fwnode_ops_ptr", facts.fwnode_ops_ptr);
    sink.diag_usize("expected_fwnode_ops_ptr", facts.expected_fwnode_ops_ptr);
    sink.diag_usize("membase", facts.membase);
    sink.diag_usize("source_count", facts.source_count);
    sink.diag_usize("context_count", facts.context_count);
    sink.diag_usize("context_id", facts.context_id);
    sink.diag_usize("domain_ptr", facts.domain_ptr);
    sink.diag_usize("domain_ops", facts.domain_ops);
    sink.diag_usize("domain_host_data", facts.domain_host_data);
    sink.diag_usize("parent_irq", facts.parent_irq);
    sink.diag_usize("chained_irq", facts.chained_irq);
    sink.diag_usize("chained_handler", facts.chained_handler);
    sink.diag_usize("chained_is_chained", facts.chained_is_chained);
    sink.diag_usize("thread_info_base", facts.thread_info_base);
    sink.diag_usize("thread_info_cpu", facts.thread_info_cpu as usize);
    sink.diag_usize("per_cpu_offset0", facts.per_cpu_offset0);
    sink.diag_usize("of_iomap_calls", facts.of_iomap_calls);
    sink.diag_usize("of_irq_count_calls", facts.of_irq_count_calls);
    sink.diag_usize("of_irq_parse_calls", facts.of_irq_parse_calls);
    sink.diag_usize("of_irq_parse_successes", facts.of_irq_parse_successes);
    sink.diag_usize("of_match_calls", facts.of_match_calls);
    sink.diag_usize("of_property_ndev_calls", facts.of_property_ndev_calls);
    sink.diag_usize("heap_used", facts.heap_used);
    sink.diag_usize(
        "unmapped_irq_failure_count",
        facts.unmapped_irq_failure_count,
    );
    sink.diag_usize(
        "unmapped_irq_last_source",
        facts.unmapped_irq_last_source as usize,
    );
    sink.diag_usize("unmapped_irq_last_errno", facts.unmapped_irq_last_errno);
    sink.diag_usize(
        "unmapped_irq_exercise_successes",
        facts.unmapped_irq_exercise_successes,
    );
    sink.diag_usize("ratelimit_deferred_count", facts.ratelimit_deferred_count);
    sink.diag_usize("chip_enable_count", facts.chip_enable_count);
    sink.diag_usize("chip_disable_count", facts.chip_disable_count);
    sink.diag_usize("chip_mask_count", facts.chip_mask_count);
    sink.diag_usize("chip_unmask_count", facts.chip_unmask_count);
    sink.diag_usize("chip_ack_count", facts.chip_ack_count);
    sink.diag_usize("chip_eoi_count", facts.chip_eoi_count);
    sink.diag_usize("chip_set_type_count", facts.chip_set_type_count);
    sink.diag_usize("chip_disabled_eoi_count", facts.chip_disabled_eoi_count);
    sink.diag_usize("chip_edge_ack_count", facts.chip_edge_ack_count);
    sink.diag_usize(
        "chip_callback_exercise_successes",
        facts.chip_callback_exercise_successes,
    );
    sink.diag_usize(
        "edge_callback_exercise_successes",
        facts.edge_callback_exercise_successes,
    );
    sink.diag_usize("parent_desc_prepare_count", facts.parent_desc_prepare_count);
    sink.diag_usize("parent_irq_data_get_count", facts.parent_irq_data_get_count);
    sink.diag_usize(
        "parent_enable_percpu_count",
        facts.parent_enable_percpu_count,
    );
    sink.diag_usize(
        "parent_enable_percpu_last_irq",
        facts.parent_enable_percpu_last_irq,
    );
    sink.diag_usize(
        "parent_enable_percpu_last_type",
        facts.parent_enable_percpu_last_type as usize,
    );
    sink.diag_usize("parent_irq_eoi_count", facts.parent_irq_eoi_count);
    sink.diag_usize("parent_status", facts.parent_status as usize);
    sink.diag_usize("irq_modify_status_count", facts.irq_modify_status_count);
    sink.diag_usize(
        "irq_modify_status_last_irq",
        facts.irq_modify_status_last_irq,
    );
    sink.diag_usize(
        "irq_modify_status_last_clear",
        facts.irq_modify_status_last_clear,
    );
    sink.diag_usize(
        "irq_modify_status_last_set",
        facts.irq_modify_status_last_set,
    );
    sink.diag_usize(
        "irq_modify_status_last_status",
        facts.irq_modify_status_last_status,
    );
    sink.diag_usize("leaf_action_prepare_count", facts.leaf_action_prepare_count);
    sink.diag_usize(
        "leaf_action_dispatch_count",
        facts.leaf_action_dispatch_count,
    );
    sink.diag_usize("leaf_action_last_irq", facts.leaf_action_last_irq);
    sink.diag_usize("leaf_action_last_depth", facts.leaf_action_last_depth);
    sink.diag_usize(
        "leaf_action_last_handler_kind",
        facts.leaf_action_last_handler_kind,
    );
    sink.diag_usize(
        "leaf_action_last_handler_bound",
        facts.leaf_action_last_handler_bound as usize,
    );
    sink.diag_usize("action_request_count", facts.action_request_count);
    sink.diag_usize("action_request_last_irq", facts.action_request_last_irq);
    sink.diag_usize(
        "action_request_last_device",
        facts.action_request_last_device,
    );
    sink.diag_usize(
        "action_request_last_handler_kind",
        facts.action_request_last_handler_kind,
    );
    sink.diag_usize(
        "action_request_match_count",
        facts.action_request_match_count,
    );
    sink.diag_usize(
        "action_chain_install_count",
        facts.action_chain_install_count,
    );
    sink.diag_usize("action_chain_match_count", facts.action_chain_match_count);
    sink.diag_usize("action_chain_last_irq", facts.action_chain_last_irq);
    sink.diag_usize("action_chain_last_action", facts.action_chain_last_action);
    sink.diag_usize("action_chain_last_device", facts.action_chain_last_device);
    sink.diag_usize(
        "action_chain_last_handler_kind",
        facts.action_chain_last_handler_kind,
    );
    sink.diag_usize(
        "irq_desc_action_write_count",
        facts.irq_desc_action_write_count,
    );
    sink.diag_usize(
        "irq_desc_action_match_count",
        facts.irq_desc_action_match_count,
    );
    sink.diag_usize("irq_desc_action_last_irq", facts.irq_desc_action_last_irq);
    sink.diag_usize(
        "irq_desc_action_last_action",
        facts.irq_desc_action_last_action,
    );
    sink.diag_usize(
        "irq_desc_action_last_readback",
        facts.irq_desc_action_last_readback,
    );
    sink.diag_usize(
        "irq_desc_status_write_count",
        facts.irq_desc_status_write_count,
    );
    sink.diag_usize("irq_desc_status_last_irq", facts.irq_desc_status_last_irq);
    sink.diag_usize(
        "irq_desc_status_last_status",
        facts.irq_desc_status_last_status,
    );
    sink.diag_usize(
        "irq_common_state_write_count",
        facts.irq_common_state_write_count,
    );
    sink.diag_usize(
        "irq_common_state_disabled_count",
        facts.irq_common_state_disabled_count,
    );
    sink.diag_usize("irq_common_state_last_irq", facts.irq_common_state_last_irq);
    sink.diag_usize(
        "irq_common_state_last_state",
        facts.irq_common_state_last_state,
    );
    sink.diag_usize(
        "leaf_action_last_desc_status",
        facts.leaf_action_last_desc_status,
    );
    sink.diag_usize(
        "leaf_action_last_common_state",
        facts.leaf_action_last_common_state,
    );
}

fn facts_valid(facts: LinuxPlicBoundaryFacts) -> bool {
    facts.driver_registered
        && facts.driver_matched
        && facts.probe_entered
        && facts.probe_return == Some(0)
        && strictly_before(facts.initcall_seq, facts.driver_register_seq)
        && strictly_before(facts.driver_register_seq, facts.platform_match_seq)
        && strictly_before(facts.platform_match_seq, facts.probe_enter_seq)
        && strictly_before(facts.probe_enter_seq, facts.domain_instantiate_seq)
        && strictly_before(facts.domain_instantiate_seq, facts.find_parent_domain_seq)
        && strictly_before(facts.find_parent_domain_seq, facts.create_mapping_seq)
        && strictly_before(facts.create_mapping_seq, facts.set_handler_seq)
        && strictly_before(facts.set_handler_seq, facts.cpuhp_seq)
        && strictly_before(facts.cpuhp_seq, facts.syscore_seq)
        && strictly_before(facts.syscore_seq, facts.probe_return_seq)
        && facts.driver_ptr != 0
        && facts.probe_ptr != 0
        && facts.platform_device_ptr != 0
        && facts.fwnode_ptr != 0
        && facts.fwnode_ops_ptr == facts.expected_fwnode_ops_ptr
        && facts.membase != 0
        && facts.source_count != 0
        && facts.context_count != 0
        && facts.context_id < facts.context_count
        && facts.domain_ptr != 0
        && facts.domain_ops != 0
        && facts.domain_host_data != 0
        && facts.parent_irq == facts.chained_irq
        && facts.parent_irq == 9
        && facts.chained_handler != 0
        && facts.chained_is_chained == 1
        && facts.thread_info_base != 0
        && facts.thread_info_cpu == 0
        && facts.per_cpu_offset0 == 0
        && facts.of_iomap_calls != 0
        && facts.of_irq_count_calls != 0
        && facts.of_irq_parse_calls != 0
        && facts.of_irq_parse_successes == 1
        && facts.of_match_calls != 0
        && facts.of_property_ndev_calls != 0
        && facts.heap_used != 0
        && facts.unmapped_irq_failure_count != 0
        && facts.unmapped_irq_last_source != 0
        && facts.unmapped_irq_last_errno == 22
        && facts.unmapped_irq_exercise_successes != 0
        && facts.chip_enable_count >= 2
        && facts.chip_disable_count != 0
        && facts.chip_mask_count != 0
        && facts.chip_unmask_count != 0
        && facts.chip_ack_count != 0
        && facts.chip_set_type_count >= 2
        && facts.chip_disabled_eoi_count != 0
        && facts.chip_edge_ack_count != 0
        && facts.chip_callback_exercise_successes != 0
        && facts.edge_callback_exercise_successes != 0
        && facts.parent_desc_prepare_count != 0
        && facts.parent_irq_data_get_count != 0
        && facts.parent_enable_percpu_count != 0
        && facts.parent_enable_percpu_last_irq == facts.parent_irq
        && facts.parent_enable_percpu_last_type == 0
        && parent_chained_status_valid(facts.parent_status)
        && facts.irq_modify_status_count != 0
        && facts.irq_modify_status_last_irq == ns16550a::uart8250_port_logical_irq().as_usize()
        && facts.irq_modify_status_last_clear == 0
        && facts.irq_modify_status_last_set == linux_irq_noprobe() as usize
        && facts.irq_modify_status_last_status & linux_irq_noprobe() as usize != 0
        && facts.leaf_action_prepare_count != 0
        && facts.leaf_action_dispatch_count != 0
        && facts.leaf_action_last_irq == ns16550a::uart8250_port_logical_irq().as_usize()
        && facts.leaf_action_last_depth == linux_irq_action_depth_enabled()
        && facts.leaf_action_last_handler_kind == linux_irq_handler_kind_ns16550a_uart()
        && facts.leaf_action_last_handler_bound
        && facts.action_request_count != 0
        && facts.action_request_last_irq == ns16550a::uart8250_port_logical_irq().as_usize()
        && facts.action_request_last_device
            == ns16550a::uart8250_port_device_ref().map_or(usize::MAX, |device| device.index())
        && facts.action_request_last_handler_kind == linux_irq_handler_kind_ns16550a_uart()
        && facts.action_request_match_count != 0
        && facts.action_chain_install_count != 0
        && facts.action_chain_match_count != 0
        && facts.action_chain_last_irq == ns16550a::uart8250_port_logical_irq().as_usize()
        && facts.action_chain_last_action != 0
        && facts.action_chain_last_device
            == ns16550a::uart8250_port_device_ref().map_or(usize::MAX, |device| device.index())
        && facts.action_chain_last_handler_kind == linux_irq_handler_kind_ns16550a_uart()
        && facts.irq_desc_action_write_count != 0
        && facts.irq_desc_action_match_count != 0
        && facts.irq_desc_action_last_irq == ns16550a::uart8250_port_logical_irq().as_usize()
        && facts.irq_desc_action_last_action == facts.action_chain_last_action
        && facts.irq_desc_action_last_readback == facts.action_chain_last_action
        && facts.irq_desc_status_write_count != 0
        && (facts.irq_desc_status_last_irq == facts.parent_irq
            || facts.irq_desc_status_last_irq == ns16550a::uart8250_port_logical_irq().as_usize())
        && facts.irq_desc_status_last_status & linux_irq_noprobe() as usize != 0
        && facts.irq_common_state_write_count != 0
        && facts.irq_common_state_disabled_count != 0
        && facts.irq_common_state_last_irq == ns16550a::uart8250_port_logical_irq().as_usize()
        && facts.irq_common_state_last_state & linux_irqd_irq_disabled() as usize == 0
        && facts.leaf_action_last_desc_status == facts.irq_modify_status_last_status
        && facts.leaf_action_last_desc_status & linux_irq_noprobe() as usize != 0
        && facts.leaf_action_last_common_state & linux_irqd_irq_disabled() as usize == 0
}

fn strictly_before(before: usize, after: usize) -> bool {
    before != 0 && after != 0 && before < after
}

fn parent_chained_status_valid(status: u32) -> bool {
    let required = linux_irq_norequest() | linux_irq_noprobe() | linux_irq_nothread();
    status & required == required
}

const fn linux_irq_noprobe() -> u32 {
    1u32 << 10
}

const fn linux_irq_norequest() -> u32 {
    1u32 << 11
}

const fn linux_irq_nothread() -> u32 {
    1u32 << 16
}

const fn linux_irqd_irq_disabled() -> u32 {
    1u32 << 16
}

const fn linux_irq_action_depth_enabled() -> usize {
    0
}

const fn linux_irq_handler_kind_ns16550a_uart() -> usize {
    1
}
