/*
 * InitcallPhase coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in initcall.md.
 */

predicate arceos_ex_must_initcall_classify_pre_do_initcalls_by_config() -> bool;
predicate arceos_ex_must_initcall_record_driver_init_deferred_sync_primitives() -> bool;
predicate arceos_ex_must_initcall_dispatcher_remain_entry_agnostic() -> bool;
predicate arceos_ex_must_initcall_record_do_one_initcall_context_repair() -> bool;
predicate arceos_ex_must_initcall_same_level_order_independence_defer_to_proof_or_nightly() -> bool;
predicate arceos_ex_must_of_platform_default_populate_use_device_tree_source() -> bool;
predicate arceos_ex_must_of_platform_default_populate_follow_linux_root_child_traversal() -> bool;
predicate arceos_ex_must_of_platform_default_populate_require_compatible_strict() -> bool;
predicate arceos_ex_must_of_platform_default_populate_recurse_default_bus_matches() -> bool;
predicate arceos_ex_must_of_platform_default_populate_filter_available_nodes() -> bool;
predicate arceos_ex_must_of_platform_default_populate_print_candidate_identity() -> bool;
predicate arceos_ex_must_action_checkpoints_use_entry_exit_and_semantic_points() -> bool;
predicate arceos_ex_must_of_platform_default_populate_scan_complete_after_candidate_print() -> bool;
predicate arceos_ex_must_of_platform_default_populate_create_and_register_platform_devices() -> bool;
predicate arceos_ex_must_platform_bus_own_platform_devices_with_stable_storage() -> bool;
predicate arceos_ex_must_platform_bus_klist_devices_use_vec_device_refs() -> bool;
predicate arceos_ex_must_platform_device_creation_use_stable_device_node_id() -> bool;
predicate arceos_ex_must_device_node_id_resolve_through_persistent_device_tree() -> bool;
predicate arceos_ex_must_device_and_platform_device_store_node_id_not_borrowed_ref() -> bool;
predicate arceos_ex_must_platform_device_storage_keep_device_refs_stable_across_vec_growth() -> bool;
predicate arceos_ex_must_platform_bus_insert_owned_platform_device_before_klist_device_ref() -> bool;
predicate arceos_ex_must_of_platform_default_populate_require_dynamic_container_runtime_ready() -> bool;
predicate arceos_ex_must_of_platform_default_populate_emit_entry_scan_devices_exit_checkpoints() -> bool;
predicate arceos_ex_must_of_platform_population_smoke_cover_refs_to_nodes() -> bool;
predicate arceos_ex_must_of_platform_population_smoke_ignore_unrelated_initcall_entries() -> bool;
predicate arceos_ex_must_bus_device_ref_set_map_to_klist_like_storage() -> bool;
predicate arceos_ex_must_device_driver_ref_set_map_to_klist_like_storage() -> bool;
predicate arceos_ex_must_platform_bus_klist_drivers_use_vec_driver_refs() -> bool;
predicate arceos_ex_must_platform_driver_register_lower_to_device_initcall_static_entry() -> bool;
predicate arceos_ex_must_concrete_driver_declare_own_initcall_in_driver_module() -> bool;
predicate arceos_ex_must_device_driver_descriptor_carry_name_bus_of_match_and_probe() -> bool;
predicate arceos_ex_must_of_match_table_lower_to_static_compatible_array() -> bool;
predicate arceos_ex_must_device_driver_ref_reference_driver_descriptor_not_enum_special_case() -> bool;
predicate arceos_ex_must_device_driver_storage_keep_driver_refs_stable() -> bool;
predicate arceos_ex_must_platform_driver_storage_use_static_or_pinned_owner() -> bool;
predicate arceos_ex_must_platform_bus_match_use_driver_of_match_table_and_device_node() -> bool;
predicate arceos_ex_must_ns16550a_driver_spec_live_in_common_file() -> bool;
predicate arceos_ex_must_ns16550a_driver_impl_live_in_independent_module() -> bool;
predicate arceos_ex_must_ns16550a_initcall_action_call_platform_driver_register() -> bool;
predicate arceos_ex_must_ns16550a_probe_avoid_platform_bus_hardcoded_compatible_branch() -> bool;
predicate arceos_ex_must_platform_bus_probe_driver_scan_existing_devices() -> bool;
predicate arceos_ex_must_platform_bus_probe_device_scan_registered_drivers() -> bool;
predicate arceos_ex_must_ns16550a_driver_match_of_compatible_from_device_node() -> bool;
predicate arceos_ex_must_ns16550a_driver_smoke_cover_probe_and_bind() -> bool;
predicate arceos_ex_must_platform_probe_context_be_context_temporary_window() -> bool;
predicate arceos_ex_must_platform_probe_not_receive_mut_context_directly() -> bool;
predicate arceos_ex_must_platform_probe_context_fields_private() -> bool;
predicate arceos_ex_must_platform_probe_context_expose_narrow_capability_methods() -> bool;
predicate arceos_ex_must_platform_bus_api_not_expose_subsystem_specific_probe_args() -> bool;
predicate arceos_ex_must_device_tree_parse_stdout_path_from_chosen() -> bool;
predicate arceos_ex_must_device_tree_preserve_stdout_path_options_after_colon() -> bool;
predicate arceos_ex_must_device_tree_stdout_path_resolve_to_stable_node_id() -> bool;
predicate arceos_ex_must_ns16550a_probe_parse_resources_from_platform_device_node() -> bool;
predicate arceos_ex_must_ns16550a_probe_create_uart8250_port_object() -> bool;
predicate arceos_ex_must_ns16550a_probe_register_serial8250_console_only_for_stdout_path() -> bool;
predicate arceos_ex_must_earlycon_bootconsole_serialconsole_registry_stay_distinct() -> bool;
predicate arceos_ex_must_boot_console_wrap_earlycon_as_con_boot_registry_entry() -> bool;
predicate arceos_ex_must_console_registry_model_register_console_handoff_policy() -> bool;
predicate arceos_ex_must_console_registry_own_route_cursor_and_keepbootcon_policy() -> bool;
predicate arceos_ex_must_keep_bootcon_prevent_boot_console_unregister() -> bool;
predicate arceos_ex_must_printk_route_switch_to_serial_console_fact_before_mmio_backend() -> bool;
predicate arceos_ex_must_console_handoff_smoke_cover_stdout_path_match_and_nonmatch() -> bool;
predicate arceos_ex_must_console_handoff_smoke_cover_keep_bootcon() -> bool;
predicate arceos_ex_must_console_handoff_smoke_cover_printk_route() -> bool;
predicate arceos_ex_must_device_type_model_linux_struct_device_not_device_type_descriptor() -> bool;
predicate arceos_ex_must_platform_device_embed_device_and_support_container_lookup() -> bool;
predicate arceos_ex_must_raw_intrusive_bus_list_not_own_device_lifetime() -> bool;
predicate arceos_ex_must_future_intrusive_bus_list_wrap_storage_as_safe_intrusive_list() -> bool;
predicate arceos_ex_must_safe_intrusive_list_hide_raw_nodes_and_return_stable_device_refs() -> bool;
predicate arceos_ex_must_device_object_kind_not_define_driver_core_semantics() -> bool;
predicate arceos_ex_must_device_set_node_bind_ref_without_copying_compatible() -> bool;
predicate arceos_ex_must_device_node_ref_store_stable_handle_not_borrowed_view() -> bool;

type ArceosExInitcallCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_initcall_model_path_under_smp_runtime_phase();

        /* Code path. */
        arceos_ex_must_initcall_code_path_follow_smp_runtime_phase_tree();

        /* Entry gate. */
        arceos_ex_must_initcall_run_after_runtime_core();

        /* Pre-do_initcalls classification. */
        arceos_ex_must_initcall_classify_pre_do_initcalls_by_config();

        /* Deferred synchronization. */
        arceos_ex_must_initcall_record_driver_init_deferred_sync_primitives();

        /* Deferred heavy subsystems. */
        arceos_ex_must_initcall_keep_driver_core_and_irq_proc_deferred();

        /* Constructors. */
        arceos_ex_must_initcall_record_ctor_table_boundary();

        /* Entry ABI. */
        arceos_ex_must_initcall_entry_lower_to_context_ref_result_function();

        /* Static registration. */
        arceos_ex_must_initcall_register_lower_to_static_section_entry();

        /* Linker collection. */
        arceos_ex_must_initcall_sections_collected_by_lds_ranges();

        /* Preset collection. */
        arceos_ex_must_initcall_table_preset_collect_static_ranges();

        /* Setup execution. */
        arceos_ex_must_initcall_run_static_initcall_table_summary();

        /* Dispatcher shape. */
        arceos_ex_must_initcall_dispatcher_remain_entry_agnostic();

        /* do_one_initcall context repair. */
        arceos_ex_must_initcall_record_do_one_initcall_context_repair();

        /* Same-level order. */
        arceos_ex_must_initcall_same_level_order_independence_defer_to_proof_or_nightly();

        /* Mechanism/effect split. */
        arceos_ex_must_initcall_keep_static_mechanism_separate_from_entry_effects();

        /* OF platform default populate source. */
        arceos_ex_must_of_platform_default_populate_use_device_tree_source();

        /* Linux-like traversal. */
        arceos_ex_must_of_platform_default_populate_follow_linux_root_child_traversal();

        /* Strict compatible. */
        arceos_ex_must_of_platform_default_populate_require_compatible_strict();

        /* Default bus recursion. */
        arceos_ex_must_of_platform_default_populate_recurse_default_bus_matches();

        /* Availability filter. */
        arceos_ex_must_of_platform_default_populate_filter_available_nodes();

        /* Candidate printout. */
        arceos_ex_must_of_platform_default_populate_print_candidate_identity();

        /* Action checkpoint naming. */
        arceos_ex_must_action_checkpoints_use_entry_exit_and_semantic_points();

        /* Device model naming. */
        arceos_ex_must_device_type_model_linux_struct_device_not_device_type_descriptor();

        /* Platform device core-member lookup. */
        arceos_ex_must_platform_device_embed_device_and_support_container_lookup();

        /* DeviceObject category shell. */
        arceos_ex_must_device_object_kind_not_define_driver_core_semantics();

        /* device_set_node boundary. */
        arceos_ex_must_device_set_node_bind_ref_without_copying_compatible();
        arceos_ex_must_device_node_ref_store_stable_handle_not_borrowed_view();
        arceos_ex_must_device_node_id_resolve_through_persistent_device_tree();
        arceos_ex_must_device_and_platform_device_store_node_id_not_borrowed_ref();

        /* Platform device ownership. */
        arceos_ex_must_of_platform_default_populate_require_dynamic_container_runtime_ready();
        arceos_ex_must_platform_bus_own_platform_devices_with_stable_storage();
        arceos_ex_must_platform_device_creation_use_stable_device_node_id();
        arceos_ex_must_platform_device_storage_keep_device_refs_stable_across_vec_growth();
        arceos_ex_must_platform_bus_insert_owned_platform_device_before_klist_device_ref();

        /* Bus device set storage. */
        arceos_ex_must_bus_device_ref_set_map_to_klist_like_storage();
        arceos_ex_must_platform_bus_klist_devices_use_vec_device_refs();
        arceos_ex_must_raw_intrusive_bus_list_not_own_device_lifetime();
        arceos_ex_must_future_intrusive_bus_list_wrap_storage_as_safe_intrusive_list();
        arceos_ex_must_safe_intrusive_list_hide_raw_nodes_and_return_stable_device_refs();

        /* OF platform scan completion. */
        arceos_ex_must_of_platform_default_populate_scan_complete_after_candidate_print();
        arceos_ex_must_of_platform_default_populate_emit_entry_scan_devices_exit_checkpoints();

        /* OF platform device creation. */
        arceos_ex_must_of_platform_default_populate_create_and_register_platform_devices();
        arceos_ex_must_of_platform_population_smoke_cover_refs_to_nodes();
        arceos_ex_must_of_platform_population_smoke_ignore_unrelated_initcall_entries();

        /* Platform driver registration and probe. */
        arceos_ex_must_device_driver_ref_set_map_to_klist_like_storage();
        arceos_ex_must_platform_bus_klist_drivers_use_vec_driver_refs();
        arceos_ex_must_platform_driver_register_lower_to_device_initcall_static_entry();
        arceos_ex_must_concrete_driver_declare_own_initcall_in_driver_module();
        arceos_ex_must_ns16550a_driver_spec_live_in_common_file();
        arceos_ex_must_ns16550a_driver_impl_live_in_independent_module();
        arceos_ex_must_ns16550a_initcall_action_call_platform_driver_register();
        arceos_ex_must_device_driver_descriptor_carry_name_bus_of_match_and_probe();
        arceos_ex_must_of_match_table_lower_to_static_compatible_array();
        arceos_ex_must_device_driver_ref_reference_driver_descriptor_not_enum_special_case();
        arceos_ex_must_device_driver_storage_keep_driver_refs_stable();
        arceos_ex_must_platform_driver_storage_use_static_or_pinned_owner();
        arceos_ex_must_platform_bus_match_use_driver_of_match_table_and_device_node();
        arceos_ex_must_ns16550a_probe_avoid_platform_bus_hardcoded_compatible_branch();
        arceos_ex_must_platform_bus_probe_driver_scan_existing_devices();
        arceos_ex_must_platform_bus_probe_device_scan_registered_drivers();
        arceos_ex_must_ns16550a_driver_match_of_compatible_from_device_node();
        arceos_ex_must_ns16550a_driver_smoke_cover_probe_and_bind();
        arceos_ex_must_platform_probe_context_be_context_temporary_window();
        arceos_ex_must_platform_probe_not_receive_mut_context_directly();
        arceos_ex_must_platform_probe_context_fields_private();
        arceos_ex_must_platform_probe_context_expose_narrow_capability_methods();
        arceos_ex_must_platform_bus_api_not_expose_subsystem_specific_probe_args();

        /* Console/earlycon handoff. */
        arceos_ex_must_device_tree_parse_stdout_path_from_chosen();
        arceos_ex_must_device_tree_preserve_stdout_path_options_after_colon();
        arceos_ex_must_device_tree_stdout_path_resolve_to_stable_node_id();
        arceos_ex_must_ns16550a_probe_parse_resources_from_platform_device_node();
        arceos_ex_must_ns16550a_probe_create_uart8250_port_object();
        arceos_ex_must_ns16550a_probe_register_serial8250_console_only_for_stdout_path();
        /* devfs first slice. */
        arceos_ex_must_devfs_mount_after_hwrng_and_block_registration();
        arceos_ex_must_devfs_smoke_observe_nodes_without_file_path_read();
        arceos_ex_must_earlycon_bootconsole_serialconsole_registry_stay_distinct();
        arceos_ex_must_boot_console_wrap_earlycon_as_con_boot_registry_entry();
        arceos_ex_must_console_registry_model_register_console_handoff_policy();
        arceos_ex_must_console_registry_own_route_cursor_and_keepbootcon_policy();
        arceos_ex_must_keep_bootcon_prevent_boot_console_unregister();
        arceos_ex_must_printk_route_switch_to_serial_console_fact_before_mmio_backend();
        arceos_ex_must_console_handoff_smoke_cover_stdout_path_match_and_nonmatch();
        arceos_ex_must_console_handoff_smoke_cover_keep_bootcon();
        arceos_ex_must_console_handoff_smoke_cover_printk_route();
    }
}
