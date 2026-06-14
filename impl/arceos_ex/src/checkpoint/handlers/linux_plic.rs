use crate::{
    checkpoint::handlers::{CheckpointOutcome, Handler, HandlerRun, HandlerScope},
    checkpoint::kunit,
    context::Context,
    objects::linux_plic_shim::{self, LinuxPlicBoundaryFacts},
    trace::Checkpoint,
};

const SCOPE: &[Checkpoint] = &[Checkpoint::InitcallBoundaryReady];
pub const KUNIT_CASE_COUNT: usize = 1;

pub const HANDLER: Handler = Handler {
    name: "linux_plic.boundary_facts",
    priority: 88,
    scope: HandlerScope::Only(SCOPE),
    run: HandlerRun::Read(run),
};

fn run(checkpoint: Checkpoint, _ctx: &Context) -> CheckpointOutcome {
    let total = super::kunit_case_count();
    kunit::start_case(total, "", HANDLER.name, checkpoint);

    let facts = linux_plic_shim::boundary_facts();
    emit_facts(facts);
    if !facts_valid(facts) {
        kunit::fail(total, "", HANDLER.name, "Linux PLIC boundary facts invalid");
        return CheckpointOutcome::FailAndShutdown;
    }

    kunit::pass(total, "", HANDLER.name);
    CheckpointOutcome::Continue
}

fn emit_facts(facts: LinuxPlicBoundaryFacts) {
    kunit::diag_usize("initcall_seq", facts.initcall_seq);
    kunit::diag_usize("driver_register_seq", facts.driver_register_seq);
    kunit::diag_usize("platform_match_seq", facts.platform_match_seq);
    kunit::diag_usize("probe_enter_seq", facts.probe_enter_seq);
    kunit::diag_usize("domain_instantiate_seq", facts.domain_instantiate_seq);
    kunit::diag_usize("find_parent_domain_seq", facts.find_parent_domain_seq);
    kunit::diag_usize("create_mapping_seq", facts.create_mapping_seq);
    kunit::diag_usize("set_handler_seq", facts.set_handler_seq);
    kunit::diag_usize("cpuhp_seq", facts.cpuhp_seq);
    kunit::diag_usize("syscore_seq", facts.syscore_seq);
    kunit::diag_usize("probe_return_seq", facts.probe_return_seq);
    kunit::diag_usize("driver_ptr", facts.driver_ptr);
    kunit::diag_usize("probe_ptr", facts.probe_ptr);
    kunit::diag_usize("platform_device_ptr", facts.platform_device_ptr);
    kunit::diag_usize("fwnode_ptr", facts.fwnode_ptr);
    kunit::diag_usize("fwnode_ops_ptr", facts.fwnode_ops_ptr);
    kunit::diag_usize("expected_fwnode_ops_ptr", facts.expected_fwnode_ops_ptr);
    kunit::diag_usize("membase", facts.membase);
    kunit::diag_usize("source_count", facts.source_count);
    kunit::diag_usize("context_count", facts.context_count);
    kunit::diag_usize("context_id", facts.context_id);
    kunit::diag_usize("domain_ptr", facts.domain_ptr);
    kunit::diag_usize("domain_ops", facts.domain_ops);
    kunit::diag_usize("domain_host_data", facts.domain_host_data);
    kunit::diag_usize("parent_irq", facts.parent_irq);
    kunit::diag_usize("chained_irq", facts.chained_irq);
    kunit::diag_usize("chained_handler", facts.chained_handler);
    kunit::diag_usize("chained_is_chained", facts.chained_is_chained);
    kunit::diag_usize("thread_info_base", facts.thread_info_base);
    kunit::diag_usize("thread_info_cpu", facts.thread_info_cpu as usize);
    kunit::diag_usize("per_cpu_offset0", facts.per_cpu_offset0);
    kunit::diag_usize("of_iomap_calls", facts.of_iomap_calls);
    kunit::diag_usize("of_irq_count_calls", facts.of_irq_count_calls);
    kunit::diag_usize("of_irq_parse_calls", facts.of_irq_parse_calls);
    kunit::diag_usize("of_irq_parse_successes", facts.of_irq_parse_successes);
    kunit::diag_usize("of_match_calls", facts.of_match_calls);
    kunit::diag_usize("of_property_ndev_calls", facts.of_property_ndev_calls);
    kunit::diag_usize("heap_used", facts.heap_used);
    kunit::diag_usize("chip_enable_count", facts.chip_enable_count);
    kunit::diag_usize("chip_disable_count", facts.chip_disable_count);
    kunit::diag_usize("chip_mask_count", facts.chip_mask_count);
    kunit::diag_usize("chip_unmask_count", facts.chip_unmask_count);
    kunit::diag_usize("chip_eoi_count", facts.chip_eoi_count);
    kunit::diag_usize(
        "chip_callback_probe_successes",
        facts.chip_callback_probe_successes,
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
        && facts.chip_enable_count >= 2
        && facts.chip_disable_count != 0
        && facts.chip_mask_count != 0
        && facts.chip_unmask_count != 0
        && facts.chip_callback_probe_successes != 0
}

fn strictly_before(before: usize, after: usize) -> bool {
    before != 0 && after != 0 && before < after
}
