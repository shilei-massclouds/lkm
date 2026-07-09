/*
 * IrqTimeInitPhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in irq-time-init.md.
 */

predicate arceos_ex_must_irq_time_init_model_path_under_interrupt_phase() -> bool;
predicate arceos_ex_must_irq_time_init_code_path_follow_interrupt_phase_tree() -> bool;
predicate arceos_ex_must_irq_time_init_keep_local_irq_closed() -> bool;
predicate arceos_ex_must_irq_time_init_record_riscv_irq_stack_setup() -> bool;
predicate arceos_ex_must_irq_time_init_record_timekeeper_irqsave_seqwrite() -> bool;
predicate arceos_ex_must_irq_time_init_classify_rcu_nohz_and_kfence_by_config() -> bool;
predicate arceos_ex_must_irq_time_init_keep_task_and_smp_concurrency_closed() -> bool;
predicate arceos_ex_must_irq_time_init_keep_runtime_services_deferred() -> bool;
predicate arceos_ex_must_irq_time_init_expose_time_and_clockevent_smoke_actions() -> bool;
predicate arceos_ex_must_model_plic_as_independent_irqchip_driver() -> bool;
predicate arceos_ex_must_irqchip_init_entries_use_static_lds_section() -> bool;
predicate arceos_ex_must_init_irq_chain_find_plic_driver_from_irqchip_section() -> bool;
predicate arceos_ex_must_of_irq_init_call_plic_init_via_matched_section_entry() -> bool;
predicate arceos_ex_must_plic_output_feed_riscv_intc_external_input() -> bool;
predicate arceos_ex_must_use_named_interrupt_causes_not_raw_numbers() -> bool;
predicate arceos_ex_must_model_external_irq_gates_as_named_closed_gates() -> bool;
predicate arceos_ex_must_plic_ioremap_as_system_irqchip_not_platform_device() -> bool;
predicate arceos_ex_must_plic_setup_parse_dt_reg_ndev_and_interrupts_extended() -> bool;
predicate arceos_ex_must_model_irqdomain_as_type_and_plic_domain_as_instance() -> bool;
predicate arceos_ex_must_plic_irq_domain_map_source_to_logical_irq_only() -> bool;
predicate arceos_ex_must_platform_irq_resource_parse_uart_interrupts_from_dt() -> bool;
predicate arceos_ex_must_uart_irq_mapping_not_enable_source_or_handler() -> bool;
predicate arceos_ex_must_uart_external_irq_enable_be_explicit_boundary() -> bool;
predicate arceos_ex_must_uart_interrupt_chain_probe_be_production_boundary() -> bool;
predicate arceos_ex_must_uart_interrupt_chain_probe_observe_full_irq_cycle() -> bool;
predicate arceos_ex_must_model_irq_handler_registry_as_irq_core_object() -> bool;
predicate arceos_ex_must_request_irq_require_mapped_logical_irq() -> bool;
predicate arceos_ex_must_request_irq_record_handler_without_enabling_source() -> bool;
predicate arceos_ex_must_irq_handler_context_guard_remain_deferred_execution() -> bool;
predicate arceos_ex_must_root_intc_external_irq_enter_plic_chained_handler_only() -> bool;
predicate arceos_ex_must_plic_claim_before_irq_dispatch_and_complete_after_handler() -> bool;
predicate arceos_ex_must_irq_core_dispatch_registered_action_by_logical_irq() -> bool;
predicate arceos_ex_must_uart_irq_chain_kunit_remain_read_only_observer() -> bool;
predicate arceos_ex_must_uart_irq_chain_kunit_not_drive_interrupt_flow() -> bool;
predicate arceos_ex_must_kunit_handlers_receive_read_only_context_by_default() -> bool;
predicate arceos_ex_must_kunit_writes_go_through_limited_sink_capability() -> bool;
predicate arceos_ex_must_kunit_sink_not_access_or_mutate_context_objects() -> bool;
predicate arceos_ex_must_checkpoint_handler_run_keep_single_observer_variant() -> bool;
predicate arceos_ex_must_checkpoint_handler_run_not_accept_mut_context() -> bool;
predicate arceos_ex_must_not_reintroduce_checkpoint_write_handler_variant() -> bool;
predicate arceos_ex_must_not_register_smoke_cases_as_checkpoint_handlers() -> bool;
predicate arceos_ex_must_checkpoint_consumers_be_cfg_selected() -> bool;
predicate arceos_ex_must_log_trace_and_probe_remain_distinct_consumers() -> bool;
predicate arceos_ex_must_checkpoint_inventory_use_checkpoint_mod_as_source() -> bool;
predicate arceos_ex_must_checkpoint_inventory_export_stable_fields() -> bool;
predicate arceos_ex_must_checkpoint_inventory_not_modify_runtime_or_linux() -> bool;
predicate arceos_ex_must_checkpoint_inventory_support_regeneration_check() -> bool;
predicate arceos_ex_must_linux_checkpoint_mapping_be_mapping_only() -> bool;
predicate arceos_ex_must_linux_checkpoint_mapping_consume_inventory_and_read_linux_only() -> bool;
predicate arceos_ex_must_linux_checkpoint_mapping_preserve_checkpoint_order() -> bool;
predicate arceos_ex_must_linux_checkpoint_mapping_classify_exact_range_unmapped() -> bool;
predicate arceos_ex_must_linux_checkpoint_mapping_export_stable_fields() -> bool;
predicate arceos_ex_must_linux_checkpoint_mapping_not_instrument_or_collect_runtime() -> bool;
predicate arceos_ex_must_linux_checkpoint_mapping_guard_userboot_userexec_ownership() -> bool;
predicate arceos_ex_must_linux_checkpoint_mapping_support_riscv64_entry_anchors() -> bool;
predicate arceos_ex_must_linux_checkpoint_mapping_keep_entry_arch_scope_explicit() -> bool;
predicate arceos_ex_must_linux_checkpoint_mapping_support_regeneration_check() -> bool;
predicate arceos_ex_must_linux_checkpoint_coverage_be_mapping_only_review_artifact() -> bool;
predicate arceos_ex_must_linux_checkpoint_coverage_export_aggregate_fields_only() -> bool;
predicate arceos_ex_must_linux_checkpoint_coverage_support_regeneration_check() -> bool;
predicate arceos_ex_must_paired_checkpoint_scope_be_hard_comparison_scope_only() -> bool;
predicate arceos_ex_must_paired_checkpoint_coverage_account_all_required_mappings() -> bool;
predicate arceos_ex_must_linux_checkpoint_marker_patch_derive_from_plan() -> bool;
predicate arceos_ex_must_linux_checkpoint_marker_patch_not_mutate_linux_tree() -> bool;
predicate arceos_ex_must_linux_checkpoint_marker_patch_insert_before_anchor() -> bool;
predicate arceos_ex_must_linux_checkpoint_marker_patch_sort_same_anchor_by_index() -> bool;
predicate arceos_ex_must_linux_checkpoint_marker_patch_reject_stale_or_mismatched_markers() -> bool;
predicate arceos_ex_must_linux_checkpoint_marker_check_report_summary_counts() -> bool;
predicate arceos_ex_must_define_observation_levels() -> bool;
predicate arceos_ex_must_observation_domains_be_subsystem_or_object_scoped() -> bool;
predicate arceos_ex_must_observation_facts_be_owned_by_objects_or_providers() -> bool;
predicate arceos_ex_must_failure_diagnostic_collection_and_output_be_separate() -> bool;

type ArceosExIrqTimeInitCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_irq_time_init_model_path_under_interrupt_phase();

        /* Code path. */
        arceos_ex_must_irq_time_init_code_path_follow_interrupt_phase_tree();

        /* Interrupt-open boundary. */
        arceos_ex_must_irq_time_init_keep_local_irq_closed();

        /* RISC-V IRQ stack/SCS setup. */
        arceos_ex_must_irq_time_init_record_riscv_irq_stack_setup();

        /* Timekeeper lock protocol. */
        arceos_ex_must_irq_time_init_record_timekeeper_irqsave_seqwrite();

        /* Config-trimmed calls. */
        arceos_ex_must_irq_time_init_classify_rcu_nohz_and_kfence_by_config();

        /* PLIC driver split. */
        arceos_ex_must_model_plic_as_independent_irqchip_driver();

        /* IRQCHIP_DECLARE lowering. */
        arceos_ex_must_irqchip_init_entries_use_static_lds_section();

        /* init_IRQ call chain. */
        arceos_ex_must_init_irq_chain_find_plic_driver_from_irqchip_section();

        /* Compatible match and callback. */
        arceos_ex_must_of_irq_init_call_plic_init_via_matched_section_entry();

        /* Parent interrupt link. */
        arceos_ex_must_plic_output_feed_riscv_intc_external_input();

        /* Named interrupt causes. */
        arceos_ex_must_use_named_interrupt_causes_not_raw_numbers();

        /* External IRQ gates. */
        arceos_ex_must_model_external_irq_gates_as_named_closed_gates();

        /* PLIC MMIO ownership. */
        arceos_ex_must_plic_ioremap_as_system_irqchip_not_platform_device();

        /* PLIC DT setup. */
        arceos_ex_must_plic_setup_parse_dt_reg_ndev_and_interrupts_extended();

        /* IRQ domain split. */
        arceos_ex_must_model_irqdomain_as_type_and_plic_domain_as_instance();

        /* PLIC source mapping. */
        arceos_ex_must_plic_irq_domain_map_source_to_logical_irq_only();

        /* UART IRQ resource. */
        arceos_ex_must_platform_irq_resource_parse_uart_interrupts_from_dt();

        /* Deferred interrupt output. */
        arceos_ex_must_uart_irq_mapping_not_enable_source_or_handler();

        /* Explicit external IRQ enable. */
        arceos_ex_must_uart_external_irq_enable_be_explicit_boundary();

        /* Production UART interrupt-chain probe. */
        arceos_ex_must_uart_interrupt_chain_probe_be_production_boundary();

        /* UART interrupt cycle observation. */
        arceos_ex_must_uart_interrupt_chain_probe_observe_full_irq_cycle();

        /* IRQ handler registry. */
        arceos_ex_must_model_irq_handler_registry_as_irq_core_object();

        /* request_irq input contract. */
        arceos_ex_must_request_irq_require_mapped_logical_irq();

        /* Handler registration boundary. */
        arceos_ex_must_request_irq_record_handler_without_enabling_source();

        /* Context guard. */
        arceos_ex_must_irq_handler_context_guard_remain_deferred_execution();

        /* External interrupt dispatch contract. */
        arceos_ex_must_root_intc_external_irq_enter_plic_chained_handler_only();

        /* PLIC claim/complete order. */
        arceos_ex_must_plic_claim_before_irq_dispatch_and_complete_after_handler();

        /* IRQ core action dispatch. */
        arceos_ex_must_irq_core_dispatch_registered_action_by_logical_irq();

        /* KUnit capability boundary. */
        arceos_ex_must_kunit_handlers_receive_read_only_context_by_default();
        arceos_ex_must_checkpoint_handler_run_keep_single_observer_variant();
        arceos_ex_must_checkpoint_handler_run_not_accept_mut_context();
        arceos_ex_must_not_reintroduce_checkpoint_write_handler_variant();

        /* Checkpoint consumers. */
        arceos_ex_must_checkpoint_consumers_be_cfg_selected();
        arceos_ex_must_log_trace_and_probe_remain_distinct_consumers();

        /* Checkpoint inventory export. */
        arceos_ex_must_checkpoint_inventory_use_checkpoint_mod_as_source();
        arceos_ex_must_checkpoint_inventory_export_stable_fields();
        arceos_ex_must_checkpoint_inventory_not_modify_runtime_or_linux();
        arceos_ex_must_checkpoint_inventory_support_regeneration_check();

        /* Linux checkpoint alignment mapping. */
        arceos_ex_must_linux_checkpoint_mapping_be_mapping_only();
        arceos_ex_must_linux_checkpoint_mapping_consume_inventory_and_read_linux_only();
        arceos_ex_must_linux_checkpoint_mapping_preserve_checkpoint_order();
        arceos_ex_must_linux_checkpoint_mapping_classify_exact_range_unmapped();
        arceos_ex_must_linux_checkpoint_mapping_export_stable_fields();
        arceos_ex_must_linux_checkpoint_mapping_not_instrument_or_collect_runtime();
        arceos_ex_must_linux_checkpoint_mapping_guard_userboot_userexec_ownership();
        arceos_ex_must_linux_checkpoint_mapping_support_riscv64_entry_anchors();
        arceos_ex_must_linux_checkpoint_mapping_keep_entry_arch_scope_explicit();
        arceos_ex_must_linux_checkpoint_mapping_ignore_marker_comment_lines();
        arceos_ex_must_linux_checkpoint_mapping_support_regeneration_check();
        arceos_ex_must_linux_checkpoint_mapping_ignore_runtime_recorder_lines();

        /* Linux checkpoint mapping coverage review. */
        arceos_ex_must_linux_checkpoint_coverage_be_mapping_only_review_artifact();
        arceos_ex_must_linux_checkpoint_coverage_export_aggregate_fields_only();
        arceos_ex_must_linux_checkpoint_coverage_support_regeneration_check();
        arceos_ex_must_paired_checkpoint_scope_be_hard_comparison_scope_only();
        arceos_ex_must_paired_checkpoint_coverage_account_all_required_mappings();

        /* Linux checkpoint marker patch generation. */
        arceos_ex_must_linux_checkpoint_marker_patch_derive_from_plan();
        arceos_ex_must_linux_checkpoint_marker_patch_not_mutate_linux_tree();
        arceos_ex_must_linux_checkpoint_marker_patch_insert_before_anchor();
        arceos_ex_must_linux_checkpoint_marker_patch_sort_same_anchor_by_index();
        arceos_ex_must_linux_checkpoint_marker_patch_reject_stale_or_mismatched_markers();
        arceos_ex_must_linux_checkpoint_marker_check_report_summary_counts();

        /* Observation levels and domains. */
        arceos_ex_must_define_observation_levels();
        arceos_ex_must_observation_domains_be_subsystem_or_object_scoped();
        arceos_ex_must_observation_facts_be_owned_by_objects_or_providers();

        /* Failure diagnostic lifecycle. */
        arceos_ex_must_failure_diagnostic_collection_and_output_be_separate();

        /* Sink-only writes. */
        arceos_ex_must_kunit_writes_go_through_limited_sink_capability();
        arceos_ex_must_kunit_sink_not_access_or_mutate_context_objects();

        /* Smoke separation. */
        arceos_ex_must_not_register_smoke_cases_as_checkpoint_handlers();

        /* UART IRQ chain KUnit boundary. */
        arceos_ex_must_uart_irq_chain_kunit_remain_read_only_observer();

        /* Flow ownership. */
        arceos_ex_must_uart_irq_chain_kunit_not_drive_interrupt_flow();

        /* Concurrency scope. */
        arceos_ex_must_irq_time_init_keep_task_and_smp_concurrency_closed();

        /* Runtime services. */
        arceos_ex_must_irq_time_init_keep_runtime_services_deferred();

        /* Smoke actions. */
        arceos_ex_must_irq_time_init_expose_time_and_clockevent_smoke_actions();
    }
}
