/*
 * arceos_ex formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in arceos_ex.md.
 */

predicate arceos_ex_must_device_tree_setup_allocates_from_memblock() -> bool;
predicate arceos_ex_must_device_tree_unflatten_uses_two_passes() -> bool;
predicate arceos_ex_must_device_tree_storage_uses_established_linear_mapping() -> bool;
predicate arceos_ex_must_device_tree_avoid_heap_and_fixed_static_storage() -> bool;
predicate arceos_ex_must_device_tree_checkpoint_after_validation() -> bool;
predicate arceos_ex_should_encapsulate_device_tree_unflatten_unsafe() -> bool;
predicate arceos_ex_must_memory_topology_project_existing_zones_only() -> bool;
predicate arceos_ex_must_page_allocator_preset_only_builds_topology_and_hooks() -> bool;
predicate arceos_ex_must_page_allocator_setup_hands_memblock_pages_to_buddy() -> bool;
predicate arceos_ex_must_page_allocator_buddy_lists_live_inside_page_allocator() -> bool;
predicate arceos_ex_must_page_allocator_buddy_use_zone_order_free_area() -> bool;
predicate arceos_ex_must_page_allocator_buddy_first_round_single_migratetype() -> bool;
predicate arceos_ex_must_page_allocator_buddy_split_memblock_free_ranges() -> bool;
predicate arceos_ex_must_page_allocator_buddy_use_page_metadata_nodes() -> bool;
predicate arceos_ex_must_page_allocator_buddy_use_intrusive_list() -> bool;
predicate arceos_ex_must_page_allocator_buddy_free_area_store_only_head_and_count() -> bool;
predicate arceos_ex_must_page_allocator_buddy_node_is_block_head_metadata() -> bool;
predicate arceos_ex_must_page_allocator_buddy_avoid_heap_storage() -> bool;
predicate arceos_ex_must_page_allocator_expose_linux_like_alloc_pages_api() -> bool;
predicate arceos_ex_must_page_allocator_alloc_pages_return_owned_linear_mapped_pageref() -> bool;
predicate arceos_ex_must_page_allocator_free_pages_match_alloc_order() -> bool;
predicate arceos_ex_must_page_allocator_smoke_cover_page_alloc_free_read_write() -> bool;
predicate arceos_ex_must_mm_core_init_model_sync_even_when_boot_lowering_elides_code() -> bool;
predicate arceos_ex_must_page_allocator_preset_record_zonelist_irqsave_protocol() -> bool;
predicate arceos_ex_must_page_allocator_runtime_locking_remain_explicit_deferred() -> bool;
predicate arceos_ex_must_memblock_disable_reaches_offline_not_destroyed() -> bool;
predicate arceos_ex_must_swiotlb_setup_before_memblock_disable() -> bool;
predicate arceos_ex_must_memory_debug_hardening_use_static_branch_registry() -> bool;
predicate arceos_ex_must_slub_subsystem_be_single_facade_not_cache_instance() -> bool;
predicate arceos_ex_must_slub_cache_type_name_be_slub_cache() -> bool;
predicate arceos_ex_must_slub_cache_registry_own_all_cache_instances() -> bool;
predicate arceos_ex_must_kmalloc_caches_reference_registered_slub_caches() -> bool;
predicate arceos_ex_must_kmem_cache_sites_register_named_slub_caches_or_defer() -> bool;
predicate arceos_ex_must_slub_bootstrap_before_kmalloc_caches_ready() -> bool;
predicate arceos_ex_must_slub_expose_kmalloc_kzalloc_kfree_api() -> bool;
predicate arceos_ex_must_slub_kmalloc_use_page_allocator_backing_pages() -> bool;
predicate arceos_ex_must_slub_kmalloc_use_fixed_size_classes() -> bool;
predicate arceos_ex_must_slub_kmalloc_use_slab_slot_freelist() -> bool;
predicate arceos_ex_must_slub_kzalloc_zero_returned_object() -> bool;
predicate arceos_ex_must_slub_kfree_recycle_object_to_cache() -> bool;
predicate arceos_ex_must_slub_first_round_defer_complex_linux_paths() -> bool;
predicate arceos_ex_must_slub_bootstrap_record_slab_mutex_boundary() -> bool;
predicate arceos_ex_must_slub_runtime_locking_remain_explicit_deferred() -> bool;
predicate arceos_ex_must_slub_smoke_cover_kmalloc_kzalloc_kfree() -> bool;
predicate arceos_ex_must_global_allocator_setup_after_slub_ready() -> bool;
predicate arceos_ex_must_global_allocator_implement_core_alloc_globalalloc() -> bool;
predicate arceos_ex_must_global_allocator_alloc_use_kmalloc() -> bool;
predicate arceos_ex_must_global_allocator_alloc_zeroed_use_kzalloc_or_zeroing() -> bool;
predicate arceos_ex_must_global_allocator_dealloc_recover_kmalloc_object_from_ptr() -> bool;
predicate arceos_ex_must_global_allocator_support_documented_layout_subset() -> bool;
predicate arceos_ex_must_dynamic_containers_require_global_allocator_ready() -> bool;
predicate arceos_ex_must_dynamic_container_smoke_cover_vec_growth_drop() -> bool;
predicate arceos_ex_must_dynamic_container_pressure_smoke_cover_layout_boundary() -> bool;
predicate arceos_ex_must_page_table_lock_cache_named_page_ptl() -> bool;
predicate arceos_ex_must_vmalloc_allocator_manage_vmap_addresses_and_execute_mappings() -> bool;
predicate arceos_ex_must_vmalloc_setup_build_all_vmap_subobjects() -> bool;
predicate arceos_ex_must_vmalloc_map_page_range_record_each_mapping_action() -> bool;
predicate arceos_ex_must_vmalloc_support_preallocated_windows_and_reject_duplicate_mapping() -> bool;
predicate arceos_ex_must_vmalloc_allocate_l0_windows_on_demand() -> bool;
predicate arceos_ex_must_vmalloc_unmap_before_free_vmap_area() -> bool;
predicate arceos_ex_must_vmalloc_sync_and_locking_contracts_remain_visible() -> bool;
predicate arceos_ex_must_ioremap_keep_physical_resource_and_mmio_policy_external_to_vmalloc() -> bool;
predicate arceos_ex_must_ioremap_iounmap_request_vmalloc_teardown_only() -> bool;
predicate arceos_ex_must_ioremap_model_mmio_attribute_policy_explicitly() -> bool;
predicate arceos_ex_must_mm_struct_cache_only_create_mm_struct_cache() -> bool;
predicate arceos_ex_must_entry_prelude_keep_early_alternatives_deferred() -> bool;
predicate arceos_ex_must_entry_successor_keep_start_kernel_deferred_facts() -> bool;
predicate arceos_ex_must_entry_successor_memblock_record_riscv_setup_bootmem_facts() -> bool;
predicate arceos_ex_must_entry_successor_keep_swapper_rwx_boundary_deferred() -> bool;
predicate arceos_ex_must_core_prepare_preserve_early_irq_and_smp_closed_facts() -> bool;
predicate arceos_ex_must_static_branch_setup_record_jump_label_guards() -> bool;
predicate arceos_ex_must_jump_label_mutex_be_independent_context_object() -> bool;
predicate arceos_ex_must_static_branch_setup_drive_jump_label_mutex_guard() -> bool;
predicate arceos_ex_must_static_branch_text_patch_sync_remain_deferred() -> bool;
predicate arceos_ex_must_resource_tree_setup_record_resource_lock_write_guard() -> bool;
predicate arceos_ex_must_resource_tree_setup_drive_resource_lock_write_guard() -> bool;
predicate arceos_ex_must_rwlock_expose_reader_writer_protocol() -> bool;
predicate arceos_ex_must_printk_buffer_setup_record_local_irq_save_restore() -> bool;
predicate arceos_ex_must_printk_buffer_setup_bind_local_irq_guard_to_boot_cpu_control() -> bool;
predicate arceos_ex_must_randomness_preset_not_unconditionally_lock_input_pool() -> bool;
predicate arceos_ex_must_randomness_conditional_reseed_use_base_crng_irqsave_lock() -> bool;
predicate arceos_ex_should_keep_mm_core_init_checkpoints_observable() -> bool;
predicate arceos_ex_must_startup_drive_boot_then_interrupt_then_payload() -> bool;
predicate arceos_ex_must_payload_require_interrupt_phase_ready() -> bool;
predicate arceos_ex_must_payload_follow_smp_runtime_not_nested_under_it() -> bool;
predicate arceos_ex_must_payload_require_finalize_ready() -> bool;
predicate arceos_ex_must_kernel_init_execution_line_reach_payload() -> bool;
predicate arceos_ex_must_user_boot_payload_be_selected_payload_variant() -> bool;
predicate arceos_ex_must_user_boot_use_existing_syscall_exception() -> bool;
predicate arceos_ex_must_not_generate_elf_loader_object() -> bool;
predicate arceos_ex_must_elf_object_setup_build_pt_load_mapping_plan() -> bool;
predicate arceos_ex_must_elf_object_support_pt_interp_without_elf_loader_object() -> bool;
predicate arceos_ex_must_elf_object_support_et_dyn_pie_main_with_fixed_bias() -> bool;
predicate arceos_ex_must_elf_object_defer_et_dyn_loader_main() -> bool;
predicate arceos_ex_must_user_address_space_map_main_and_interpreter_elfs() -> bool;
predicate arceos_ex_must_user_address_space_provide_dynamic_linker_heap_arena() -> bool;
predicate arceos_ex_must_user_stack_setup_initial_argc_argv_envp_auxv() -> bool;
predicate arceos_ex_must_user_stack_provide_dynamic_linker_auxv_fields() -> bool;
predicate arceos_ex_must_user_trap_frame_enter_interpreter_when_present() -> bool;
predicate arceos_ex_must_validate_user_init_elf_without_test_only_object_api() -> bool;
predicate arceos_ex_must_user_boot_payload_bind_kernel_init_task() -> bool;
predicate arceos_ex_must_first_user_address_space_bind_kernel_init_task() -> bool;
predicate arceos_ex_must_user_address_space_setup_consume_elf_load_plan() -> bool;
predicate arceos_ex_must_user_address_space_setup_materialize_pre_switch_backing() -> bool;
predicate arceos_ex_must_user_address_space_enable_build_real_page_table() -> bool;
predicate arceos_ex_must_user_address_space_enable_prepare_satp_without_switch() -> bool;
predicate arceos_ex_must_user_address_space_round_not_enter_user_mode() -> bool;
predicate arceos_ex_must_user_trap_frame_setup_prepare_but_not_sret() -> bool;
predicate arceos_ex_must_user_trap_frame_enable_fpu_initial_like_linux_start_thread() -> bool;
predicate arceos_ex_must_user_mode_entry_use_existing_trap_return_path() -> bool;
predicate arceos_ex_must_user_trap_entry_switch_to_kernel_stack() -> bool;
predicate arceos_ex_must_user_trap_entry_clear_live_fpu_vector_before_kernel_handling() -> bool;
predicate arceos_ex_must_user_syscall_dispatch_use_exception_stream_branch() -> bool;
predicate arceos_ex_must_not_generate_syscall_dispatcher_object() -> bool;
predicate arceos_ex_must_syscall_table_hold_concrete_syscall_actions() -> bool;
predicate arceos_ex_must_user_syscall_write_copy_from_user_address_space() -> bool;
predicate arceos_ex_must_user_syscall_fixed_usercopy_precheck_mapping_permissions() -> bool;
predicate arceos_ex_must_syscall_table_support_dynamic_linker_memory_actions() -> bool;
predicate arceos_ex_must_syscall_table_support_directory_openat_getdents64_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_getrandom_via_hwrng_core() -> bool;
predicate arceos_ex_must_user_init_process_carry_root_credentials_and_signal_mask() -> bool;
predicate arceos_ex_must_syscall_table_support_credentials_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_pid_uts_getcwd_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_getpgid_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_setpgid_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_setsid_eperm_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_getsid_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_child_setsid_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_rt_sigprocmask_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_rt_sigaction_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_time_read_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_stdin_ready_data_read_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_tiocgpgrp_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_tiocspgrp_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_tiocgsid_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_tiocsctty_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_mmap_fixed_prot_none_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_clone_plain_fork_first_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_wait4_parent_wait_first_slice() -> bool;
predicate arceos_ex_must_user_clone_deferred_boundaries_classify_out_of_slice_paths() -> bool;
predicate arceos_ex_must_user_syscall_error_probe_be_explicit_and_low_noise() -> bool;
predicate arceos_ex_must_user_read_trace_probe_be_explicit_and_side_effect_free() -> bool;
predicate arceos_ex_must_user_syscall_trace_probe_be_explicit_and_side_effect_free() -> bool;
predicate arceos_ex_must_syscall_table_support_fd_cloexec_fcntl_first_slice() -> bool;
predicate arceos_ex_must_newfstatat_support_cwd_dot_component_and_nofollow_first_slice() -> bool;
predicate arceos_ex_must_user_boot_emit_init_attempt_failure_trace() -> bool;
predicate arceos_ex_must_user_boot_kunit_observe_init_attempt_failure_trace() -> bool;
predicate arceos_ex_must_user_boot_extend_long_term_checkpoints_for_exec_debug() -> bool;
predicate arceos_ex_must_user_syscall_exit_stop_first_user_process() -> bool;
predicate arceos_ex_must_user_address_space_share_kernel_half_with_swapper_vm() -> bool;
predicate arceos_ex_must_keep_swapper_vm_single_kernel_shared_instance() -> bool;
predicate arceos_ex_must_user_boot_not_require_partition_objects_for_whole_disk_ext2() -> bool;
predicate arceos_ex_must_user_boot_harness_require_zero_user_exit_status() -> bool;
predicate arceos_ex_must_user_boot_harness_isolate_overlay_disk_images() -> bool;
predicate arceos_ex_must_action_lowering_use_context_ref_and_typed_packet() -> bool;
predicate arceos_ex_must_phase_boundary_guard_lower_to_context_contribution_only() -> bool;
predicate arceos_ex_must_preemption_guard_lower_to_counted_enter_exit() -> bool;
predicate arceos_ex_must_local_interrupt_guard_preserve_saved_flags() -> bool;
predicate arceos_ex_must_raw_spin_lock_irqsave_guard_lock_and_restore() -> bool;
predicate arceos_ex_must_raw_spin_lock_guard_use_plain_lock_unlock() -> bool;
predicate arceos_ex_must_bind_boot_scheduler_locks_to_owned_objects() -> bool;
predicate arceos_ex_must_effective_context_not_elide_protocol_guards() -> bool;
predicate arceos_ex_must_defer_rcu_lowering_until_primitive_exists() -> bool;
predicate arceos_ex_must_rcu_read_side_first_slice_remain_marked_incomplete() -> bool;
predicate arceos_ex_must_defer_full_rcu_read_side_lowering() -> bool;
predicate arceos_ex_must_sched_init_bit_wait_table_expose_bucket_waitqueue_heads() -> bool;
predicate arceos_ex_must_sched_init_bit_wait_table_use_within_context() -> bool;
predicate arceos_ex_must_sched_init_radix_maple_rcu_free_callbacks_deferred() -> bool;
predicate arceos_ex_must_sched_init_workqueue_register_pool_workqueue_cache() -> bool;
predicate arceos_ex_must_sched_init_workqueue_use_within_mutex_contexts() -> bool;
predicate arceos_ex_must_sched_init_workqueue_keep_worker_runtime_deferred() -> bool;
predicate arceos_ex_must_sched_init_softirq_prepare_shell_only() -> bool;
predicate arceos_ex_must_sched_init_rcu_register_rcu_softirq_explicitly() -> bool;
predicate arceos_ex_must_sched_init_rcu_expose_tree_and_tasks_init_facts() -> bool;
predicate arceos_ex_must_sched_init_trace_housekeeping_context_tracking_classify_by_config() -> bool;
predicate arceos_ex_must_scheduler_action_checkpoints_cover_pick_switch_and_schedule_exit() -> bool;
predicate arceos_ex_must_irq_time_init_model_path_under_interrupt_phase() -> bool;
predicate arceos_ex_must_irq_time_init_code_path_follow_interrupt_phase_tree() -> bool;
predicate arceos_ex_must_irq_time_init_keep_local_irq_closed() -> bool;
predicate arceos_ex_must_irq_time_init_record_riscv_irq_stack_setup() -> bool;
predicate arceos_ex_must_irq_time_init_record_timekeeper_irqsave_seqwrite() -> bool;
predicate arceos_ex_must_irq_time_init_classify_rcu_nohz_and_kfence_by_config() -> bool;
predicate arceos_ex_must_local_irq_enable_model_path_under_interrupt_phase() -> bool;
predicate arceos_ex_must_local_irq_enable_code_path_follow_interrupt_phase_tree() -> bool;
predicate arceos_ex_must_local_irq_enable_be_separate_interrupt_subphase() -> bool;
predicate arceos_ex_must_local_irq_enable_only_open_boot_cpu_local_gate() -> bool;
predicate arceos_ex_must_local_irq_enable_clear_early_flag_before_enabling_sie() -> bool;
predicate arceos_ex_must_local_irq_enable_have_no_within_context() -> bool;
predicate arceos_ex_must_local_irq_enable_keep_runtime_gates_deferred() -> bool;
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
predicate arceos_ex_must_irq_open_prepare_model_path_under_interrupt_phase() -> bool;
predicate arceos_ex_must_irq_open_prepare_code_path_follow_interrupt_phase_tree() -> bool;
predicate arceos_ex_must_irq_open_prepare_run_after_local_irq_enable() -> bool;
predicate arceos_ex_must_irq_open_prepare_keep_runtime_services_deferred() -> bool;
predicate arceos_ex_must_irq_open_prepare_keep_slub_ready_not_online() -> bool;
predicate arceos_ex_must_irq_open_prepare_console_prepared_only() -> bool;
predicate arceos_ex_must_irq_open_prepare_record_trimmed_paths_structurally() -> bool;
predicate arceos_ex_must_irq_open_prepare_sched_clock_record_local_irq_guard() -> bool;
predicate arceos_ex_must_irq_open_prepare_expose_sched_clock_and_delay_smoke_actions() -> bool;
predicate arceos_ex_must_process_prepare_model_path_under_interrupt_phase() -> bool;
predicate arceos_ex_must_process_prepare_code_path_follow_interrupt_phase_tree() -> bool;
predicate arceos_ex_must_process_prepare_run_after_irq_open_prepare() -> bool;
predicate arceos_ex_must_process_prepare_not_create_rest_init_tasks() -> bool;
predicate arceos_ex_must_process_prepare_keep_runtime_services_deferred() -> bool;
predicate arceos_ex_must_process_prepare_cover_pid_task_cred_memory_namespace_key_security_objects() -> bool;
predicate arceos_ex_must_task_creation_core_bind_task_entry_in_copy_process() -> bool;
predicate arceos_ex_must_task_creation_core_api_smoke_use_copy_process_contract() -> bool;
predicate arceos_ex_must_process_prepare_keep_deferred_paths_explicit() -> bool;
predicate arceos_ex_must_process_prepare_record_trimmed_paths_structurally() -> bool;
predicate arceos_ex_must_completion_map_to_reusable_object() -> bool;
predicate arceos_ex_must_completion_own_simple_wait_queue() -> bool;
predicate arceos_ex_must_completion_processes_keep_state_effects() -> bool;
predicate arceos_ex_must_completion_instances_drive_type_processes() -> bool;
predicate arceos_ex_must_completion_smoke_cover_setup_complete_and_token_flow() -> bool;
predicate arceos_ex_must_block_io_model_bio_buffer_head_before_ext2() -> bool;
predicate arceos_ex_must_block_io_registry_read_remain_lower_level_adapter() -> bool;
predicate arceos_ex_must_block_io_smoke_use_sb_bread_path() -> bool;
predicate arceos_ex_must_buffer_head_data_not_be_large_stack_storage() -> bool;
predicate arceos_ex_must_ext2_first_slice_read_only_and_buffer_head_based() -> bool;
predicate arceos_ex_must_ext2_support_4k_buffer_and_block_sizes() -> bool;
predicate arceos_ex_must_ext2_model_driver_volume_filesystem_lifecycle() -> bool;
predicate arceos_ex_must_ext2_read_path_support_multi_direct_blocks() -> bool;
predicate arceos_ex_must_ext2_read_path_support_single_indirect_blocks() -> bool;
predicate arceos_ex_must_ext2_smoke_use_stable_alpine_rootfs_files() -> bool;
predicate arceos_ex_must_rootfs_overlay_copy_fixture_outputs_at_image_build() -> bool;
predicate arceos_ex_must_rootfs_overlay_config_allow_none_and_target_overrides() -> bool;
predicate arceos_ex_must_rootfs_overlay_read_default_map_unless_disabled() -> bool;
predicate arceos_ex_must_rootfs_overlay_user_tests_live_under_tests_user() -> bool;
predicate arceos_ex_must_rootfs_overlay_user_tests_build_via_dedicated_makefile() -> bool;
predicate arceos_ex_must_rootfs_overlay_user_tests_select_toolchain_and_link_mode() -> bool;
predicate arceos_ex_must_rc_local_test_use_inittab_direct_marker_only() -> bool;
predicate arceos_ex_must_rc_local_difftest_be_default_case() -> bool;
predicate arceos_ex_must_rc_local_difftest_report_hard_scope_coverage_counts() -> bool;
predicate arceos_ex_must_user_probe_print_per_syscall_success_marker() -> bool;
predicate arceos_ex_must_user_probe_cover_directory_openat_getdents64() -> bool;
predicate arceos_ex_must_user_syscall_analysis_use_existing_static_tools() -> bool;
predicate arceos_ex_must_user_syscall_analysis_stay_out_of_default_build_path() -> bool;
predicate arceos_ex_must_user_syscall_analysis_mark_busybox_candidates_conservative() -> bool;
predicate arceos_ex_must_user_syscall_vfs_specs_reference_linux_6_12() -> bool;
predicate arceos_ex_must_disk_build_default_not_rebuild_existing_image() -> bool;
predicate arceos_ex_must_ext2_lookup_support_path_components_from_directories() -> bool;
predicate arceos_ex_must_ext2_support_minimal_vfs_read_only_mount() -> bool;
predicate arceos_ex_must_vfs_support_minimal_absolute_path_walk_and_read() -> bool;
predicate arceos_ex_must_long_term_checkpoints_follow_model_coding_contracts() -> bool;
predicate arceos_ex_must_payload_vfs_ext2_read_emit_observation_checkpoints() -> bool;
predicate arceos_ex_must_block_io_task_wait_checkpoints_cover_submit_wait_and_timeout() -> bool;
predicate arceos_ex_must_block_io_irq_completion_checkpoints_cover_begin_end_failure() -> bool;
predicate arceos_ex_must_block_io_completion_source_distinguish_irq_and_task_poll() -> bool;
predicate arceos_ex_must_virtio_blk_sync_reads_submit_wait_complete_before_return() -> bool;
predicate arceos_ex_must_virtio_blk_initcall_superblock_probe_converge_before_ready() -> bool;
predicate arceos_ex_must_virtio_blk_completion_consumer_be_single_owner() -> bool;
predicate arceos_ex_must_virtqueue_publish_avail_before_notify_with_release_order() -> bool;
predicate arceos_ex_must_virtqueue_observe_used_with_acquire_order() -> bool;
predicate arceos_ex_must_read_path_error_classification_checkpoint_be_structured() -> bool;
predicate arceos_ex_must_rootfs_move_ext2_mount_and_chroot_dot_as_separate_actions() -> bool;
predicate arceos_ex_must_rootfs_classify_prepare_namespace_paths() -> bool;
predicate arceos_ex_must_ext2_defer_page_cache_indirect_and_writes() -> bool;
predicate arceos_ex_must_rest_init_model_path_under_up_multitask_phase() -> bool;
predicate arceos_ex_must_not_model_rest_init_phase_wrapper() -> bool;
predicate arceos_ex_must_rest_init_code_path_follow_up_multitask_phase_tree() -> bool;
predicate arceos_ex_must_rest_init_run_after_process_prepare() -> bool;
predicate arceos_ex_must_rest_init_create_kernel_init_and_kthreadd_facts() -> bool;
predicate arceos_ex_must_rest_init_create_tasks_with_explicit_entries() -> bool;
predicate arceos_ex_must_kthreadd_entry_model_minimal_schedule_loop() -> bool;
predicate arceos_ex_must_rest_init_publish_system_scheduling() -> bool;
predicate arceos_ex_must_rest_init_complete_kthreadd_ready_gate() -> bool;
predicate arceos_ex_must_kernel_init_wait_observe_kthreadd_ready_gate() -> bool;
predicate arceos_ex_must_rest_init_publish_scheduler_dispatch_facts() -> bool;
predicate arceos_ex_must_boot_idle_runtime_split_entry_and_loop_actions() -> bool;
predicate arceos_ex_must_boot_idle_runtime_model_representative_need_resched_cycle() -> bool;
predicate arceos_ex_must_bind_boot_idle_schedule_if_need_resched_to_schedule_idle() -> bool;
predicate arceos_ex_must_rest_init_setup_show_boot_idle_tail_chain() -> bool;
predicate arceos_ex_must_rest_init_smoke_cover_idle_schedule_relations() -> bool;
predicate arceos_ex_must_model_cpu_instances_with_unified_cpu_type() -> bool;
predicate arceos_ex_must_cpu_group_index_cpu_refs_by_logical_id() -> bool;
predicate arceos_ex_must_cpu_group_not_own_cpu_bodies() -> bool;
predicate arceos_ex_must_cpu_state_be_instance_facts_and_group_set_views() -> bool;
predicate arceos_ex_must_initcall_classify_pre_do_initcalls_by_config() -> bool;
predicate arceos_ex_must_initcall_record_driver_init_deferred_sync_primitives() -> bool;
predicate arceos_ex_must_initcall_dispatcher_remain_entry_agnostic() -> bool;
predicate arceos_ex_must_initcall_record_do_one_initcall_context_repair() -> bool;
predicate arceos_ex_must_initcall_same_level_order_independence_defer_to_proof_or_nightly() -> bool;
predicate arceos_ex_must_not_generate_live_ap_current_cpu_before_entry() -> bool;
predicate arceos_ex_must_cpu_group_lowering_mark_boot_secondary_split_transitional() -> bool;
predicate arceos_ex_must_cpu_group_smoke_cover_index_and_sets() -> bool;
predicate arceos_ex_must_current_task_ref_be_cpu_view_private() -> bool;
predicate arceos_ex_must_current_runqueue_ref_be_cpu_view_private() -> bool;
predicate arceos_ex_must_current_runqueue_ref_api_smoke_use_formal_runqueue_actions() -> bool;
predicate arceos_ex_must_scheduler_schedule_smoke_use_payload_cooperative_switch() -> bool;
predicate arceos_ex_must_wakeup_set_task_cpu_between_select_and_enqueue() -> bool;
predicate arceos_ex_must_rest_init_pin_kernel_init_as_task_action() -> bool;
predicate arceos_ex_must_rest_init_not_make_pre_smp_depend_on_up_multitask_wrapper() -> bool;
predicate arceos_ex_must_rest_init_keep_true_task_switching_deferred() -> bool;
predicate arceos_ex_must_rest_init_keep_smp_and_later_runtime_deferred() -> bool;
predicate arceos_ex_must_pre_smp_init_model_path_under_smp_runtime_phase() -> bool;
predicate arceos_ex_must_pre_smp_init_code_path_follow_smp_runtime_phase_tree() -> bool;
predicate arceos_ex_must_pre_smp_init_run_from_scheduler_dispatch_facts() -> bool;
predicate arceos_ex_must_pre_smp_init_consume_kernel_init_entry_contract() -> bool;
predicate arceos_ex_must_pre_smp_init_open_full_gfp_and_prepare_topology() -> bool;
predicate arceos_ex_must_pre_smp_init_setup_workqueue_vmstat_tasks_rcu_and_initcalls() -> bool;
predicate arceos_ex_must_pre_smp_init_workqueue_init_use_pool_mutex_context() -> bool;
predicate arceos_ex_must_pre_smp_init_stop_before_smp_init() -> bool;
predicate arceos_ex_must_smp_bringup_model_path_under_smp_runtime_phase() -> bool;
predicate arceos_ex_must_smp_bringup_code_path_follow_smp_runtime_phase_tree() -> bool;
predicate arceos_ex_must_smp_bringup_focus_bp_side_flow() -> bool;
predicate arceos_ex_must_smp_bringup_split_bp_and_ap_phase_lines() -> bool;
predicate arceos_ex_must_smp_bringup_prepare_per_ap_idle_task_and_stack() -> bool;
predicate arceos_ex_must_smp_bringup_use_sbi_hsm_hart_start() -> bool;
predicate arceos_ex_must_smp_bringup_generate_ap_entry_prelude_callin_online_idle_phases() -> bool;
predicate arceos_ex_must_smp_bringup_online_only_after_ap_done_up_ack() -> bool;
predicate arceos_ex_must_smp_bringup_checkpoint_ap_subphases() -> bool;
predicate arceos_ex_must_smp_bringup_keep_hotplug_callbacks_deferred() -> bool;
predicate arceos_ex_must_smp_bringup_keep_bp_ap_sync_explicit() -> bool;
predicate arceos_ex_must_smp_bringup_preserve_hotplug_guards() -> bool;
predicate arceos_ex_must_smp_bringup_preserve_completion_wait_locks() -> bool;
predicate arceos_ex_must_smp_bringup_preserve_sbi_boot_data_ordering() -> bool;
predicate arceos_ex_must_smp_bringup_record_ap_local_sync_summary() -> bool;
predicate arceos_ex_must_smp_bringup_make_secondary_cpus_online() -> bool;
predicate arceos_ex_must_smp_bringup_keep_ap_internals_deferred() -> bool;
predicate arceos_ex_must_smp_runtime_keep_later_subphases_deferred() -> bool;
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

type ArceosExDeviceTreeCodingMust {
    invariant {
        /* MemBlock allocation. */
        arceos_ex_must_device_tree_setup_allocates_from_memblock();

        /* Two-pass unflatten. */
        arceos_ex_must_device_tree_unflatten_uses_two_passes();

        /* Established mapping. */
        arceos_ex_must_device_tree_storage_uses_established_linear_mapping();

        /* No heap or fixed static substitute. */
        arceos_ex_must_device_tree_avoid_heap_and_fixed_static_storage();

        /* Checkpoint after validation. */
        arceos_ex_must_device_tree_checkpoint_after_validation();
    }
}

type ArceosExCorePrepareCodingMust {
    invariant {
        /* CorePrepare concurrency boundary. */
        arceos_ex_must_core_prepare_preserve_early_irq_and_smp_closed_facts();

        /* StaticBranch.setup guard facts. */
        arceos_ex_must_static_branch_setup_record_jump_label_guards();

        /* JumpLabelMutex object mapping. */
        arceos_ex_must_jump_label_mutex_be_independent_context_object();

        /* CpuHotplugLock object mapping. */
        arceos_ex_must_cpu_hotplug_lock_be_independent_percpu_rwsem_object();

        /* StaticBranch jump-label mutex guard lowering. */
        arceos_ex_must_static_branch_setup_drive_jump_label_mutex_guard();

        /* StaticBranch CPU hotplug read guard lowering. */
        arceos_ex_must_static_branch_setup_drive_cpu_hotplug_read_guard();

        /* PerCpuRwSemaphore observable behavior. */
        arceos_ex_must_percpu_rwsem_expose_reader_writer_protocol();

        /* Text patch synchronization boundary. */
        arceos_ex_must_static_branch_text_patch_sync_remain_deferred();

        /* ResourceTree setup guard. */
        arceos_ex_must_resource_tree_setup_record_resource_lock_write_guard();

        /* ResourceTree resource_lock guard lowering. */
        arceos_ex_must_resource_tree_setup_drive_resource_lock_write_guard();

        /* RwLock observable behavior. */
        arceos_ex_must_rwlock_expose_reader_writer_protocol();

        /* PrintkBuffer setup IRQ guard. */
        arceos_ex_must_printk_buffer_setup_record_local_irq_save_restore();
        arceos_ex_must_printk_buffer_setup_bind_local_irq_guard_to_boot_cpu_control();

        /* Randomness preset conditional lock boundary. */
        arceos_ex_must_randomness_preset_not_unconditionally_lock_input_pool();

        /* Randomness conditional reseed/credit lock boundary. */
        arceos_ex_must_randomness_conditional_reseed_use_base_crng_irqsave_lock();
    }
}

type ArceosExEffectiveContextCodingMust {
    invariant {
        /* Natural phase-boundary guard. */
        arceos_ex_must_phase_boundary_guard_lower_to_context_contribution_only();

        /* PreemptionControl guard boundary. */
        arceos_ex_must_preemption_guard_lower_to_counted_enter_exit();

        /* LocalInterruptControl guard boundary. */
        arceos_ex_must_local_interrupt_guard_preserve_saved_flags();

        /* RawSpinLock irq-save guard boundary. */
        arceos_ex_must_raw_spin_lock_irqsave_guard_lock_and_restore();

        /* RawSpinLock ordinary guard boundary. */
        arceos_ex_must_raw_spin_lock_guard_use_plain_lock_unlock();

        /* Effective Context is not a license to erase protocol guards. */
        arceos_ex_must_effective_context_not_elide_protocol_guards();

        /* RCU read-side first slice. */
        arceos_ex_must_rcu_read_side_first_slice_remain_marked_incomplete();
        arceos_ex_must_defer_full_rcu_read_side_lowering();
    }
}

type ArceosExDeviceTreeCodingShould {
    invariant {
        /* Unsafe encapsulation. */
        arceos_ex_should_encapsulate_device_tree_unflatten_unsafe();
    }
}

type ArceosExMmCoreInitCodingMust {
    invariant {
        /* Memory topology view. */
        arceos_ex_must_memory_topology_project_existing_zones_only();

        /* PageAllocator preset/setup split. */
        arceos_ex_must_page_allocator_preset_only_builds_topology_and_hooks();

        /* MemBlock handoff. */
        arceos_ex_must_page_allocator_setup_hands_memblock_pages_to_buddy();

        /* Minimal buddy free lists. */
        arceos_ex_must_page_allocator_buddy_lists_live_inside_page_allocator();
        arceos_ex_must_page_allocator_buddy_use_zone_order_free_area();
        arceos_ex_must_page_allocator_buddy_first_round_single_migratetype();

        /* MemBlock range splitting. */
        arceos_ex_must_page_allocator_buddy_split_memblock_free_ranges();

        /* Metadata-backed free nodes. */
        arceos_ex_must_page_allocator_buddy_use_page_metadata_nodes();
        arceos_ex_must_page_allocator_buddy_use_intrusive_list();
        arceos_ex_must_page_allocator_buddy_free_area_store_only_head_and_count();
        arceos_ex_must_page_allocator_buddy_node_is_block_head_metadata();
        arceos_ex_must_page_allocator_buddy_avoid_heap_storage();

        /* Linux-like buddy API. */
        arceos_ex_must_page_allocator_expose_linux_like_alloc_pages_api();

        /* PageRef contract. */
        arceos_ex_must_page_allocator_alloc_pages_return_owned_linear_mapped_pageref();

        /* Free order contract. */
        arceos_ex_must_page_allocator_free_pages_match_alloc_order();

        /* mm_core_init synchronization surface. */
        arceos_ex_must_mm_core_init_model_sync_even_when_boot_lowering_elides_code();

        /* Zonelist update protocol. */
        arceos_ex_must_page_allocator_preset_record_zonelist_irqsave_protocol();

        /* Page allocator runtime locking. */
        arceos_ex_must_page_allocator_runtime_locking_remain_explicit_deferred();

        /* Page allocator smoke. */
        arceos_ex_must_page_allocator_smoke_cover_page_alloc_free_read_write();

        /* MemBlock remains Offline. */
        arceos_ex_must_memblock_disable_reaches_offline_not_destroyed();

        /* SWIOTLB ordering. */
        arceos_ex_must_swiotlb_setup_before_memblock_disable();

        /* Static branch policy. */
        arceos_ex_must_memory_debug_hardening_use_static_branch_registry();

        /* SLUB object hierarchy. */
        arceos_ex_must_slub_subsystem_be_single_facade_not_cache_instance();
        arceos_ex_must_slub_cache_type_name_be_slub_cache();
        arceos_ex_must_slub_cache_registry_own_all_cache_instances();
        arceos_ex_must_kmalloc_caches_reference_registered_slub_caches();
        arceos_ex_must_kmem_cache_sites_register_named_slub_caches_or_defer();

        /* SLUB bootstrap. */
        arceos_ex_must_slub_bootstrap_before_kmalloc_caches_ready();

        /* Linux-like kmalloc API. */
        arceos_ex_must_slub_expose_kmalloc_kzalloc_kfree_api();
        arceos_ex_must_slub_kmalloc_use_page_allocator_backing_pages();
        arceos_ex_must_slub_kmalloc_use_fixed_size_classes();
        arceos_ex_must_slub_kmalloc_use_slab_slot_freelist();
        arceos_ex_must_slub_kzalloc_zero_returned_object();
        arceos_ex_must_slub_kfree_recycle_object_to_cache();
        arceos_ex_must_slub_first_round_defer_complex_linux_paths();

        /* SLUB bootstrap synchronization. */
        arceos_ex_must_slub_bootstrap_record_slab_mutex_boundary();
        arceos_ex_must_slub_runtime_locking_remain_explicit_deferred();

        /* SLUB/kmalloc smoke. */
        arceos_ex_must_slub_smoke_cover_kmalloc_kzalloc_kfree();

        /* Global allocator. */
        arceos_ex_must_global_allocator_setup_after_slub_ready();
        arceos_ex_must_global_allocator_implement_core_alloc_globalalloc();
        arceos_ex_must_global_allocator_alloc_use_kmalloc();
        arceos_ex_must_global_allocator_alloc_zeroed_use_kzalloc_or_zeroing();
        arceos_ex_must_global_allocator_dealloc_recover_kmalloc_object_from_ptr();
        arceos_ex_must_global_allocator_support_documented_layout_subset();
        arceos_ex_must_dynamic_containers_require_global_allocator_ready();

        /* Dynamic container smoke. */
        arceos_ex_must_dynamic_container_smoke_cover_vec_growth_drop();
        arceos_ex_must_dynamic_container_pressure_smoke_cover_layout_boundary();

        /* Page table lock cache. */
        arceos_ex_must_page_table_lock_cache_named_page_ptl();

        /* Vmalloc boundary. */
        arceos_ex_must_vmalloc_allocator_manage_vmap_addresses_and_execute_mappings();

        /* Vmap subobjects. */
        arceos_ex_must_vmalloc_setup_build_all_vmap_subobjects();

        /* Mapping record boundary. */
        arceos_ex_must_vmalloc_map_page_range_record_each_mapping_action();

        /* Runtime mapping window pool. */
        arceos_ex_must_vmalloc_support_preallocated_windows_and_reject_duplicate_mapping();
        arceos_ex_must_vmalloc_allocate_l0_windows_on_demand();

        /* Vmalloc synchronization contracts. */
        arceos_ex_must_vmalloc_sync_and_locking_contracts_remain_visible();

        /* Unmap/free boundary. */
        arceos_ex_must_vmalloc_unmap_before_free_vmap_area();

        /* Ioremap/vmalloc split. */
        arceos_ex_must_ioremap_keep_physical_resource_and_mmio_policy_external_to_vmalloc();

        /* Iounmap boundary. */
        arceos_ex_must_ioremap_iounmap_request_vmalloc_teardown_only();

        /* MMIO attributes. */
        arceos_ex_must_ioremap_model_mmio_attribute_policy_explicitly();

        /* mm_struct only. */
        arceos_ex_must_mm_struct_cache_only_create_mm_struct_cache();
    }
}

type ArceosExMmCoreInitCodingShould {
    invariant {
        /* Checkpoints. */
        arceos_ex_should_keep_mm_core_init_checkpoints_observable();
    }
}

type ArceosExLocalIrqEnableCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_local_irq_enable_model_path_under_interrupt_phase();

        /* Code path. */
        arceos_ex_must_local_irq_enable_code_path_follow_interrupt_phase_tree();

        /* Separate subphase. */
        arceos_ex_must_local_irq_enable_be_separate_interrupt_subphase();

        /* Scope. */
        arceos_ex_must_local_irq_enable_only_open_boot_cpu_local_gate();

        /* Linux ordering. */
        arceos_ex_must_local_irq_enable_clear_early_flag_before_enabling_sie();

        /* Context. */
        arceos_ex_must_local_irq_enable_have_no_within_context();

        /* Deferred runtime gates. */
        arceos_ex_must_local_irq_enable_keep_runtime_gates_deferred();
    }
}

type ArceosExEntryPreludeCodingMust {
    invariant {
        /* RISC-V early alternatives boundary. */
        arceos_ex_must_entry_prelude_keep_early_alternatives_deferred();
    }
}

type ArceosExEntrySuccessorCodingMust {
    invariant {
        /* start_kernel() deferred calls. */
        arceos_ex_must_entry_successor_keep_start_kernel_deferred_facts();

        /* RISC-V setup_bootmem() facts. */
        arceos_ex_must_entry_successor_memblock_record_riscv_setup_bootmem_facts();

        /* SwapperVm permission boundary. */
        arceos_ex_must_entry_successor_keep_swapper_rwx_boundary_deferred();
    }
}

type ArceosExStartupPhaseCodingMust {
    invariant {
        /* Startup phase order. */
        arceos_ex_must_startup_drive_boot_then_interrupt_then_payload();

        /* Payload dependency. */
        arceos_ex_must_payload_require_interrupt_phase_ready();

        /* Payload placement. */
        arceos_ex_must_payload_follow_smp_runtime_not_nested_under_it();

        /* Finalize handoff. */
        arceos_ex_must_payload_require_finalize_ready();

        /* KernelInitTask execution line. */
        arceos_ex_must_kernel_init_execution_line_reach_payload();

        /* UserBootPayload selected variant. */
        arceos_ex_must_user_boot_payload_be_selected_payload_variant();

        /* Syscall ownership. */
        arceos_ex_must_user_boot_use_existing_syscall_exception();

        /* ELF object boundary. */
        arceos_ex_must_not_generate_elf_loader_object();
        arceos_ex_must_elf_object_setup_build_pt_load_mapping_plan();
        arceos_ex_must_elf_object_support_pt_interp_without_elf_loader_object();
        arceos_ex_must_elf_object_support_et_dyn_pie_main_with_fixed_bias();
        arceos_ex_must_elf_object_defer_et_dyn_loader_main();

        /* User init ELF validation boundary. */
        arceos_ex_must_validate_user_init_elf_without_test_only_object_api();

        /* User address-space boundary. */
        arceos_ex_must_user_boot_payload_bind_kernel_init_task();
        arceos_ex_must_first_user_address_space_bind_kernel_init_task();
        arceos_ex_must_user_address_space_setup_consume_elf_load_plan();
        arceos_ex_must_user_address_space_map_main_and_interpreter_elfs();
        arceos_ex_must_user_address_space_provide_dynamic_linker_heap_arena();
        arceos_ex_must_user_address_space_setup_materialize_pre_switch_backing();
        arceos_ex_must_user_address_space_enable_build_real_page_table();
        arceos_ex_must_user_address_space_enable_prepare_satp_without_switch();
        arceos_ex_must_user_address_space_round_not_enter_user_mode();
        arceos_ex_must_user_stack_setup_initial_argc_argv_envp_auxv();
        arceos_ex_must_user_stack_provide_dynamic_linker_auxv_fields();
        arceos_ex_must_user_trap_frame_setup_prepare_but_not_sret();
        arceos_ex_must_user_trap_frame_enable_fpu_initial_like_linux_start_thread();
        arceos_ex_must_user_trap_frame_enter_interpreter_when_present();
        arceos_ex_must_user_mode_entry_use_existing_trap_return_path();
        arceos_ex_must_user_trap_entry_switch_to_kernel_stack();
        arceos_ex_must_user_trap_entry_clear_live_fpu_vector_before_kernel_handling();
        arceos_ex_must_user_syscall_dispatch_use_exception_stream_branch();
        arceos_ex_must_not_generate_syscall_dispatcher_object();
        arceos_ex_must_syscall_table_hold_concrete_syscall_actions();
        arceos_ex_must_user_syscall_write_copy_from_user_address_space();
        arceos_ex_must_user_syscall_fixed_usercopy_precheck_mapping_permissions();
        arceos_ex_must_syscall_table_support_dynamic_linker_memory_actions();
        arceos_ex_must_syscall_table_support_directory_openat_getdents64_slice();
        arceos_ex_must_syscall_table_support_getrandom_via_hwrng_core();
        arceos_ex_must_user_init_process_carry_root_credentials_and_signal_mask();
        arceos_ex_must_syscall_table_support_credentials_first_slice();
        arceos_ex_must_syscall_table_support_pid_uts_getcwd_first_slice();
        arceos_ex_must_syscall_table_support_getpgid_first_slice();
        arceos_ex_must_syscall_table_support_setpgid_first_slice();
        arceos_ex_must_syscall_table_support_setsid_eperm_first_slice();
        arceos_ex_must_syscall_table_support_getsid_first_slice();
        arceos_ex_must_syscall_table_support_child_setsid_first_slice();
        arceos_ex_must_syscall_table_support_rt_sigprocmask_first_slice();
        arceos_ex_must_syscall_table_support_rt_sigaction_first_slice();
        arceos_ex_must_syscall_table_support_time_read_first_slice();
        arceos_ex_must_syscall_table_support_stdin_ready_data_read_first_slice();
        arceos_ex_must_syscall_table_support_tiocgpgrp_first_slice();
        arceos_ex_must_syscall_table_support_tiocspgrp_first_slice();
        arceos_ex_must_syscall_table_support_tiocgsid_first_slice();
        arceos_ex_must_syscall_table_support_tiocsctty_first_slice();
        arceos_ex_must_syscall_table_support_mmap_fixed_prot_none_first_slice();
        arceos_ex_must_syscall_table_support_clone_plain_fork_first_slice();
        arceos_ex_must_syscall_table_support_wait4_parent_wait_first_slice();
        arceos_ex_must_user_clone_deferred_boundaries_classify_out_of_slice_paths();
        arceos_ex_must_user_syscall_error_probe_be_explicit_and_low_noise();
        arceos_ex_must_user_read_trace_probe_be_explicit_and_side_effect_free();
        arceos_ex_must_user_syscall_trace_probe_be_explicit_and_side_effect_free();
        arceos_ex_must_syscall_table_support_fd_cloexec_fcntl_first_slice();
        arceos_ex_must_newfstatat_support_cwd_dot_component_and_nofollow_first_slice();
        arceos_ex_must_user_boot_extend_long_term_checkpoints_for_exec_debug();
        arceos_ex_must_user_syscall_exit_stop_first_user_process();
        arceos_ex_must_user_address_space_share_kernel_half_with_swapper_vm();
        arceos_ex_must_keep_swapper_vm_single_kernel_shared_instance();
        arceos_ex_must_user_boot_harness_require_zero_user_exit_status();
        arceos_ex_must_user_boot_harness_isolate_overlay_disk_images();

        /* Whole-disk ext2 input. */
        arceos_ex_must_user_boot_not_require_partition_objects_for_whole_disk_ext2();
    }
}

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

type ArceosExIrqOpenPrepareCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_irq_open_prepare_model_path_under_interrupt_phase();

        /* Code path. */
        arceos_ex_must_irq_open_prepare_code_path_follow_interrupt_phase_tree();

        /* Ordering. */
        arceos_ex_must_irq_open_prepare_run_after_local_irq_enable();

        /* Runtime services. */
        arceos_ex_must_irq_open_prepare_keep_runtime_services_deferred();

        /* SLUB late boundary. */
        arceos_ex_must_irq_open_prepare_keep_slub_ready_not_online();

        /* Console boundary. */
        arceos_ex_must_irq_open_prepare_console_prepared_only();

        /* Trimmed/deferred paths. */
        arceos_ex_must_irq_open_prepare_record_trimmed_paths_structurally();

        /* Sched clock local IRQ guard. */
        arceos_ex_must_irq_open_prepare_sched_clock_record_local_irq_guard();

        /* Smoke actions. */
        arceos_ex_must_irq_open_prepare_expose_sched_clock_and_delay_smoke_actions();
    }
}

type ArceosExProcessPrepareCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_process_prepare_model_path_under_interrupt_phase();

        /* Code path. */
        arceos_ex_must_process_prepare_code_path_follow_interrupt_phase_tree();

        /* Ordering. */
        arceos_ex_must_process_prepare_run_after_irq_open_prepare();

        /* rest_init boundary. */
        arceos_ex_must_process_prepare_not_create_rest_init_tasks();

        /* Runtime services. */
        arceos_ex_must_process_prepare_keep_runtime_services_deferred();

        /* Object coverage. */
        arceos_ex_must_process_prepare_cover_pid_task_cred_memory_namespace_key_security_objects();

        /* Task entry creation contract. */
        arceos_ex_must_task_creation_core_bind_task_entry_in_copy_process();

        /* TaskCreationCore API smoke. */
        arceos_ex_must_task_creation_core_api_smoke_use_copy_process_contract();

        /* Deferred paths. */
        arceos_ex_must_process_prepare_keep_deferred_paths_explicit();

        /* Trimmed/deferred path carrier. */
        arceos_ex_must_process_prepare_record_trimmed_paths_structurally();
    }
}

type ArceosExCompletionCodingMust {
    invariant {
        /* Reusable object. */
        arceos_ex_must_completion_map_to_reusable_object();

        /* Owned wait queue. */
        arceos_ex_must_completion_own_simple_wait_queue();

        /* Event/action boundary. */
        arceos_ex_must_completion_processes_keep_state_effects();

        /* Instance inheritance. */
        arceos_ex_must_completion_instances_drive_type_processes();

        /* Smoke coverage. */
        arceos_ex_must_completion_smoke_cover_setup_complete_and_token_flow();
    }
}

type ArceosExBlockIoCodingMust {
    invariant {
        /* Linux-like block I/O adapter. */
        arceos_ex_must_block_io_model_bio_buffer_head_before_ext2();

        /* Registry read role. */
        arceos_ex_must_block_io_registry_read_remain_lower_level_adapter();

        /* Smoke entry. */
        arceos_ex_must_block_io_smoke_use_sb_bread_path();

        /* BufferHead storage. */
        arceos_ex_must_buffer_head_data_not_be_large_stack_storage();

        /* Read-only ext2 first slice. */
        arceos_ex_must_ext2_first_slice_read_only_and_buffer_head_based();

        /* Ext2 object model. */
        arceos_ex_must_ext2_model_driver_volume_filesystem_lifecycle();

        /* 4K Buffer / ext2 block-size support. */
        arceos_ex_must_ext2_support_4k_buffer_and_block_sizes();

        /* Direct-block read path generalization. */
        arceos_ex_must_ext2_read_path_support_multi_direct_blocks();
        arceos_ex_must_ext2_read_path_support_single_indirect_blocks();

        /* Stable Alpine smoke targets. */
        arceos_ex_must_ext2_smoke_use_stable_alpine_rootfs_files();

        /* Build-time rootfs overlay. */
        arceos_ex_must_rootfs_overlay_copy_fixture_outputs_at_image_build();
        arceos_ex_must_rootfs_overlay_config_allow_none_and_target_overrides();
        arceos_ex_must_rootfs_overlay_read_default_map_unless_disabled();
        arceos_ex_must_rootfs_overlay_user_tests_live_under_tests_user();
        arceos_ex_must_rootfs_overlay_user_tests_build_via_dedicated_makefile();
        arceos_ex_must_rootfs_overlay_user_tests_select_toolchain_and_link_mode();
        arceos_ex_must_user_probe_print_per_syscall_success_marker();
        arceos_ex_must_user_probe_cover_directory_openat_getdents64();
        arceos_ex_must_user_syscall_analysis_use_existing_static_tools();
        arceos_ex_must_user_syscall_analysis_stay_out_of_default_build_path();
        arceos_ex_must_user_syscall_analysis_mark_busybox_candidates_conservative();
        arceos_ex_must_user_syscall_vfs_specs_reference_linux_6_12();
        arceos_ex_must_disk_build_default_not_rebuild_existing_image();
        arceos_ex_must_qemu_append_default_user_boot_to_bin_sh_and_passthrough();
        arceos_ex_must_test_harness_pin_user_smoke_qemu_append();
        arceos_ex_must_test_harness_cover_no_overlay_bin_ls();
        arceos_ex_must_rootfs_file_overlay_apply_after_fixture_overlay();
        arceos_ex_must_rc_local_test_use_inittab_direct_marker_only();
        arceos_ex_must_rc_local_difftest_be_default_case();
        arceos_ex_must_rc_local_difftest_report_hard_scope_coverage_counts();
        arceos_ex_must_openrc_login_test_use_explicit_account_overlay();
        arceos_ex_must_test_harness_cover_no_overlay_bin_sh_with_host_input();
        arceos_ex_must_keep_shell_external_commands_and_native_init_diagnostic_until_specified();
        arceos_ex_must_keep_overlay_as_fixture_injection_after_init_cmdline_support();

        /* Directory path lookup. */
        arceos_ex_must_ext2_lookup_support_path_components_from_directories();

        /* Minimal VFS read-only mount. */
        arceos_ex_must_ext2_support_minimal_vfs_read_only_mount();

        /* Minimal pathname walk/read. */
        arceos_ex_must_vfs_support_minimal_absolute_path_walk_and_read();

        /* Long-term observation checkpoints. */
        arceos_ex_must_long_term_checkpoints_follow_model_coding_contracts();
        arceos_ex_must_payload_vfs_ext2_read_emit_observation_checkpoints();
        arceos_ex_must_block_io_task_wait_checkpoints_cover_submit_wait_and_timeout();
        arceos_ex_must_block_io_irq_completion_checkpoints_cover_begin_end_failure();
        arceos_ex_must_block_io_completion_source_distinguish_irq_and_task_poll();
        /* Virtio-blk synchronous request lifecycle. */
        arceos_ex_must_virtio_blk_sync_reads_submit_wait_complete_before_return();

        /* Initcall superblock probe convergence. */
        arceos_ex_must_virtio_blk_initcall_superblock_probe_converge_before_ready();

        /* Single completion consumer. */
        arceos_ex_must_virtio_blk_completion_consumer_be_single_owner();

        /* Virtqueue memory ordering. */
        arceos_ex_must_virtqueue_publish_avail_before_notify_with_release_order();
        arceos_ex_must_virtqueue_observe_used_with_acquire_order();
        arceos_ex_must_read_path_error_classification_checkpoint_be_structured();

        /* Deferred ext2 scope. */
        arceos_ex_must_ext2_defer_page_cache_indirect_and_writes();
    }
}

type ArceosExRestInitCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_rest_init_model_path_under_up_multitask_phase();
        arceos_ex_must_not_model_rest_init_phase_wrapper();

        /* Code path. */
        arceos_ex_must_rest_init_code_path_follow_up_multitask_phase_tree();

        /* Ordering. */
        arceos_ex_must_rest_init_run_after_process_prepare();

        /* Task creation facts. */
        arceos_ex_must_rest_init_create_kernel_init_and_kthreadd_facts();

        /* Explicit task entries. */
        arceos_ex_must_rest_init_create_tasks_with_explicit_entries();

        /* Kthreadd entry loop. */
        arceos_ex_must_kthreadd_entry_model_minimal_schedule_loop();

        /* System state. */
        arceos_ex_must_rest_init_publish_system_scheduling();

        /* Completion. */
        arceos_ex_must_rest_init_complete_kthreadd_ready_gate();

        /* Scheduler dispatch facts. */
        arceos_ex_must_rest_init_publish_scheduler_dispatch_facts();

        /* Boot idle runtime actions. */
        arceos_ex_must_boot_idle_runtime_split_entry_and_loop_actions();

        /* Representative need_resched idle cycle. */
        arceos_ex_must_boot_idle_runtime_model_representative_need_resched_cycle();

        /* schedule_idle wrapper. */
        arceos_ex_must_bind_boot_idle_schedule_if_need_resched_to_schedule_idle();

        /* Action lowering ABI. */
        arceos_ex_must_action_lowering_use_context_ref_and_typed_packet();

        /* Scheduler action checkpoints. */
        arceos_ex_must_scheduler_action_checkpoints_cover_pick_switch_and_schedule_exit();

        /* BootIdleEntryPhase boot-idle chain. */
        arceos_ex_must_rest_init_setup_show_boot_idle_tail_chain();

        /* Smoke/KUnit coverage. */
        arceos_ex_must_rest_init_smoke_cover_idle_schedule_relations();

        /* CPU instance model. */
        arceos_ex_must_model_cpu_instances_with_unified_cpu_type();

        /* CpuGroup indexing. */
        arceos_ex_must_cpu_group_index_cpu_refs_by_logical_id();

        /* CpuGroup ownership. */
        arceos_ex_must_cpu_group_not_own_cpu_bodies();

        /* CPU state facts. */
        arceos_ex_must_cpu_state_be_instance_facts_and_group_set_views();

        /* AP CurrentCPU boundary. */
        arceos_ex_must_not_generate_live_ap_current_cpu_before_entry();

        /* DefaultSchedRootDomain coverage. */
        arceos_ex_must_default_root_domain_cover_cpu_group_possible_refs();

        /* RunQueue root-domain attach. */
        arceos_ex_must_attach_possible_cpu_runqueues_to_default_root_domain();

        /* sched_init wait-bit/radix/maple synchronization facts. */
        arceos_ex_must_sched_init_bit_wait_table_expose_bucket_waitqueue_heads();
        arceos_ex_must_sched_init_bit_wait_table_use_within_context();
        arceos_ex_must_sched_init_radix_maple_rcu_free_callbacks_deferred();

        /* sched_init workqueue early synchronization facts. */
        arceos_ex_must_sched_init_workqueue_register_pool_workqueue_cache();
        arceos_ex_must_sched_init_workqueue_use_within_mutex_contexts();
        arceos_ex_must_sched_init_workqueue_keep_worker_runtime_deferred();

        /* sched_init softirq/RCU/tracing boundaries. */
        arceos_ex_must_sched_init_softirq_prepare_shell_only();
        arceos_ex_must_sched_init_rcu_register_rcu_softirq_explicitly();
        arceos_ex_must_sched_init_rcu_expose_tree_and_tasks_init_facts();
        arceos_ex_must_sched_init_trace_housekeeping_context_tracking_classify_by_config();

        /* CPU-owned RunQueue/IdleTask. */
        arceos_ex_must_model_runqueue_and_idle_task_as_cpu_owned();

        /* Boot scheduler lock ownership. */
        arceos_ex_must_bind_boot_scheduler_locks_to_owned_objects();

        /* CPU-owned scheduler view lowering. */
        arceos_ex_must_expose_boot_cpu_owned_scheduler_view();
        arceos_ex_must_observe_rest_init_runqueue_facts_through_cpu_view();
        arceos_ex_must_validate_task_creation_through_cpu_owned_scheduler_view();
        arceos_ex_must_observe_smoke_scheduler_facts_through_cpu_view();

        /* Transitional lowering. */
        arceos_ex_must_cpu_group_lowering_mark_boot_secondary_split_transitional();

        /* CPU/CpuGroup coverage. */
        arceos_ex_must_cpu_group_smoke_cover_index_and_sets();

        /* CurrentTaskRef scope. */
        arceos_ex_must_current_task_ref_be_cpu_view_private();

        /* CurrentRunQueueRef scope. */
        arceos_ex_must_current_runqueue_ref_be_cpu_view_private();

        /* CurrentRunQueueRef topology lowering. */
        arceos_ex_must_current_runqueue_ref_carry_resolved_cpu_id();

        /* RunQueueRef / CurrentRunQueueRef type split. */
        arceos_ex_must_split_selected_runqueue_ref_from_current_runqueue_ref();

        /* BootRunQueueRef transitional lowering. */
        arceos_ex_must_treat_boot_runqueue_ref_as_up_transitional_lowering();

        /* CurrentRunQueueRef API smoke. */
        arceos_ex_must_current_runqueue_ref_api_smoke_use_formal_runqueue_actions();

        /* Scheduler.schedule() payload smoke. */
        arceos_ex_must_scheduler_schedule_smoke_use_payload_cooperative_switch();

        /* Wake-up task CPU action. */
        arceos_ex_must_wakeup_set_task_cpu_between_select_and_enqueue();

        /* KernelInitTask affinity action. */
        arceos_ex_must_rest_init_pin_kernel_init_as_task_action();

        /* Fork dependency. */
        arceos_ex_must_rest_init_not_make_pre_smp_depend_on_up_multitask_wrapper();

        /* No real task switch. */
        arceos_ex_must_rest_init_keep_true_task_switching_deferred();

        /* Deferred runtime. */
        arceos_ex_must_rest_init_keep_smp_and_later_runtime_deferred();
    }
}

type ArceosExPreSmpInitCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_pre_smp_init_model_path_under_smp_runtime_phase();

        /* Code path. */
        arceos_ex_must_pre_smp_init_code_path_follow_smp_runtime_phase_tree();

        /* Entry facts. */
        arceos_ex_must_pre_smp_init_run_from_scheduler_dispatch_facts();

        /* kthreadd_done wait side. */
        arceos_ex_must_kernel_init_wait_observe_kthreadd_ready_gate();

        /* KernelInitTask entry. */
        arceos_ex_must_pre_smp_init_consume_kernel_init_entry_contract();

        /* Allocation and CPU topology. */
        arceos_ex_must_pre_smp_init_open_full_gfp_and_prepare_topology();

        /* Runtime support setup. */
        arceos_ex_must_pre_smp_init_setup_workqueue_vmstat_tasks_rcu_and_initcalls();

        /* workqueue_init() synchronization. */
        arceos_ex_must_pre_smp_init_workqueue_init_use_pool_mutex_context();

        /* Stop before SMP. */
        arceos_ex_must_pre_smp_init_stop_before_smp_init();
    }
}

type ArceosExSmpBringupCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_smp_bringup_model_path_under_smp_runtime_phase();

        /* Code path. */
        arceos_ex_must_smp_bringup_code_path_follow_smp_runtime_phase_tree();

        /* BP/AP phase split. */
        arceos_ex_must_smp_bringup_split_bp_and_ap_phase_lines();

        /* Per-AP idle task and stack. */
        arceos_ex_must_smp_bringup_prepare_per_ap_idle_task_and_stack();

        /* SBI HSM start path. */
        arceos_ex_must_smp_bringup_use_sbi_hsm_hart_start();

        /* AP subphases. */
        arceos_ex_must_smp_bringup_generate_ap_entry_prelude_callin_online_idle_phases();

        /* Synchronization. */
        arceos_ex_must_smp_bringup_keep_bp_ap_sync_explicit();

        /* CPU hotplug guards. */
        arceos_ex_must_smp_bringup_preserve_hotplug_guards();

        /* Completion wait locks. */
        arceos_ex_must_smp_bringup_preserve_completion_wait_locks();

        /* SBI boot data ordering. */
        arceos_ex_must_smp_bringup_preserve_sbi_boot_data_ordering();

        /* AP local sync summary. */
        arceos_ex_must_smp_bringup_record_ap_local_sync_summary();

        /* Online boundary. */
        arceos_ex_must_smp_bringup_make_secondary_cpus_online();
        arceos_ex_must_smp_bringup_online_only_after_ap_done_up_ack();

        /* AP checkpoints. */
        arceos_ex_must_smp_bringup_checkpoint_ap_subphases();

        /* Deferred hotplug callbacks. */
        arceos_ex_must_smp_bringup_keep_hotplug_callbacks_deferred();

        /* Later runtime. */
        arceos_ex_must_smp_bringup_handoff_to_runtime_core();
        arceos_ex_must_smp_runtime_expand_later_subphases_explicitly();
    }
}

type ArceosExRuntimeCoreCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_runtime_core_model_path_under_smp_runtime_phase();

        /* Code path. */
        arceos_ex_must_runtime_core_code_path_follow_smp_runtime_phase_tree();

        /* Entry gate. */
        arceos_ex_must_runtime_core_run_after_smp_bringup();

        /* Scheduler SMP action. */
        arceos_ex_must_runtime_core_enable_scheduler_smp_action();
        arceos_ex_must_runtime_core_use_sched_domains_mutex_guard();

        /* Workqueue topology. */
        arceos_ex_must_runtime_core_setup_workqueue_topology_action();
        arceos_ex_must_runtime_core_use_workqueue_topology_mutex_guards();

        /* Deferred runtime cores. */
        arceos_ex_must_runtime_core_keep_async_and_padata_deferred();

        /* Page allocator late. */
        arceos_ex_must_runtime_core_setup_page_allocator_late_action();
        arceos_ex_must_runtime_core_record_page_late_trimmed_reasons();
    }
}

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

type ArceosExRootfsCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_rootfs_model_path_under_smp_runtime_phase();

        /* Code path. */
        arceos_ex_must_rootfs_code_path_follow_smp_runtime_phase_tree();

        /* Entry gate. */
        arceos_ex_must_rootfs_run_after_initcall();

        /* KUnit runtime. */
        arceos_ex_must_rootfs_keep_kunit_trimmed_inside_rootfs_phase();

        /* Deferred initramfs and console details. */
        arceos_ex_must_rootfs_keep_initramfs_and_console_deferred();

        /* Required branch checkpoint. */
        arceos_ex_must_rootfs_require_prepare_namespace_branch();

        /* RootFS enable. */
        arceos_ex_must_rootfs_prepare_namespace_inputs_use_existing_devfs_and_block_registry();
        arceos_ex_must_rootfs_mount_ext2_at_linux_root_staging_point();

        /* prepare_namespace() path classification. */
        arceos_ex_must_rootfs_classify_prepare_namespace_paths();

        /* Root switch. */
        arceos_ex_must_rootfs_move_ext2_mount_and_chroot_dot_as_separate_actions();

        /* Integrity keys. */
        arceos_ex_must_rootfs_keep_integrity_keys_deferred_only();

        /* Boundary. */
        arceos_ex_must_rootfs_boundary_handoff_to_finalize();
    }
}

type ArceosExFinalizeCodingMust {
    invariant {
        /* Model path. */
        arceos_ex_must_finalize_model_path_under_smp_runtime_phase();

        /* Code path. */
        arceos_ex_must_finalize_code_path_follow_smp_runtime_phase_tree();

        /* Entry gate. */
        arceos_ex_must_finalize_run_after_rootfs_phase();

        /* Deferred cleanup details. */
        arceos_ex_must_finalize_keep_cleanup_details_deferred();

        /* Trimmed current-config paths. */
        arceos_ex_must_finalize_record_trimmed_config_paths();

        /* System state. */
        arceos_ex_must_finalize_publish_system_running();

        /* RCU boot end. */
        arceos_ex_must_finalize_end_rcu_inkernel_boot();

        /* Boundary. */
        arceos_ex_must_finalize_boundary_handoff_to_payload();
    }
}
