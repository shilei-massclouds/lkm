/*
 * arceos_ex Object Coding Specification
 *
 * This file records target-specific object-coding constraints for the current
 * arceos_ex prototype. It does not redefine model semantics; it constrains how
 * code must realize selected model transitions in this implementation target.
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
predicate arceos_ex_must_user_mode_entry_use_existing_trap_return_path() -> bool;
predicate arceos_ex_must_user_trap_entry_switch_to_kernel_stack() -> bool;
predicate arceos_ex_must_user_syscall_dispatch_use_exception_stream_branch() -> bool;
predicate arceos_ex_must_not_generate_syscall_dispatcher_object() -> bool;
predicate arceos_ex_must_syscall_table_hold_concrete_syscall_actions() -> bool;
predicate arceos_ex_must_user_syscall_write_copy_from_user_address_space() -> bool;
predicate arceos_ex_must_syscall_table_support_dynamic_linker_memory_actions() -> bool;
predicate arceos_ex_must_syscall_table_support_directory_openat_getdents64_slice() -> bool;
predicate arceos_ex_must_syscall_table_support_getrandom_via_hwrng_core() -> bool;
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
predicate arceos_ex_must_rest_init_not_make_pre_smp_depend_on_rest_init_ready() -> bool;
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
        /*
         * MemBlock allocation:
         *
         * DeviceTree.setup() must allocate the expanded tree storage through
         * MemBlock early allocation. Its dependency on MemBlock.Online is an
         * allocation capability, not merely a read-only state check.
         */
        arceos_ex_must_device_tree_setup_allocates_from_memblock();

        /*
         * Two-pass unflatten:
         *
         * DeviceTree.setup() must first traverse RawDtb to validate and compute
         * the required expanded storage, then allocate storage, then traverse
         * RawDtb again to populate DeviceNode and property relations.
         */
        arceos_ex_must_device_tree_unflatten_uses_two_passes();

        /*
         * Established mapping:
         *
         * MemBlock returns physical storage. DeviceTree.setup() may write the
         * expanded tree only after resolving that storage through the already
         * established kernel linear mapping provided by SwapperVm/Config.
         */
        arceos_ex_must_device_tree_storage_uses_established_linear_mapping();

        /*
         * No heap or fixed static substitute:
         *
         * DeviceTree.setup() must not use the ordinary heap, Vec/Box, or a
         * fixed static array as the final expanded tree storage. A bounded
         * temporary stack/global workspace is allowed only for traversal state,
         * not as the DeviceTree storage itself.
         */
        arceos_ex_must_device_tree_avoid_heap_and_fixed_static_storage();

        /*
         * Checkpoint after validation:
         *
         * The DeviceTree Ready checkpoint may be emitted only after the second
         * pass has populated the tree and the model-level root, parent/child,
         * path lookup and property query facts have been checked.
         */
        arceos_ex_must_device_tree_checkpoint_after_validation();
    }
}

type ArceosExCorePrepareCodingMust {
    invariant {
        /*
         * CorePrepare concurrency boundary:
         *
         * CorePreparePhase runs after paging_init() and before trap_init() /
         * mm_core_init() while Linux still has early_boot_irqs_disabled == true
         * and secondary CPUs have not been brought online. The implementation
         * must not open local IRQs, task concurrency, or SMP concurrency in
         * this phase; its ready check must still observe the model facts
         * early_boot_irqs_disabled_true(), interrupt_concurrency_closed(),
         * task_concurrency_closed(), and smp_concurrency_closed().
         */
        arceos_ex_must_core_prepare_preserve_early_irq_and_smp_closed_facts();

        /*
         * StaticBranch.setup guard facts:
         *
         * StaticBranch.setup() models Linux jump_label_init(). Even though the
         * current prototype runs before SMP/task concurrency opens, generated
         * code must make the Linux guard semantics visible as facts: the CPU
         * hotplug read guard corresponding to cpus_read_lock() and the
         * jump_label_mutex guard corresponding to jump_label_lock().
         */
        arceos_ex_must_static_branch_setup_record_jump_label_guards();

        /*
         * JumpLabelMutex object mapping:
         *
         * The Linux jump_label_mutex used by jump_label_lock() must be
         * represented as an independent Context object, not as a private bool
         * hidden inside StaticBranch. CorePrepare setup must drive its static
         * initializer/Preset and Ready setup before StaticBranch.setup()
         * consumes it.
         */
        arceos_ex_must_jump_label_mutex_be_independent_context_object();

        /*
         * CpuHotplugLock object mapping:
         *
         * The Linux cpu_hotplug_lock used by cpus_read_lock() must be
         * represented as an independent Context object of type
         * PerCpuRwSemaphore. The object may depend on PerCpuStorage.Prepared
         * for early boot-CPU static per-cpu storage, but PerCpuRwSemaphore as a
         * generic type must not be globally tied to PerCpuStorage. CorePrepare
         * setup must drive PerCpuStorage.Preset, CpuHotplugLock.Preset and
         * CpuHotplugLock.Setup before StaticBranch.setup() consumes it.
         */
        arceos_ex_must_cpu_hotplug_lock_be_independent_percpu_rwsem_object();

        /*
         * StaticBranch jump-label mutex guard lowering:
         *
         * StaticBranch.setup() must preserve the StaticBranchJumpLabelContext
         * source boundary and execute the modeled JumpLabelMutex.Lock/Unlock
         * protocol, or an equivalent implementation that preserves owner,
         * nesting/debug and wakeup-observable effects. SingleTaskContext facts
         * are not enough to erase the mutex protocol in the current lowering
         * strategy. The CorePrepare ready check must observe the independent
         * JumpLabelMutex ready object and a completed lock/unlock guard fact.
         */
        arceos_ex_must_static_branch_setup_drive_jump_label_mutex_guard();

        /*
         * StaticBranch CPU hotplug read guard lowering:
         *
         * StaticBranch.setup() must preserve the CpuHotplugReadContext source
         * boundary for cpus_read_lock()/cpus_read_unlock() and execute the
         * modeled CpuHotplugLock.ReadLock/ReadUnlock pair. SingleTaskContext
         * facts are not enough to erase this read-side protocol in the current
         * lowering strategy. The generic PerCpuRwSemaphore implementation must
         * still provide real read/write behavior for later call sites and
         * smoke tests.
         */
        arceos_ex_must_static_branch_setup_drive_cpu_hotplug_read_guard();

        /*
         * PerCpuRwSemaphore observable behavior:
         *
         * PerCpuRwSemaphore must model static/runtime initialization, ready
         * setup, read-side fast path, writer block flag, reader drain, reader
         * slow-path/blocked observations, write unlock wakeup, and the local
         * RcuSync child object. Lockdep, tracing, exact scheduler waitqueue
         * mechanics and a true asynchronous RCU grace-period service may be
         * internal or deferred, but the visible counters and outcomes must be
         * testable from the implementation.
         */
        arceos_ex_must_percpu_rwsem_expose_reader_writer_protocol();

        /*
         * Text patch synchronization boundary:
         *
         * Runtime static-key code patching uses separate text patch guards and
         * instruction-cache synchronization on RISC-V. CorePrepare
         * StaticBranch.setup() must not silently claim those runtime sync
         * effects; they remain deferred to later StaticBranch action modeling.
         */
        arceos_ex_must_static_branch_text_patch_sync_remain_deferred();

        /*
         * ResourceTree setup guard:
         *
         * Linux init_resources() inserts resources through insert_resource(),
         * whose kernel/resource.c path takes resource_lock with write_lock().
         * ResourceLock must be represented as an independent Context object of
         * type RwLock, not as a bool hidden inside ResourceTree. CorePrepare
         * setup must drive ResourceLock.Preset and ResourceLock.Setup before
         * ResourceTree.setup() consumes it.
         */
        arceos_ex_must_resource_tree_setup_record_resource_lock_write_guard();

        /*
         * ResourceTree resource_lock guard lowering:
         *
         * ResourceTree.setup() must preserve the ResourceTreeWriteContext
         * source boundary and execute the modeled
         * ResourceLock.WriteLock/WriteUnlock pair. SingleTaskContext facts are
         * not enough to erase this write-side protocol in the current lowering
         * strategy. The generic RwLock implementation must still provide real
         * read/write behavior for later call sites and smoke tests.
         */
        arceos_ex_must_resource_tree_setup_drive_resource_lock_write_guard();

        /*
         * RwLock observable behavior:
         *
         * RwLock must model static/runtime initialization, Ready/unlocked
         * setup, read-side sharing, write-side exclusion, trylock outcomes,
         * read/write unlock conditions and the ordinary read_lock()/write_lock()
         * boundary that does not save IRQ flags. Lockdep/debug owner,
         * PREEMPT_RT rwbase_rt, exact architecture raw lock details, exact
         * reader count internals and irqsave/bh/nested API variants may remain
         * internal or deferred until a concrete object needs them.
         */
        arceos_ex_must_rwlock_expose_reader_writer_protocol();

        /*
         * PrintkBuffer setup IRQ guard:
         *
         * Linux setup_log_buf() switches the active printk ring buffer under
         * local_irq_save()/local_irq_restore(). PrintkBuffer.setup() must
         * record the local IRQ save/restore guard before reporting Ready, and
         * must bind that guard to the existing BootCpuLocalInterrupt
         * LocalInterruptControl object rather than hiding it as a PrintkBuffer
         * internal bool.
         *
         * In the current lowering strategy arceos_ex must execute or preserve
         * the save/restore protocol instead of relying on SingleTaskContext to
         * erase it. A future proof-only optimization may be reintroduced only
         * after the model marks the lexical guarded block with a verified
         * single-entry proof and confirms that no saved-flags/debug side
         * effects are consumed.
         */
        arceos_ex_must_printk_buffer_setup_record_local_irq_save_restore();
        arceos_ex_must_printk_buffer_setup_bind_local_irq_guard_to_boot_cpu_control();

        /*
         * Randomness preset conditional lock boundary:
         *
         * Randomness.preset() maps to Linux random_init_early(command_line).
         * The main early-mix path calls the internal _mix_pool_bytes() helper
         * directly, not mix_pool_bytes(), so it does not take input_pool.lock
         * and generated code must not add an unconditional input-pool spinlock
         * guard merely because later random paths use that lock.
         */
        arceos_ex_must_randomness_preset_not_unconditionally_lock_input_pool();

        /*
         * Randomness conditional reseed/credit lock boundary:
         *
         * Linux random_init_early() may enter crng_reseed() when crng_ready()
         * is already true, or _credit_init_bits() when trust_cpu is enabled.
         * Those conditional paths may update base_crng under
         * spin_lock_irqsave(&base_crng.lock, flags) /
         * spin_unlock_irqrestore(&base_crng.lock, flags). The current
         * arceos_ex minimal path may keep this condition deferred, but if the
         * path is implemented it must lower through the RawSpinLock irqsave
         * guard protocol, not through an unguarded update or unconditional
         * early-mix lock.
         */
        arceos_ex_must_randomness_conditional_reseed_use_base_crng_irqsave_lock();
    }
}

type ArceosExEffectiveContextCodingMust {
    invariant {
        /*
         * Natural phase-boundary guard:
         *
         * A natural phase-boundary guard records the Effective Context contribution of
         * the surrounding execution window, such as boot-time single CPU/task
         * execution with local interrupts and preemption already disabled. It
         * must lower to no runtime enter/exit code by itself. Its contribution
         * still participates in nested Effective Context and may influence
         * lowering decisions for inner context or guard constructs; generated
         * code must not emit dummy lock, irq or preemption operations for the
         * phase-boundary guard itself.
         */
        arceos_ex_must_phase_boundary_guard_lower_to_context_contribution_only();

        /*
         * PreemptionControl guard boundary:
         *
         * A guard whose entered_by/exited_by use PreemptionControl is a protocol
         * guard, not a pure boolean proof. It
         * must lower to the target's counted preemption-disable enter and
         * matching exit operation, or an equivalent RAII guard that performs
         * those operations exactly once. Even if an outer Effective Context
         * already proves preemption: disabled, the nested preemption-control guard must
         * still preserve its own count/owner/debug protocol. Avoiding those
         * operations is valid only when the model does not introduce a nested
         * preemption-control guard and instead relies solely on an outer context
         * contribution.
         */
        arceos_ex_must_preemption_guard_lower_to_counted_enter_exit();

        /*
         * LocalInterruptControl guard boundary:
         *
         * A guard whose entered_by/exited_by use LocalInterruptControl
         * irqsave/irqrestore must lower to
         * operations that save the incoming local interrupt state and restore
         * exactly that saved state on exit by default. It must not be reduced
         * to an unconditional disable/enable pair. Current arceos_ex lowering
         * does not use Effective Context plus `only-once` to elide this
         * protocol. A future proof-only optimization must be introduced as a
         * separate rule and must prove that no saved-flags token, count, debug
         * side effect or later consumer depends on the runtime protocol. A
         * guard that intentionally models unconditional local IRQ
         * disable/enable must still be represented as a distinct explicitly
         * documented action, not inferred from the irqsave form.
         */
        arceos_ex_must_local_interrupt_guard_preserve_saved_flags();

        /*
         * RawSpinLock irq-save guard boundary:
         *
         * A guard whose entered_by/exited_by use RawSpinLock LockIrqSave /
         * UnlockIrqRestore must lower to the lock irqsave protocol:
         * save local interrupt state, disable local interrupts as required by
         * the target primitive, acquire the raw spin lock, and on exit release
         * the same lock before restoring the saved interrupt state. Its
         * Effective Context contribution includes the held lock, local
         * interrupts disabled, preemption disabled and voluntary switching
         * disabled while the guard is active, but those derived attributes do
         * not replace the lock/unlock protocol itself.
         */
        arceos_ex_must_raw_spin_lock_irqsave_guard_lock_and_restore();

        /*
         * RawSpinLock ordinary guard boundary:
         *
         * A guard whose entered_by/exited_by use RawSpinLock Action::Acquire /
         * Action::Release represents plain raw_spin_lock/raw_spin_unlock. It
         * must lower to acquiring and releasing the same raw spin lock and
         * must not be silently dropped merely because an outer context already
         * disables local interrupts or preemption. It also must not be
         * rewritten into a second irqsave/irqrestore pair; irq/preemption
         * effects must come only from the explicit outer guard that models
         * them. init_idle() uses this shape for BootRunQueueLock inside the
         * outer BootIdlePiLock irqsave context.
         */
        arceos_ex_must_raw_spin_lock_guard_use_plain_lock_unlock();

        /*
         * Effective Context is not a license to erase protocol guards:
         *
         * Effective Context may influence lowering of runtime-code-free context
         * sources and avoid duplicate attribute-only scaffolding. It must not
         * by itself erase a guard whose implementation owns state, nesting
         * counts, saved flags, lock ownership, memory ordering, debug
         * assertions, wakeups or other resource protocol effects. Such guards
         * remain real code even when an outer context already provides the same
         * high-level attribute. Future proof-only lowering must be introduced
         * as an explicit optimization rule with its own model proof and
         * protocol-side-effect audit.
         */
        arceos_ex_must_effective_context_not_elide_protocol_guards();

        /*
         * RCU read-side first slice:
         *
         * RCU read-side guards belong to the same guard lowering family. The
         * current SchedInitPhase mapping may implement BootIdleRcuReadSide
         * only as the incomplete first slice required by init_idle():
         * balanced rcu_read_lock()/rcu_read_unlock() accounting around
         * __set_task_cpu(). It must keep explicit incomplete/deferred facts
         * and must not claim full RCU reader nesting, preemptible-RCU
         * accounting, quiescent-state reporting, lockdep/debug checks, or
         * scheduler/RCU context-switch integration. Broader RCU read-side
         * lowering requires a later complete primitive-specific rule.
         */
        arceos_ex_must_rcu_read_side_first_slice_remain_marked_incomplete();
        arceos_ex_must_defer_full_rcu_read_side_lowering();
    }
}

type ArceosExDeviceTreeCodingShould {
    invariant {
        /*
         * Unsafe encapsulation:
         *
         * Raw pointer writes into MemBlock-backed storage should be kept behind
         * a narrow internal boundary. The public DeviceTree event surface
         * should expose safe state/query operations.
         */
        arceos_ex_should_encapsulate_device_tree_unflatten_unsafe();
    }
}

type ArceosExMmCoreInitCodingMust {
    invariant {
        /*
         * Memory topology view:
         *
         * MemoryTopology.setup() must only project already Ready Zones into
         * allocator-visible MemoryNode/ZoneSet views. It must not repartition
         * zones, allocate mem_map/page metadata, or claim new ownership of
         * Linux node_zones.
         */
        arceos_ex_must_memory_topology_project_existing_zones_only();

        /*
         * PageAllocator preset/setup split:
         *
         * PageAllocator.preset() must only establish zonelist topology and
         * page allocator CPU hotplug step registration. It must not release
         * MemBlock pages to buddy/free page sets.
         */
        arceos_ex_must_page_allocator_preset_only_builds_topology_and_hooks();

        /*
         * MemBlock handoff:
         *
         * PageAllocator.setup() must perform the memblock_free_all() handoff:
         * account managed pages, populate buddy free page sets, and advance
         * MemBlock through Disable to Offline.
         */
        arceos_ex_must_page_allocator_setup_hands_memblock_pages_to_buddy();

        /*
         * Minimal buddy free lists:
         *
         * PageAllocator.setup() must build buddy free lists as PageAllocator
         * internal storage. The first implementation round uses per-zone,
         * per-order free_area-like lists and a single migratetype.
         */
        arceos_ex_must_page_allocator_buddy_lists_live_inside_page_allocator();
        arceos_ex_must_page_allocator_buddy_use_zone_order_free_area();
        arceos_ex_must_page_allocator_buddy_first_round_single_migratetype();

        /*
         * MemBlock range splitting:
         *
         * The buddy setup must iterate MemBlock usable ranges, exclude
         * reserved ranges, and split the remaining pages into order-aligned
         * buddy blocks before linking them into the free areas.
         */
        arceos_ex_must_page_allocator_buddy_split_memblock_free_ranges();

        /*
         * Metadata-backed free nodes:
         *
         * Buddy free-list nodes must be represented through PageMetadata
         * fields on the block head page. The buddy structure must not depend
         * on Vec, Box, heap, SLUB, or kmalloc storage.
         */
        arceos_ex_must_page_allocator_buddy_use_page_metadata_nodes();
        arceos_ex_must_page_allocator_buddy_use_intrusive_list();
        arceos_ex_must_page_allocator_buddy_free_area_store_only_head_and_count();
        arceos_ex_must_page_allocator_buddy_node_is_block_head_metadata();
        arceos_ex_must_page_allocator_buddy_avoid_heap_storage();

        /*
         * Linux-like buddy API:
         *
         * Once PageAllocator reaches Ready it must expose the formal
         * PageAllocatorType runtime actions as production APIs named after
         * Linux's buddy boundary: alloc_pages(order, gfp), alloc_page(gfp) as
         * an order-0 convenience, and free_pages(page_ref, order). These APIs
         * must be used by later SLUB/kmalloc and smoke paths instead of
         * private test-only hooks.
         */
        arceos_ex_must_page_allocator_expose_linux_like_alloc_pages_api();

        /*
         * PageRef contract:
         *
         * alloc_pages(order, gfp) must return a caller-owned PageRef covering
         * 2^order buddy pages that are reachable through the established
         * linear mapping. The PageRef is the object-model handle used by later
         * SLUB and smoke code; it must not expose allocator internals.
         */
        arceos_ex_must_page_allocator_alloc_pages_return_owned_linear_mapped_pageref();

        /*
         * Free order contract:
         *
         * free_pages(page_ref, order) must release only PageRefs allocated from
         * PageAllocator, and the order must match the allocation order.
         */
        arceos_ex_must_page_allocator_free_pages_match_alloc_order();

        /*
         * mm_core_init synchronization surface:
         *
         * SingleTaskContext contributes contextual facts for boot-time call
         * sites, but it must not make Linux lock, irq, preempt, RCU, per-cpu,
         * or TLB/cache requirements disappear from the formal model. For every
         * mm_core_init object that publishes a later runtime API, the
         * model/coding surface must say whether the sync protocol is executed
         * in setup, is a pure context fact with no runtime protocol, or is
         * explicitly deferred to the runtime consumer.
         */
        arceos_ex_must_mm_core_init_model_sync_even_when_boot_lowering_elides_code();

        /*
         * Zonelist update protocol:
         *
         * Linux __build_all_zonelists(NULL) runs inside
         * write_seqlock_irqsave(&zonelist_update_seq, flags) and
         * printk_deferred_enter()/exit(). The current target must still
         * execute and observe a balanced setup-time irqsave seqlock section
         * and printk-deferred section. Full seqlock reader/retry behavior
         * remains a later runtime refinement.
         */
        arceos_ex_must_page_allocator_preset_record_zonelist_irqsave_protocol();

        /*
         * Page allocator runtime locking:
         *
         * PageAllocator.Ready means alloc_pages/free_pages are callable, not
         * that the complete Linux zone lock, PCP lock, irqsave and preempt
         * protocol has already been implemented. Until a consumer requires
         * those paths outside boot-exclusive context, the runtime locking
         * requirements must remain explicit deferred facts.
         */
        arceos_ex_must_page_allocator_runtime_locking_remain_explicit_deferred();

        /*
         * Page allocator smoke:
         *
         * The first allocator smoke coverage must allocate pages through the
         * formal alloc_pages API, perform bounded read/write through the
         * mapped page reference, and release them through free_pages().
         */
        arceos_ex_must_page_allocator_smoke_cover_page_alloc_free_read_write();

        /*
         * MemBlock remains Offline:
         *
         * mm_core_init() must not destroy or discard MemBlock metadata. That
         * later discard belongs to page_alloc_init_late()/memblock_discard().
         */
        arceos_ex_must_memblock_disable_reaches_offline_not_destroyed();

        /*
         * SWIOTLB ordering:
         *
         * Swiotlb.setup() must complete before MemBlock.Disable so any early
         * default pool or resolved no-pool fact is established while MemBlock
         * allocation/reservation is still available.
         */
        arceos_ex_must_swiotlb_setup_before_memblock_disable();

        /*
         * Static branch policy:
         *
         * MemoryDebugHardening.setup() must bind to the existing StaticBranch
         * registry and scanned EarlyParam boundary. Current mm hardening
         * early-param policy is trimmed to the default policy; it must not
         * initialize a private static-key mechanism or imply full parameter
         * support.
         */
        arceos_ex_must_memory_debug_hardening_use_static_branch_registry();

        /*
         * SLUB object hierarchy:
         *
         * SlubSubsystem is the single SLUB facade object, not a cache
         * instance and not a reusable SlubSubsystemType. The formal cache type
         * name is SlubCache, not SlubCacheType. SlubCacheRegistry must be the
         * single registry/ownership collection for boot caches, formal
         * kmem_cache/kmem_cache_node, kmalloc size-class caches, and later
         * named caches. KmallocCaches is only a size-class reference/index
         * view over registered SlubCache instances; it must not own a second
         * set of cache instances. Named phase objects and Linux
         * KMEM_CACHE(...) sites such as page->ptl, vmap_area, mm_struct,
         * radix tree node, maple node, and pool_workqueue caches must
         * register their underlying SlubCache instance in SlubCacheRegistry
         * instead of keeping only private ready flags. A KMEM_CACHE(...) site
         * may only remain outside the registry if it has an explicit deferred
         * classification with a later owner.
         */
        arceos_ex_must_slub_subsystem_be_single_facade_not_cache_instance();
        arceos_ex_must_slub_cache_type_name_be_slub_cache();
        arceos_ex_must_slub_cache_registry_own_all_cache_instances();
        arceos_ex_must_kmalloc_caches_reference_registered_slub_caches();
        arceos_ex_must_kmem_cache_sites_register_named_slub_caches_or_defer();

        /*
         * SLUB bootstrap:
         *
         * SlubSubsystem.setup() must bootstrap kmem_cache/kmem_cache_node
         * before KmallocCaches is considered Ready.
         */
        arceos_ex_must_slub_bootstrap_before_kmalloc_caches_ready();

        /*
         * Linux-like kmalloc API:
         *
         * Once SlubSubsystem reaches Ready it must expose production
         * kmalloc(size, gfp), kzalloc(size, gfp), and kfree(ref) APIs. The
         * first round may use a minimal page-backed slab implementation, but
         * it must allocate backing pages through PageAllocator rather than
         * test hooks or MemBlock.
         */
        arceos_ex_must_slub_expose_kmalloc_kzalloc_kfree_api();
        arceos_ex_must_slub_kmalloc_use_page_allocator_backing_pages();
        arceos_ex_must_slub_kmalloc_use_fixed_size_classes();
        arceos_ex_must_slub_kmalloc_use_slab_slot_freelist();
        arceos_ex_must_slub_kzalloc_zero_returned_object();
        arceos_ex_must_slub_kfree_recycle_object_to_cache();
        arceos_ex_must_slub_first_round_defer_complex_linux_paths();

        /*
         * SLUB bootstrap synchronization:
         *
         * Linux kmem_cache_init() explicitly runs before slab_mutex is needed
         * for the early bootstrap caches, but SLUB runtime allocation/free,
         * per-cpu/node partial lists, global list mutation, slab sysfs, and
         * FULL state still have synchronization semantics. The target must
         * record the pre-FULL slab_mutex boundary and keep the runtime SLUB
         * locking model as an explicit deferred contract instead of treating
         * SingleTaskContext as a proof that no SLUB locks exist.
         */
        arceos_ex_must_slub_bootstrap_record_slab_mutex_boundary();
        arceos_ex_must_slub_runtime_locking_remain_explicit_deferred();

        /*
         * SLUB/kmalloc smoke:
         *
         * The kmalloc smoke coverage must use the formal kmalloc/kzalloc/kfree
         * APIs to check writable allocations, zeroed kzalloc storage,
         * independent same-size allocations, and free-list reuse.
         */
        arceos_ex_must_slub_smoke_cover_kmalloc_kzalloc_kfree();

        /*
         * Global allocator:
         *
         * KernelGlobalAllocator.setup() must run after SlubSubsystem.Ready and
         * expose the Rust GlobalAlloc boundary through SLUB/kmalloc. Ordinary
         * dynamic containers must depend on this boundary rather than using
         * MemBlock, PageAllocator internals, or SLUB private structures.
         */
        arceos_ex_must_global_allocator_setup_after_slub_ready();
        arceos_ex_must_global_allocator_implement_core_alloc_globalalloc();
        arceos_ex_must_global_allocator_alloc_use_kmalloc();
        arceos_ex_must_global_allocator_alloc_zeroed_use_kzalloc_or_zeroing();
        arceos_ex_must_global_allocator_dealloc_recover_kmalloc_object_from_ptr();
        arceos_ex_must_global_allocator_support_documented_layout_subset();
        arceos_ex_must_dynamic_containers_require_global_allocator_ready();

        /*
         * Dynamic container smoke:
         *
         * The first dynamic-container smoke must use Vec through the ordinary
         * allocator path, force at least one growth, validate stored values,
         * and drop the Vec without direct SLUB or PageAllocator access.
         */
        arceos_ex_must_dynamic_container_smoke_cover_vec_growth_drop();
        arceos_ex_must_dynamic_container_pressure_smoke_cover_layout_boundary();

        /*
         * Page table lock cache:
         *
         * PageTableLockCache.setup() must create the split page-table lock
         * cache corresponding to Linux "page->ptl".
         */
        arceos_ex_must_page_table_lock_cache_named_page_ptl();

        /*
         * Vmalloc boundary:
         *
         * VmallocAllocator.setup() must manage vmalloc/vmap virtual address
         * resources and metadata. VmallocAllocator.map_page_range() is the
         * vmap mapping executor and must install VA/PA mappings using the
         * PageTableCaches/SwapperVm capability already prepared for the
         * vmalloc range.
         */
        arceos_ex_must_vmalloc_allocator_manage_vmap_addresses_and_execute_mappings();

        /*
         * Vmap subobjects:
         *
         * VmallocAllocator.setup() must establish VmapAreaCache,
         * VmapAddressSpace, VmapNodeSet, VmapBlockQueues, and VfreeDeferredSet
         * before VmallocAllocator.Ready is emitted.
         */
        arceos_ex_must_vmalloc_setup_build_all_vmap_subobjects();

        /*
         * Mapping record boundary:
         *
         * Every successful map_page_range() action must create a distinct
         * VmapMapping record bound to the reserved VmapArea, caller-supplied
         * physical range/PFN/pages, protection, and installed swapper page
         * table entries. Reusing a global "mapping ready" bit is not enough.
         */
        arceos_ex_must_vmalloc_map_page_range_record_each_mapping_action();

        /*
         * Runtime mapping window pool:
         *
         * VmallocAllocator.map_page_range() must support mappings within the
         * runtime vmalloc address space, including ranges that cross from one
         * supported L0/PTE window into the next. It must allocate additional
         * L1/L0 page-table pages and sparse slot metadata on demand through
         * PageTableCaches/PageAllocator when a reserved vmap area touches an
         * uninstalled window, reject ranges beyond VMALLOC_END, and reject a
         * second mapping record for an area that already has an installed
         * mapping. VmapArea/VmapMapping record storage must use dynamic
         * container backing and must not fail at the former fixed test-slot
         * capacity.
         */
        arceos_ex_must_vmalloc_support_preallocated_windows_and_reject_duplicate_mapping();
        arceos_ex_must_vmalloc_allocate_l0_windows_on_demand();

        /*
         * Vmalloc synchronization contracts:
         *
         * vmalloc_init() initializes per-cpu vmap block queues, deferred vfree
         * work, vmap nodes and their locks. The current setup path can publish
         * those structures under SystemExclusive, but runtime get/map/unmap/
         * free actions must keep their guard, RCU/lazy purge and TLB/cache
         * contracts visible. Local RISC-V sfence.vma lowering may satisfy the
         * current single-CPU mapping visibility boundary; remote shootdown,
         * lazy purge and RCU freeing remain explicit deferred facts.
         */
        arceos_ex_must_vmalloc_sync_and_locking_contracts_remain_visible();

        /*
         * Unmap/free boundary:
         *
         * VmallocAllocator must tear down installed page table mappings before
         * releasing the corresponding vmap area metadata. The ioremap layer may
         * request this flow, but the actual VA/PTE teardown remains owned by
         * vmalloc/vmap.
         */
        arceos_ex_must_vmalloc_unmap_before_free_vmap_area();

        /*
         * Ioremap/vmalloc split:
         *
         * Ioremap owns the device physical resource source and MMIO attribute
         * policy. VmallocAllocator owns vmap VA allocation and mapping
         * execution only; it must not parse DeviceTree/PCI BARs, decide
         * whether a physical range is mappable, or choose device/non-cache/
         * write-combine/normal memory attributes.
         */
        arceos_ex_must_ioremap_keep_physical_resource_and_mmio_policy_external_to_vmalloc();

        /*
         * Iounmap boundary:
         *
         * Ioremap.iounmap() must retire the ioremap cookie by requesting
         * VmallocAllocator.unmap_page_range() followed by free_vm_area().
         * Ioremap must not clear PTEs itself or maintain vmap free-space
         * metadata.
         */
        arceos_ex_must_ioremap_iounmap_request_vmalloc_teardown_only();

        /*
         * MMIO attributes:
         *
         * Ioremap must model device/non-cache/write-combine/normal-memory
         * mapping attributes explicitly. The current RISC-V implementation may
         * expose only plain device ioremap as supported; the remaining
         * attributes must be recorded as deferred rather than inferred from a
         * mapping that happens to be accessible.
         */
        arceos_ex_must_ioremap_model_mmio_attribute_policy_explicitly();

        /*
         * mm_struct only:
         *
         * MmStructCache.setup() must only establish the "mm_struct" cache as
         * a named cache instance registered under SlubSubsystem/
         * SlubCacheRegistry. MmStructCache must not become a separate
         * allocator type. vm_area_struct, vma lock cache, and mmap_init()
         * remain later proc_caches_init()/process-memory work.
         */
        arceos_ex_must_mm_struct_cache_only_create_mm_struct_cache();
    }
}

type ArceosExMmCoreInitCodingShould {
    invariant {
        /*
         * Checkpoints:
         *
         * mm_core_init() implementation should keep compile-time, report and
         * trimmed-path checkpoints observable in trace/logging without turning
         * them into lifecycle states.
         */
        arceos_ex_should_keep_mm_core_init_checkpoints_observable();
    }
}

type ArceosExLocalIrqEnableCodingMust {
    invariant {
        /*
         * Model path:
         *
         * LocalIrqEnablePhase is InterruptPhase subphase 2. Its formal model
         * path is spec/model/interrupt/local-irq-enable/.
         */
        arceos_ex_must_local_irq_enable_model_path_under_interrupt_phase();

        /*
         * Code path:
         *
         * Phase source layout must follow the model phase tree. The target
         * implementation path for this phase is the interrupt phase subtree,
         * for example impl/arceos_ex/src/phases/interrupt/local_irq_enable.rs.
         */
        arceos_ex_must_local_irq_enable_code_path_follow_interrupt_phase_tree();

        /*
         * Separate subphase:
         *
         * local_irq_enable() must be represented as a standalone
         * LocalIrqEnablePhase after IrqTimeInitPhase.Ready. It must not be
         * folded into IrqTimeInitPhase or moved to IrqOpenPreparePhase.
         */
        arceos_ex_must_local_irq_enable_be_separate_interrupt_subphase();

        /*
         * Scope:
         *
         * The phase may only open the boot CPU local interrupt total gate
         * (RISC-V sstatus.SIE) and clear early_boot_irqs_disabled. It must not
         * enable PLIC source gates, root external input gates, periodic tick,
         * full softirq execution, workqueue workers, RCU GP kthreads, task
         * concurrency or SMP concurrency.
         */
        arceos_ex_must_local_irq_enable_only_open_boot_cpu_local_gate();

        /*
         * Linux ordering:
         *
         * start_kernel() clears early_boot_irqs_disabled before executing
         * local_irq_enable(). The implementation must preserve that ordering
         * so there is no window where SIE is open while the early flag still
         * claims IRQs are disabled.
         */
        arceos_ex_must_local_irq_enable_clear_early_flag_before_enabling_sie();

        /*
         * Context:
         *
         * LocalIrqEnablePhase must not be wrapped in a within context. It is
         * the standalone boundary that changes the boot CPU local interrupt
         * context from disabled to enabled.
         */
        arceos_ex_must_local_irq_enable_have_no_within_context();

        /*
         * Deferred runtime gates:
         *
         * The ready check must keep the negative facts observable: root
         * supervisor external input, PLIC UART source enable, full softirq
         * execution, IPI runtime, workqueue workers, RCU GP threads, task
         * concurrency and SMP concurrency remain closed or deferred.
         */
        arceos_ex_must_local_irq_enable_keep_runtime_gates_deferred();
    }
}

type ArceosExEntryPreludeCodingMust {
    invariant {
        /*
         * RISC-V early alternatives boundary:
         *
         * Linux setup_vm() calls apply_early_boot_alternatives() while the MMU
         * is still off when CONFIG_RISCV_ALTERNATIVE_EARLY=y. The current
         * arceos_ex EntryPrelude implementation may defer the actual
         * alternatives/errata text patch object, but Vm.Preset and the phase
         * ready check must keep that deferral observable instead of silently
         * treating the Linux path as absent or implemented.
         */
        arceos_ex_must_entry_prelude_keep_early_alternatives_deferred();
    }
}

type ArceosExEntrySuccessorCodingMust {
    invariant {
        /*
         * start_kernel() deferred calls:
         *
         * Linux start_kernel() calls init_vmlinux_build_id() before disabling
         * local IRQs, then page_address_init() after boot_cpu_init() and
         * before setup_arch(). EntrySuccessorPhase may defer both objects, but
         * the phase ready check and implementation-visible facts must preserve
         * those call positions instead of treating them as absent.
         */
        arceos_ex_must_entry_successor_keep_start_kernel_deferred_facts();

        /*
         * RISC-V setup_bootmem() facts:
         *
         * MemBlock.setup() must record the RISC-V setup_bootmem() facts needed
         * by later boot objects: phys_ram_base, kernel va-pa offset, DMA32
         * limit/zone input and the hugetlb CMA reserve call position. The
         * current implementation may defer hugetlb/CMA details, but the
         * deferral must remain observable.
         */
        arceos_ex_must_entry_successor_memblock_record_riscv_setup_bootmem_facts();

        /*
         * SwapperVm permission boundary:
         *
         * SwapperVm.setup()/enable() must keep the setup_vm_final() SATP/TLB
         * synchronization fact while recording that CONFIG_STRICT_KERNEL_RWX
         * final text/rodata/data permission splitting is still deferred. It
         * must not claim the final RW/RO/NX protection split merely because
         * the complete kernel address space is online.
         */
        arceos_ex_must_entry_successor_keep_swapper_rwx_boundary_deferred();
    }
}

type ArceosExStartupPhaseCodingMust {
    invariant {
        /*
         * Startup phase order:
         *
         * The arceos_ex startup chain must follow the formal top-level model
         * order PreparePhase -> BootPhase -> InterruptPhase ->
         * UpMultitaskPhase -> SmpRuntimePhase -> PayloadPhase.
         * PayloadPhase setup/enable must not run directly after BootPhase
         * without first completing the intervening phase chain.
         */
        arceos_ex_must_startup_drive_boot_then_interrupt_then_payload();

        /*
         * Payload dependency:
         *
         * PayloadPhase setup and enable must require InterruptPhase.Ready in
         * addition to BootPhase.Ready. A selected payload may rely on the
         * boot CPU interrupt gate and IRQ/time acceptance facts established by
         * InterruptPhase.
         */
        arceos_ex_must_payload_require_interrupt_phase_ready();

        /*
         * Payload placement:
         *
         * PayloadPhase is a StartupTimeline child that follows
         * SmpRuntimePhase. It must not be implemented as the last subphase
         * nested under SmpRuntimePhase.
         */
        arceos_ex_must_payload_follow_smp_runtime_not_nested_under_it();

        /*
         * Finalize handoff:
         *
         * PayloadPhase setup/enable must require FinalizePhase.Ready and the
         * FinalizeBoundary next-boundary fact. This makes the direct handoff
         * from kernel_init() finalization to payload explicit.
         */
        arceos_ex_must_payload_require_finalize_ready();

        /*
         * KernelInitTask execution line:
         *
         * KernelInitTask execution starts at the PreSmpInitPhase entry after
         * rest_init() dispatch facts, then proceeds through the ordered
         * phase chain to FinalizePhase and PayloadPhase. Payload ownership is
         * derived from that continuous execution line instead of from an
         * isolated payload-only fact.
         */
        arceos_ex_must_kernel_init_execution_line_reach_payload();

        /*
         * UserBootPayload selected variant:
         *
         * The first user-mode program path is a selected payload variant named
         * UserBootPayload. It is driven by PayloadPhase setup/enable and must
         * not be implemented as an unrelated phase or as a second root
         * startup chain.
         */
        arceos_ex_must_user_boot_payload_be_selected_payload_variant();

        /*
         * Syscall ownership:
         *
         * User-mode ecall/syscall handling must extend the existing
         * SyscallException branch under ExceptionStream. Code generation must
         * not introduce a separate root Syscall object that bypasses
         * ExceptionStream dispatch.
         */
        arceos_ex_must_user_boot_use_existing_syscall_exception();

        /*
         * ELF object boundary:
         *
         * The user executable model object is ElfObject. Code generation must
         * not create a separate ElfLoader resource object for this slice.
         * ElfObject.Setup parses ELF header/program headers and builds the
         * PT_LOAD mapping plan; the actual user-address-space mapping belongs
         * to UserAddressSpace.Setup. ElfObject.Enable only confirms user-entry
         * preconditions and hands them to UserBootPayload.
         *
         * Dynamic libc support still uses ElfObject. PT_INTERP on the main
         * executable binds an interpreter-path fact and causes UserBootPayload
         * to read the interpreter as another ElfObject role. That interpreter
         * may be ET_DYN, but it is not an ElfLoader resource object and it does
         * not introduce a separate Load lifecycle stage.
         */
        arceos_ex_must_not_generate_elf_loader_object();
        arceos_ex_must_elf_object_setup_build_pt_load_mapping_plan();
        arceos_ex_must_elf_object_support_pt_interp_without_elf_loader_object();

        /*
         * User init ELF validation boundary:
         *
         * Smoke/KUnit may verify the temporary /sbin/init overlay ELF's normal
         * metadata and content facts through ElfObject state, such as entry,
         * PT_LOAD segment count, permissions and expected embedded bytes.
         * Implementations MUST NOT add test-only methods or APIs to ordinary
         * objects for this validation.
         */
        arceos_ex_must_validate_user_init_elf_without_test_only_object_api();

        /*
         * User address-space boundary:
         *
         * UserAddressSpace is the multi-instance user address-space object.
         * Its low half is per user process; its high half shares or references
         * SwapperVm. SwapperVm remains the single kernel shared address-space
         * instance and must not be mechanically converted into a user address
         * space type with arbitrary instances.
         *
         * The first UserAddressSpace instance belongs to the KernelInitTask
         * execution line that reached PayloadPhase. Preset must require
         * KernelInitTask.Online and record the first-instance binding before
         * stack, ELF mappings or trap-frame setup consume the address space.
         * This expresses the Linux-like kernel_init/kernel_execve handoff
         * without claiming that satp has already switched to a user page
         * table.
         *
         * UserAddressSpace.Setup consumes ElfObject's PT_LOAD mapping plan and
         * records segment/stack mapping facts, user-page U permission facts and
         * kernel-page U=0 facts. For PT_INTERP executables it must consume both
         * the main executable ElfObject and the interpreter ElfObject mapping
         * plans into the same user address space. It must also provide a
         * minimal user heap/anonymous mapping arena for the dynamic loader's
         * early brk/mmap-style allocations. It may allocate backing pages,
         * copy ELF file bytes, zero .bss/stack/heap bytes and build a
         * page-table-shaped view. ELF segment backing and low-half user leaf
         * PTEs are page-granular over align_down(p_vaddr)..align_up(p_vaddr +
         * p_memsz); mprotect/munmap mapped-range checks must use that same
         * page range so GNU_RELRO whole-page protection requests are accepted.
         *
         * UserAddressSpace.Enable is the real pre-switch satp-ready boundary.
         * It may allocate real Sv39 user page-table pages, install low-half
         * user leaf PTEs, copy/share SwapperVm high-half root entries, produce
         * a satp token and mark runtime_ready as "ready for the next trap
         * return consumer". It MUST also preserve a prepared-but-not-current
         * fact, and MUST NOT write satp, perform the address-space switching
         * sfence.vma, execute sret, or set up syscall/UserInitProcess state
         * before the explicit trap/syscall round is modeled.
         *
         * The explicit trap/syscall round must consume the prepared
         * UserTrapFrame through the existing EventStream trap-return shape.
         * The first user entry may write satp, execute the required sfence.vma
         * boundary and sret to U-mode. The corresponding trap entry must not
         * trust the user stack as a kernel trap frame stack: it must switch to
         * a kernel-owned trap stack, for example through sscratch, before
         * saving the full trap frame.
         *
         * User ecall must enter the existing ExceptionStream ->
         * SyscallException branch and install a concrete syscall policy there.
         * Code generation MUST NOT create a new root Syscall object or bypass
         * ExceptionStream dispatch. It also MUST NOT create a separate
         * SyscallDispatcher object: SyscallException owns syscall entry,
         * source validation, argument extraction and dispatch selection.
         * SyscallTable is the independent table object and concrete syscalls
         * are SyscallTable actions. The current table handles the first user
         * program's console/file syscalls, vector console writes through
         * writev, the dynamic loader memory actions brk, mmap, mprotect and
         * munmap by routing them to UserAddressSpace, directory-capable
         * openat plus getdents64 by routing through FilesStruct fd position
         * and Ext2/VFS directory records, readlinkat through the VFS no-follow
         * final symlink path, getrandom(278) through HwRngCore current-device
         * reads rather than /dev or VFS, and set_tid_address by recording the
         * current PID1 UserInitProcess clear_child_tid pointer.
         * This is a formal runtime boundary, not a test-only API. It does not
         * implement a full VMA tree, fd table, devfs console file, TTY line
         * discipline, futex/clone/thread-group semantics, fork/wait or signal
         * semantics.
         *
         * The first directory slice must reference local Linux 6.12
         * fs/open.c::do_sys_openat2()/sys_openat(),
         * fs/readdir.c::sys_getdents64()/iterate_dir()/filldir64(), and
         * fs/file.c::fdget_pos(). It may trim Linux permission, LSM,
         * fsnotify/file_accessed, inode i_rwsem, RCU/f_pos_lock contention,
         * mount namespace and complete errno behavior, but it must preserve
         * the production path shape: openat(O_DIRECTORY) installs a readable
         * directory fd, getdents64 serializes linux_dirent64 records from the
         * ext2 directory data, returns copied bytes, and advances the fd
         * offset only at complete record boundaries.
         *
         * The first readlinkat slice must reference local Linux 6.12
         * fs/stat.c::do_readlinkat()/sys_readlinkat(),
         * fs/namei.c::vfs_readlink()/readlink_copy(), and
         * include/uapi/asm-generic/unistd.h::__NR_readlinkat=78. It must
         * preserve the Linux core order: reject bufsiz <= 0 with EINVAL,
         * copy the user pathname, look up the path without following the final
         * symlink, reject a non-symlink final dentry with EINVAL, copy at most
         * bufsiz target bytes without appending NUL, and return the copied
         * byte count. The current slice may support only AT_FDCWD plus
         * absolute paths on read-only ext2 fast symlinks; relative dirfd,
         * AT_EMPTY_PATH, slow symlink page/block bodies, security hooks,
         * atime, RCU/retry_estale and complete errno details remain trimmed.
         *
         * The first getrandom slice must reference local Linux 6.12
         * drivers/char/random.c::sys_getrandom(),
         * include/uapi/linux/random.h and
         * include/uapi/asm-generic/unistd.h::__NR_getrandom=278. It must keep
         * the syscall as a kernel random-core interface, not a /dev pathname
         * operation. The current implementation may support only flags == 0
         * and small USER_COPY_MAX-bounded buffers, routing successful reads
         * through the existing HwRngCore current provider backed by
         * virtio-rng; full CRNG pool readiness, blocking wait queues,
         * GRND_NONBLOCK, GRND_RANDOM, GRND_INSECURE, signal interruption,
         * large-buffer iteration and random-quality policy remain trimmed.
         *
         * APP=user-boot validation in make test must not treat QEMU/SBI
         * shutdown success as sufficient. The host harness must parse the
         * ordinary guest output line "user exit status=N" and require N == 0
         * for native and every configured provider. A nonzero status, missing
         * status line, or failed QEMU command is a visible test failure. The
         * guest exit syscall path still owns status printing and shutdown; the
         * harness only interprets the output. The default checked-in overlay
         * map must install the combined user_smoke fixture as /sbin/init, and
         * the harness must use a case-local disk image so the result cannot
         * pass by reusing stale build/virtio-blk.raw contents.
         */
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
        arceos_ex_must_user_trap_frame_enter_interpreter_when_present();
        arceos_ex_must_user_mode_entry_use_existing_trap_return_path();
        arceos_ex_must_user_trap_entry_switch_to_kernel_stack();
        arceos_ex_must_user_syscall_dispatch_use_exception_stream_branch();
        arceos_ex_must_not_generate_syscall_dispatcher_object();
        arceos_ex_must_syscall_table_hold_concrete_syscall_actions();
        arceos_ex_must_user_syscall_write_copy_from_user_address_space();
        arceos_ex_must_syscall_table_support_dynamic_linker_memory_actions();
        arceos_ex_must_syscall_table_support_directory_openat_getdents64_slice();
        arceos_ex_must_syscall_table_support_getrandom_via_hwrng_core();
        arceos_ex_must_user_syscall_exit_stop_first_user_process();
        arceos_ex_must_user_address_space_share_kernel_half_with_swapper_vm();
        arceos_ex_must_keep_swapper_vm_single_kernel_shared_instance();
        arceos_ex_must_user_boot_harness_require_zero_user_exit_status();
        arceos_ex_must_user_boot_harness_isolate_overlay_disk_images();

        /*
         * Whole-disk ext2 input:
         *
         * The current rootfs image is a whole-disk ext2 filesystem. The
         * UserBootPayload path must not require PartitionTable or
         * BlockPartition objects until the disk image format is changed to
         * include a partition table.
         */
        arceos_ex_must_user_boot_not_require_partition_objects_for_whole_disk_ext2();
    }
}

type ArceosExIrqTimeInitCodingMust {
    invariant {
        /*
         * Model path:
         *
         * IrqTimeInitPhase is InterruptPhase subphase 1. Its formal model
         * path is spec/model/interrupt/irq-time-init/, not
         * spec/model/boot/irq-time-init/.
         */
        arceos_ex_must_irq_time_init_model_path_under_interrupt_phase();

        /*
         * Code path:
         *
         * Phase source layout must follow the model phase tree. The target
         * implementation path for this phase is the interrupt phase subtree,
         * for example impl/arceos_ex/src/phases/interrupt/irq_time_init.rs,
         * rather than the boot phase subtree.
         */
        arceos_ex_must_irq_time_init_code_path_follow_interrupt_phase_tree();

        /*
         * Interrupt-open boundary:
         *
         * IrqTimeInitPhase must finish with IRQ/time infrastructure ready but
         * boot CPU local interrupts still disabled. The local_irq_enable()
         * boundary belongs to the following LocalIrqEnablePhase so this setup
         * body can remain under the global exclusive boot context.
         */
        arceos_ex_must_irq_time_init_keep_local_irq_closed();

        /*
         * RISC-V IRQ stack/SCS setup:
         *
         * init_IRQ() runs init_irq_scs() and init_irq_stacks() before
         * irqchip_init(). With the current config CONFIG_IRQ_STACKS=y and
         * CONFIG_VMAP_STACK=y, the implementation must expose an
         * RiscvIrqStackSet object/facts for per-CPU IRQ stack pointer setup.
         * CONFIG_SHADOW_CALL_STACK is disabled, so IRQ SCS allocation must be
         * recorded as a trimmed/no-op path. Runtime call_on_irq_stack() entry
         * switching remains deferred and must not be claimed Online here.
         */
        arceos_ex_must_irq_time_init_record_riscv_irq_stack_setup();

        /*
         * Timekeeper lock protocol:
         *
         * timekeeping_init() writes tk_core under raw_spin_lock_irqsave()
         * on timekeeper_lock and write_seqcount_begin/end on tk_core.seq.
         * The implementation must keep both the irqsave/raw-spinlock fact
         * and the seqcount-writer fact observable instead of relying only on
         * the enclosing boot-exclusive context.
         */
        arceos_ex_must_irq_time_init_record_timekeeper_irqsave_seqwrite();

        /*
         * Config-trimmed calls:
         *
         * rcu_init_nohz() and kfence_init() are real call points inside the
         * IrqTimeInitPhase Linux range. For the current config,
         * CONFIG_RCU_NOCB_CPU=n makes rcu_init_nohz() an inline no-op and
         * CONFIG_KFENCE=n makes kfence_init() an inline no-op. The
         * implementation must record both positions as structured trimmed
         * facts, not as comments or implicit absence.
         */
        arceos_ex_must_irq_time_init_classify_rcu_nohz_and_kfence_by_config();

        /*
         * PLIC driver split:
         *
         * PLIC must be represented as an independent irqchip driver entry and
         * provider object. The driver registration boundary is separate from
         * the PLIC provider setup result; it must not collapse back into an
         * IrqController boolean or an ordinary PlatformBus probe.
         */
        arceos_ex_must_model_plic_as_independent_irqchip_driver();

        /*
         * IRQCHIP_DECLARE lowering:
         *
         * PLIC irqchip init registration must lower to a retained static
         * entry in an irqchip init LDS section. The implementation must not
         * build the primary irqchip table by runtime push into a growable
         * registry.
         */
        arceos_ex_must_irqchip_init_entries_use_static_lds_section();

        /*
         * init_IRQ call chain:
         *
         * The IrqTimeInitPhase implementation of init_IRQ() must call
         * irqchip_init(), which must call of_irq_init() over the LDS-collected
         * irqchip init section. PLIC setup must be reached because this chain
         * finds the PLIC entry in that section, not because the phase directly
         * invokes a PLIC-specific setup function.
         */
        arceos_ex_must_init_irq_chain_find_plic_driver_from_irqchip_section();

        /*
         * Compatible match and callback:
         *
         * of_irq_init() must match the DeviceTree interrupt-controller node's
         * compatible against the section entry and invoke the matched PLIC
         * init callback. Tests must cover both observations: the section entry
         * is present, and the callback was invoked through traversal.
         */
        arceos_ex_must_of_irq_init_call_plic_init_via_matched_section_entry();

        /*
         * Parent interrupt link:
         *
         * PLIC's interrupt output must be modeled and implemented as feeding
         * the external interrupt input of the parent RISC-V CPU local INTC.
         * The UART external interrupt physical propagation chain is
         * UART -> PLIC -> RISC-V root INTC -> CPU. This link records the
         * physical parent chain only; it is separate from the software
         * dispatch/claim chain. The chain has two independent gates
         * before a UART interrupt can reach the CPU: the PLIC UART source
         * enable gate and the root INTC supervisor external input enable
         * gate. Their ownership and identity must be explicit.
         */
        arceos_ex_must_plic_output_feed_riscv_intc_external_input();

        /*
         * Named interrupt causes:
         *
         * Interrupt causes must be modeled and implemented through named
         * architecture constants or refs. The RISC-V supervisor external
         * interrupt cause is the root INTC EXT_IRQ/SEI input; specs and code
         * must not describe the route by embedding the raw cause number in
         * control-flow logic or prose.
         */
        arceos_ex_must_use_named_interrupt_causes_not_raw_numbers();

        /*
         * External IRQ gates:
         *
         * The two UART external propagation gates must be modeled as named
         * gates with observable Closed state before any runtime source-enable
         * work. The root supervisor external input gate is defined when the
         * RiscvIntc/InterruptStream external route is installed. The PLIC
         * UART source gate is defined by the PlicIrqMapping that binds
         * HwirqRef::PlicUart0 to the UART logical IRQ. Mapping,
         * request_irq(), and chained-handler setup may define these facts,
         * but must defer the explicit Enable action and must not silently
         * open either gate.
         */
        arceos_ex_must_model_external_irq_gates_as_named_closed_gates();

        /*
         * PLIC MMIO ownership:
         *
         * PLIC MMIO mapping must use the runtime ioremap/vmalloc mapping
         * execution path, but its owner is the system irqchip itself. The
         * implementation must not fabricate a PlatformDevice just to reuse
         * device MMIO ownership, because UART platform devices later consume
         * PLIC as their interrupt parent rather than owning the controller.
         */
        arceos_ex_must_plic_ioremap_as_system_irqchip_not_platform_device();

        /*
         * PLIC DT setup:
         *
         * The matched PLIC init callback must parse the DeviceTree interrupt
         * controller node's reg range, riscv,ndev source count, and
         * interrupts-extended parent input before marking Plic.Ready. The
         * parent input must represent a RISC-V external interrupt line; strict
         * phandle-to-boot-hart binding belongs to the later IRQ domain/source
         * mapping step once DeviceTree phandle lookup is modeled.
         */
        arceos_ex_must_plic_setup_parse_dt_reg_ndev_and_interrupts_extended();

        /*
         * IRQ domain split:
         *
         * IrqDomain is the generic IRQ core mapping contract, while
         * PlicIrqDomain is the concrete instance owned by the PLIC provider.
         * The implementation must not collapse logical IRQ allocation into
         * Plic itself or into ns16550a driver-private state.
         */
        arceos_ex_must_model_irqdomain_as_type_and_plic_domain_as_instance();

        /*
         * PLIC source mapping:
         *
         * PlicIrqDomain may translate a one-cell PLIC interrupt specifier and
         * create an idempotent source -> logical IRQ mapping. It must reject
         * source 0 and sources outside the PLIC source count, and duplicate
         * mapping of the same source must return the existing logical IRQ.
         */
        arceos_ex_must_plic_irq_domain_map_source_to_logical_irq_only();

        /*
         * UART IRQ resource:
         *
         * ns16550a platform probe must parse its IRQ resource from the
         * platform device's DeviceTree node, including the interrupt specifier
         * and PLIC interrupt parent. It must then bind the UART port to the
         * PLIC logical IRQ returned by PlicIrqDomain.
         */
        arceos_ex_must_platform_irq_resource_parse_uart_interrupts_from_dt();

        /*
         * Deferred interrupt output:
         *
         * The UART IRQ resource mapping step records the UART source/logical
         * IRQ binding only. It must not enable the PLIC source or declare
         * serial8250 interrupt-driven console output ready. The root INTC
         * supervisor external input gate and PLIC UART source gate must both
         * remain Closed; registering a handler or mapping a source must not
         * silently open either gate.
         */
        arceos_ex_must_uart_irq_mapping_not_enable_source_or_handler();

        /*
         * Explicit external IRQ enable:
         *
         * After UART IRQ resource mapping and request_irq() action recording,
         * the implementation may open the two external propagation gates only
         * through a named UartExternalIrqEnable boundary. That boundary must
         * perform the PLIC UART source enable and the root INTC supervisor
         * external input unmask as separate observable facts. It must not
         * fabricate a UART interrupt, call the handler, run PLIC claim or
         * complete, or mark serial8250 console output interrupt-driven.
         */
        arceos_ex_must_uart_external_irq_enable_be_explicit_boundary();

        /*
         * Production UART interrupt-chain probe:
         *
         * The first real UART interrupt may be triggered only by a named
         * production boundary after UartExternalIrqEnable has opened both
         * gates. That boundary must follow the Linux-like 8250 THRI shape:
         * raise the UART interrupt-output precondition such as MCR.OUT2,
         * enable UART_IER_THRI, and create a real TX-empty transition instead
         * of assuming an IER write alone will always assert an interrupt. It
         * then waits for the real root INTC -> PLIC claim -> IRQ dispatch ->
         * UART handler -> PLIC complete path. KUnit/smoke code must only
         * observe the resulting facts and counters; it must not call trigger,
         * claim, complete, dispatch, or handler APIs directly.
         */
        arceos_ex_must_uart_interrupt_chain_probe_be_production_boundary();

        /*
         * UART interrupt cycle observation:
         *
         * UartInterruptChainProbe must not stop at the first handler call.
         * It must observe one complete real IRQ cycle from a pre-trigger
         * snapshot: THRE request, PLIC non-zero claim, IRQ dispatch, UART
         * handler, PLIC complete, zero-claim loop exit, with non-zero claims
         * paired with completes on the current UART source. Global
         * claim/complete counters are auxiliary observations only; a mismatch
         * caused by another source or a concurrent sampling window must not
         * fail the current UART source cycle when source-scoped deltas match.
         * Zero-claim loop exit closure means that both the zero claim and the
         * following loop exit are observed in the same real claim-loop
         * boundary; when a provider records them as separate counters,
         * diagnostics must not fail only because a concurrent sample sees the
         * two counters temporarily differ.
         */
        arceos_ex_must_uart_interrupt_chain_probe_observe_full_irq_cycle();

        /*
         * IRQ handler registry:
         *
         * request_irq-style handler registration belongs to an IRQ core-side
         * IrqHandlerRegistry/IrqAction object. The implementation must not
         * store handler ownership in PlicIrqDomain, PlicIrqMapping, or
         * ns16550a driver-private ad hoc tables.
         */
        arceos_ex_must_model_irq_handler_registry_as_irq_core_object();

        /*
         * request_irq input contract:
         *
         * The minimal request_irq path must require a logical IRQ that was
         * already produced by PlicIrqDomain for a valid PLIC source. Attempts
         * to register an unmapped logical IRQ must fail, and duplicate
         * registration for the same logical IRQ/device must be rejected or
         * represented as the explicit duplicate policy.
         */
        arceos_ex_must_request_irq_require_mapped_logical_irq();

        /*
         * Handler registration boundary:
         *
         * ns16550a probe may request a UART handler record once its logical
         * IRQ is known, but this must not enable the PLIC source, install a
         * private claim/complete route, or mark serial8250 console output
         * interrupt-driven. The route belongs to the root INTC/PLIC/IRQ core
         * dispatch chain.
         */
        arceos_ex_must_request_irq_record_handler_without_enabling_source();

        /*
         * Context guard:
         *
         * IrqAction records must carry a hardirq-context requirement before
         * they are dispatchable. IRQ dispatch must not open sleep/process-only
         * paths from interrupt context.
         */
        arceos_ex_must_irq_handler_context_guard_remain_deferred_execution();

        /*
         * External interrupt dispatch contract:
         *
         * The RISC-V root INTC external interrupt entry must be a parent
         * EXT_IRQ/SEI entry that forwards to the PLIC chained handler. It
         * must not know about UART or dispatch leaf device handlers directly.
         */
        arceos_ex_must_root_intc_external_irq_enter_plic_chained_handler_only();

        /*
         * PLIC claim/complete order:
         *
         * The PLIC runtime handler must follow the Linux-like order: claim by
         * reading the claim register, translate the claimed source through
         * PlicIrqDomain/generic IRQ dispatch, run the registered action, then
         * complete by writing the claimed source back. It must loop until
         * claim returns zero, and a zero claim must stop dispatch without
         * calling the UART handler. Missing mapping/action may be reported,
         * but each non-zero claimed source must still reach complete.
         */
        arceos_ex_must_plic_claim_before_irq_dispatch_and_complete_after_handler();

        /*
         * IRQ core action dispatch:
         *
         * IRQ core dispatch must use the logical IRQ returned by
         * PlicIrqDomain and run only an action registered in
         * IrqHandlerRegistry. Missing mapping or missing action must not be
         * treated as a successful UART interrupt.
         */
        arceos_ex_must_irq_core_dispatch_registered_action_by_logical_irq();

        /*
         * KUnit capability boundary:
         *
         * New checkpoint KUnit handlers must receive Context as read-only
         * input by default. Writable access is limited to an explicit sink
         * capability such as KTAP output, tracer or auditor objects.
         *
         * MUST: HandlerRun has exactly one ordinary checkpoint-handler
         * capability shape, equivalent to:
         *
         *   Observe(fn(Checkpoint, &Context, &mut dyn Sink) -> CheckpointOutcome)
         *
         * The enum must not regain Read/Write variants or any variant that
         * accepts &mut Context. Changing this prototype requires a prior
         * coding-spec update that names a separate action-level probe
         * capability; it must not be done as a local handler convenience.
         */
        arceos_ex_must_kunit_handlers_receive_read_only_context_by_default();
        arceos_ex_must_checkpoint_handler_run_keep_single_observer_variant();
        arceos_ex_must_checkpoint_handler_run_not_accept_mut_context();
        arceos_ex_must_not_reintroduce_checkpoint_write_handler_variant();

        /*
         * Checkpoint consumers:
         *
         * Checkpoints are observation points. Default builds must not enable a
         * heavy consumer. PROBE=announce enables the
         * checkpoint_handler_announce consumer, where each checkpoint emits a
         * minimal self-announcement. LOG=trace is only a compatibility alias
         * for PROBE=announce and must not be extended as the future
         * Linux-like trace interface. Other PROBE=... values enable
         * checkpoint_handler_* observers such as uart-irq-chain. These
         * consumers may read and emit facts, but they must remain distinct
         * from ordinary execution and from each other.
         */
        arceos_ex_must_checkpoint_consumers_be_cfg_selected();
        arceos_ex_must_log_trace_and_probe_remain_distinct_consumers();

        /*
         * Observation levels and domains:
         *
         * The implementation must distinguish default, light, failure-only,
         * probe-heavy and stress/nightly observation levels. Observation
         * domains must be stable subsystem or object scopes such as PLIC,
         * IRQ-domain, UART8250/TTY, virtio-blk/block, VFS/ext2,
         * scheduler/task, payload and phase boundaries. Long-term facts
         * belong to objects/providers; handlers only consume them.
         */
        arceos_ex_must_define_observation_levels();
        arceos_ex_must_observation_domains_be_subsystem_or_object_scoped();
        arceos_ex_must_observation_facts_be_owned_by_objects_or_providers();

        /*
         * Failure diagnostic lifecycle:
         *
         * failure_diagnostic is collected on a failing predicate/check path,
         * attached to EventError, propagated, and emitted by the final error
         * reporter. It is not a checkpoint handler and must not change the
         * successful checkpoint sequence.
         */
        arceos_ex_must_failure_diagnostic_collection_and_output_be_separate();

        /*
         * Sink-only writes:
         *
         * A KUnit sink may record, audit or emit diagnostics, but it must not
         * expose access to Context, lifecycle state, IRQ state, device state
         * or scheduler state. Adding a new writable sink requires an explicit
         * coding/spec contract.
         */
        arceos_ex_must_kunit_writes_go_through_limited_sink_capability();
        arceos_ex_must_kunit_sink_not_access_or_mutate_context_objects();

        /*
         * Smoke separation:
         *
         * MUST: app smoke cases remain under the smoke app/harness, not under
         * checkpoint KUnit handlers. Mutating object API tests that are useful
         * should be modeled as app smoke or explicit action-level probes, not
         * by re-registering smoke as a checkpoint handler.
         */
        arceos_ex_must_not_register_smoke_cases_as_checkpoint_handlers();

        /*
         * UART IRQ chain KUnit boundary:
         *
         * The checkpoint KUnit for the first UART external interrupt chain is
         * an observer. It may read trace points, counters and object facts
         * from read-only Context, and may write only to its KUnit sink. It
         * must not call the UART handler, PLIC claim/complete, root intc
         * entry, request_irq, source-enable APIs, or mutate pending/claimed
         * state to manufacture progress.
         */
        arceos_ex_must_uart_irq_chain_kunit_remain_read_only_observer();

        /*
         * Flow ownership:
         *
         * The interrupt flow must be advanced by real implementation paths:
         * UART interrupt emission, hart external interrupt entry, root intc
         * dispatch, PLIC claim, IRQ core dispatch, UART handler and PLIC
         * complete. KUnit can only assert before/after snapshots of those
         * facts.
         */
        arceos_ex_must_uart_irq_chain_kunit_not_drive_interrupt_flow();

        /*
         * Concurrency scope:
         *
         * IrqTimeInitPhase opens only the boot CPU local interrupt gate.
         * Task concurrency and SMP concurrency remain closed when this phase
         * reaches Ready.
         */
        arceos_ex_must_irq_time_init_keep_task_and_smp_concurrency_closed();

        /*
         * Runtime services:
         *
         * Opening the boot CPU interrupt gate must not implicitly advance
         * periodic tick service, full softirq execution, IPI enable, workqueue
         * workers, RCU GP kthreads or secondary CPU execution to Online.
         */
        arceos_ex_must_irq_time_init_keep_runtime_services_deferred();

        /*
         * Smoke actions:
         *
         * RiscvTimerProvider.setup() must expose enough action surface for two
         * smoke checks after InterruptStream.enable(): a monotonic time read
         * check and a one-shot clockevent callback check through the timer IRQ
         * route. These checks do not imply full periodic tick service.
         */
        arceos_ex_must_irq_time_init_expose_time_and_clockevent_smoke_actions();
    }
}

type ArceosExIrqOpenPrepareCodingMust {
    invariant {
        /*
         * Model path:
         *
         * IrqOpenPreparePhase is InterruptPhase subphase 3. Its formal model
         * path is spec/model/interrupt/irq-open-prepare/.
         */
        arceos_ex_must_irq_open_prepare_model_path_under_interrupt_phase();

        /*
         * Code path:
         *
         * Phase source layout must follow the model phase tree. The target
         * implementation path for this phase is the interrupt phase subtree,
         * for example impl/arceos_ex/src/phases/interrupt/irq_open_prepare.rs.
         */
        arceos_ex_must_irq_open_prepare_code_path_follow_interrupt_phase_tree();

        /*
         * Ordering:
         *
         * IrqOpenPreparePhase must run after LocalIrqEnablePhase.Ready, with
         * the boot CPU local interrupt gate already open. It must not contain
         * another local_irq_enable() boundary.
         */
        arceos_ex_must_irq_open_prepare_run_after_local_irq_enable();

        /*
         * Runtime services:
         *
         * This phase must keep task concurrency and SMP concurrency closed and
         * must not implicitly start periodic tick service, IPI enable,
         * workqueue workers, RCU GP kthreads or full softirq execution.
         */
        arceos_ex_must_irq_open_prepare_keep_runtime_services_deferred();

        /*
         * SLUB late boundary:
         *
         * kmem_cache_init_late() must be represented as the internal
         * SlubSubsystem flush workqueue fact. It must not advance
         * SlubSubsystem to Online/FULL; that belongs to later slab_sysfs_init()
         * style work.
         */
        arceos_ex_must_irq_open_prepare_keep_slub_ready_not_online();

        /*
         * Console boundary:
         *
         * console_init() must prepare the formal Console object, line
         * discipline registry and early console driver set only. Real device
         * probe, boot console unregister and full handoff remain conditional
         * or deferred facts.
         */
        arceos_ex_must_irq_open_prepare_console_prepared_only();

        /*
         * Trimmed/deferred paths:
         *
         * The panic_later checkpoint, lockdep_init(), locking_selftest(),
         * initrd bounds check, setup_per_cpu_pageset(), numa_policy_init(),
         * acpi_early_init(), late_time_init hook and arch_cpu_finalize_init()
         * positions must be represented by a structured
         * IrqOpenPrepareTrimmedPaths-style object. setup_per_cpu_pageset()
         * remains an explicit PageAllocator deferred fact; the others record
         * their current config/no-op reasons.
         */
        arceos_ex_must_irq_open_prepare_record_trimmed_paths_structurally();

        /*
         * Sched clock local IRQ guard:
         *
         * sched_clock_init() must record the local_irq_disable()/
         * local_irq_enable() window around generic_sched_clock_init() through the existing
         * BootCpuLocalInterrupt LocalInterruptControl. The surrounding phase
         * context has local interrupts enabled, so this temporary guard must
         * remain an explicit protocol fact.
         */
        arceos_ex_must_irq_open_prepare_sched_clock_record_local_irq_guard();

        /*
         * Smoke actions:
         *
         * SchedClock.setup() and DelayLoop.setup() must expose enough action
         * surface for smoke checks after IrqOpenPreparePhase.Ready: a sched
         * clock read that advances and a bounded busy-wait delay action.
         */
        arceos_ex_must_irq_open_prepare_expose_sched_clock_and_delay_smoke_actions();
    }
}

type ArceosExProcessPrepareCodingMust {
    invariant {
        /*
         * Model path:
         *
         * ProcessPreparePhase is InterruptPhase subphase 4. Its formal model
         * path is spec/model/interrupt/process-prepare/.
         */
        arceos_ex_must_process_prepare_model_path_under_interrupt_phase();

        /*
         * Code path:
         *
         * Phase source layout must follow the model phase tree. The target
         * implementation path for this phase is the interrupt phase subtree,
         * for example impl/arceos_ex/src/phases/interrupt/process_prepare.rs.
         */
        arceos_ex_must_process_prepare_code_path_follow_interrupt_phase_tree();

        /*
         * Ordering:
         *
         * ProcessPreparePhase must run after IrqOpenPreparePhase.Ready, with
         * Console.Prepared, SchedClock.Ready, DelayLoop.Ready and the boot CPU
         * local interrupt gate already established.
         */
        arceos_ex_must_process_prepare_run_after_irq_open_prepare();

        /*
         * rest_init boundary:
         *
         * This phase prepares the inputs to rest_init(). It must not create
         * kernel_init, kthreadd or any PID 1 task, and must not advance the
         * system into the scheduling-running state.
         */
        arceos_ex_must_process_prepare_not_create_rest_init_tasks();

        /*
         * Runtime services:
         *
         * This phase must keep task concurrency and SMP concurrency closed and
         * must not implicitly start workqueue workers, RCU GP kthreads, full
         * softirq execution, network namespace runtime or proc visible
         * services. The only VFS service allowed here is the Linux-like
         * vfs_caches_init()/mnt_init() slice that creates the initial
         * ramfs-backed rootfs mount.
         */
        arceos_ex_must_process_prepare_keep_runtime_services_deferred();

        /*
         * Object coverage:
         *
         * The implementation must provide explicit object carriers for the
         * formal PID namespace, anonymous VMA, task creation, credential,
         * vector context, uprobe, signal, task file context, VMA, namespace,
         * keyring and security readiness/preparedness facts.
         */
        arceos_ex_must_process_prepare_cover_pid_task_cred_memory_namespace_key_security_objects();

        /*
         * Task entry creation contract:
         *
         * TaskCreationCore must expose copy_process()/kernel_clone() as the
         * shared creation boundary for later rest_init tasks. That boundary
         * must bind the caller-provided TaskEntry into the new task's startup
         * context; entry is not an after-the-fact descriptive flag.
         */
        arceos_ex_must_task_creation_core_bind_task_entry_in_copy_process();

        /*
         * TaskCreationCore API smoke:
         *
         * TaskCreationCore ObjectApiBehavior smoke must exercise the formal
         * copy_process() contract directly. It may build a local
         * TaskCreationCore subject and read live prerequisite objects, but it
         * must not add a test-only copy helper or register this API case as a
         * checkpoint KUnit smoke case by default.
         */
        arceos_ex_must_task_creation_core_api_smoke_use_copy_process_contract();

        /*
         * Deferred paths:
         *
         * Trimmed and deferred Linux start_kernel() calls in this interval
         * must remain visible as checkpoints or deferred facts rather than
         * silently disappearing from the implementation boundary.
         */
        arceos_ex_must_process_prepare_keep_deferred_paths_explicit();

        /*
         * Trimmed/deferred path carrier:
         *
         * ProcessPreparePhase must use a ProcessPrepareTrimmedPaths-style
         * object to record config-trimmed calls such as x86 EFI runtime
         * switch, SCS, lockdep_init_task(), cpuset/cgroup/taskstats/
         * delayacct/ACPI/KCSAN, and enabled-but-deferred calls such as
         * net_ns_init(), pagecache_init(), seq_file_init(), proc_root_init(),
         * nsfs_init() and pidfs_init(). rcu_init_tasks_generic() is not in
         * this subphase and must be recorded as out-of-scope rather than
         * silently pulled into ProcessPreparePhase.
         */
        arceos_ex_must_process_prepare_record_trimmed_paths_structurally();
    }
}

type ArceosExCompletionCodingMust {
    invariant {
        /*
         * Reusable object:
         *
         * The formal Completion Type must map to a reusable Rust resource
         * object, not to ad-hoc boolean fields on each user. Its implementation
         * target is impl/arceos_ex/src/objects/completion.rs.
         */
        arceos_ex_must_completion_map_to_reusable_object();

        /*
         * Owned wait queue:
         *
         * Completion must own a SimpleWaitQueue field. The field is an owned
         * child resource corresponding to Linux swait_queue_head, not an
         * external wait-queue reference supplied by the caller.
         */
        arceos_ex_must_completion_own_simple_wait_queue();

        /*
         * Event/action boundary:
         *
         * Completion.setup()/enable() advance the ordinary lifecycle state.
         * complete(), complete_all(), wait(), try_wait(), reinit() operate on
         * CompletionExtState and token count while preserving the main
         * lifecycle state, and done() is a read-only action.
         */
        arceos_ex_must_completion_processes_keep_state_effects();

        /*
         * Instance inheritance:
         *
         * A model object declared as an instance of Completion, such as
         * KthreaddReadyGate, must drive the reusable Completion implementation
         * instead of copying completion-specific pending/completed bookkeeping
         * into a private object-local state machine.
         */
        arceos_ex_must_completion_instances_drive_type_processes();

        /*
         * Smoke coverage:
         *
         * Smoke tests must cover Completion.setup(), Completion.enable(),
         * complete(), token observation and token consumption, and must also
         * verify the live kthreadd_done/KthreaddReadyGate instance.
         */
        arceos_ex_must_completion_smoke_cover_setup_complete_and_token_flow();
    }
}

type ArceosExBlockIoCodingMust {
    invariant {
        /*
         * Linux-like block I/O adapter:
         *
         * Before read-only ext2 is introduced, the first filesystem-facing
         * block I/O surface must be modeled as Bio, submit_bio_wait(),
         * minimal blk_mq_submit_bio(), and BufferHead/sb_bread(). It must not
         * introduce BlockReadRequest or BlockIoBuffer as substitute Linux
         * top-level objects.
         */
        arceos_ex_must_block_io_model_bio_buffer_head_before_ext2();

        /*
         * Registry read role:
         *
         * BlockDeviceRegistry::read_default()/read_by_devt(), or equivalent
         * direct registry reads, are a lower-level synchronous adapter below
         * submit_bio_wait(). They may continue to perform default/dev_t lookup
         * and provider dispatch, but higher filesystem-facing paths should
         * enter through Bio/BufferHead rather than treating registry reads as
         * the public block layer.
         */
        arceos_ex_must_block_io_registry_read_remain_lower_level_adapter();

        /*
         * Smoke entry:
         *
         * App smoke coverage for the current ext2-superblock read must use
         * sb_bread()/BufferHead over submit_bio_wait(). It may still validate
         * registry facts produced underneath, but it must not bypass the new
         * block I/O adapter by directly calling registry read APIs.
         */
        arceos_ex_must_block_io_smoke_use_sb_bread_path();

        /*
         * BufferHead storage:
         *
         * BufferHead may carry up to 4KiB ext2 blocks, so its data payload
         * must not be embedded as a large stack-allocated array or returned
         * through nested stack frames. The BufferHead object should own
         * heap-backed or equivalent exclusive dynamic storage for block data.
         */
        arceos_ex_must_buffer_head_data_not_be_large_stack_storage();

        /*
         * Read-only ext2 first slice:
         *
         * The next filesystem step must model and implement the Linux
         * ext2_fill_super()/ext2_iget()/ext2_find_entry()/direct-block read
         * shape over BufferHead. It must remain read-only and must not bypass
         * the Bio/BufferHead layer by calling VirtioBlkDevice private reads.
         */
        arceos_ex_must_ext2_first_slice_read_only_and_buffer_head_based();

        /*
         * Ext2 object model:
         *
         * Ext2Driver must replace the older Ext2Type role and hold the
         * filesystem driver/operation-set facts. Ext2Volume must model the
         * on-disk ext2 volume discovered through BlockDeviceRegistry and
         * BufferHead; absence or invalid layout is an ordinary non-fatal
         * result. Ext2FileSystem must model the mounted in-memory filesystem
         * instance: Preset depends on Ext2Volume, Setup expands metadata/root
         * entry facts, and Enable mounts the read-only ext2 instance into
         * VFS through the minimal mount-to-parent boundary.
         */
        arceos_ex_must_ext2_model_driver_volume_filesystem_lifecycle();

        /*
         * 4K Buffer / ext2 block-size support:
         *
         * The next ext2 step must raise BufferHead and virtio-blk read buffers
         * to at least 4KiB, and Ext2Volume/Ext2FileSystem must accept ext2
         * block_size values 1024, 2048 and 4096. The superblock is still
         * discovered at byte
         * offset 1024; after parsing it, group descriptor and inode/data block
         * reads must use the actual filesystem block size and block-number
         * layout. make disk must not force 1KiB blocks by default.
         */
        arceos_ex_must_ext2_support_4k_buffer_and_block_sizes();

        /*
         * Direct-block read path generalization:
         *
         * Ext2FileSystem directory lookup must scan direct blocks of the
         * current ext2 directory inode until a matching dirent is found or the
         * direct range is exhausted. Ext2FileSystem file read must read a
         * regular file across multiple direct blocks up to inode size, reject
         * too-small caller buffers with ShortBuffer, and support the first
         * single-indirect block for read-only regular files. Double/triple
         * indirect blocks, allocation and writes remain explicit deferred
         * scope.
         */
        arceos_ex_must_ext2_read_path_support_multi_direct_blocks();
        arceos_ex_must_ext2_read_path_support_single_indirect_blocks();

        /*
         * Stable Alpine smoke targets:
         *
         * make disk must construct the ext2 image from the configured Alpine
         * minirootfs tarball instead of creating smoke-only files. The ext2
         * smoke must keep using the read-only Ext2FileSystem path through
         * BufferHead, and must choose stable files from that rootfs, including
         * a regular file whose size spans more than one ext2 block so the
         * multi-direct-block path is actually observed.
         */
        arceos_ex_must_ext2_smoke_use_stable_alpine_rootfs_files();

        /*
         * Build-time rootfs overlay:
         *
         * The current temporary user init fixture is supplied by a rootfs
         * image-construction overlay, not by runtime overlayfs. The overlay
         * operation MUST copy a built fixture output into the staged rootfs
         * target path; if the target exists, including a symlink, the target
         * path itself must be replaced. The default temporary target is
         * /sbin/init. ROOTFS_OVERLAY=none MUST skip overlay processing;
         * otherwise the kernel Makefile MUST read ROOTFS_OVERLAY_MAP, whose
         * default is impl/arceos_ex/tests/user/rootfs-overlay.map. Each
         * non-comment map row declares a rootfs target path, user test name,
         * and optional toolchain/link mode; omitted toolchain/link fields use
         * ROOTFS_OVERLAY_TOOLCHAIN and ROOTFS_OVERLAY_LINK defaults. The
         * special user-test token __absent__ deletes the target path from the
         * staging rootfs instead of building or copying a fixture; it is only
         * for explicit negative/fallback images, not for the default overlay.
         * User-mode overlay test programs MUST live under
         * impl/arceos_ex/tests/user/ and MUST be built through a dedicated
         * user-test Makefile, so the kernel Makefile does not own user-mode
         * compiler details. The default checked-in map MUST be the only
         * persistent overlay map and MUST install user_smoke as /sbin/init.
         * user_smoke lives under impl/arceos_ex/tests/user/smoke/, where
         * smoke.c owns main() and calls subtests such as fileio and
         * sh_probe. Fallback-specific maps that delete /sbin/init, /etc/init
         * or /bin/init MUST be generated as temporary harness/manual inputs,
         * not kept as long-lived checked-in maps. The user-test Makefile MUST
         * expose toolchain and link-mode selection for GNU vs musl GCC and
         * static vs dynamic linking. Staged syscall probe subtests such as
         * sh_probe must print an explicit success marker after each syscall
         * path they validate, so guest output distinguishes a loaded fixture
         * from per-syscall support. Any user-test build failure MUST abort
         * overlay processing and disk construction immediately; the Makefile
         * must not continue with a stale fixture output. The user_smoke
         * framework output MUST use the "user-smoke:" prefix, print begin/end
         * markers for the whole run and each case, include "status=N" on every
         * end marker, and use blank lines to separate the run and case
         * boundaries. The host harness still determines pass/fail from "user
         * exit status=N", not from these human-readable markers. The
         * directory-enumeration
         * sh_probe slice must use the Linux 6.12/RISC-V syscall ABI directly for
         * openat(AT_FDCWD, "/", O_RDONLY|O_DIRECTORY) and getdents64(61),
         * validate linux_dirent64 records, and then close the directory fd. A
         * failing probe is diagnostic evidence for the first unsupported
         * point, not permission to infer the cause without checking the
         * implementation and Linux reference.
         * Distribution command probes such as /bin/ls must first follow a
         * local static/semi-static analysis flow using existing tools such as
         * file, readelf, objdump, local RISC-V Linux syscall headers, and
         * optional read-only sysroot path inspection. This flow records target
         * ELF identity, optional guest symlink resolution, PT_INTERP, program
         * headers, dynamic section, dynamic symbols, visible ecall/a7 evidence,
         * and mapped syscall names in a temporary note. It must not introduce a
         * custom analysis tool, generated long-lived manifest, Makefile target,
         * rootfs staging rebuild, overlay staging change, or default
         * disk/run/test construction change unless a later reviewed spec
         * explicitly requires that engineering investment.
         * For BusyBox applets, whole-binary dynamic symbols are conservative
         * candidates and must be marked as such; the temporary analysis note
         * must not claim them as the applet's complete runtime syscall trace
         * without a later path-sensitive or guest validation step.
         * Each syscall/VFS item promoted from analysis into model/coding specs
         * must be checked against the local Linux 6.12 reference tree at
         * ../linux-6.12, including the RISC-V syscall table/header and the
         * concrete fs/open.c, fs/readdir.c, fs/stat.c, fs/file.c and fs/namei.c
         * entry points relevant to the item. Any first-slice omission of Linux
         * locking, RCU, permission, mount namespace, LSM or errno behavior must
         * be recorded as trimmed/deferred before implementation.
         * make disk must only create the disk image when it is missing by
         * default; overlay configuration changes must not silently rebuild an
         * existing disk image. Explicit rebuild remains a command decision
         * through FORCE=1 or disk-clean.
         *
         * QEMU_APPEND defaults to "earlycon=sbi", and the run target must pass
         * it through to QEMU as -append "$(QEMU_APPEND)". Overriding
         * QEMU_APPEND, for example with "earlycon=sbi init=/bin/ls", changes
         * the Linux-like kernel command line and init selection. It is not a
         * substitute for rootfs overlay, which remains the stable fixture
         * injection mechanism for default user_smoke and staged probes.
         */
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
        arceos_ex_must_qemu_append_default_to_earlycon_sbi_and_passthrough();
        arceos_ex_must_keep_overlay_as_fixture_injection_after_init_cmdline_support();

        /*
         * Directory path lookup:
         *
         * Since the rootfs is no longer a handcrafted root directory fixture,
         * VFS/ext2 lookup must support path components below arbitrary ext2
         * directories backed by direct blocks. Smoke must not require artificial
         * root-directory filler entries just to move a dirent into a later
         * block.
         */
        arceos_ex_must_ext2_lookup_support_path_components_from_directories();

        /*
         * Minimal VFS read-only mount:
         *
         * The next ext2 step must let VfsCore mount a prepared Ext2FileSystem
         * at a normal VFS dentry and route lookup/open/read through VFS before
         * dispatching to Ext2FileSystem's BufferHead-backed direct-block
         * backend. Smoke must read the deterministic ext2 file through that
         * VFS mount path instead of treating direct Ext2FileSystem calls as
         * the acceptance boundary.
         */
        arceos_ex_must_ext2_support_minimal_vfs_read_only_mount();

        /*
         * Minimal pathname walk/read:
         *
         * VfsCore must support absolute path walk from current root, direct
         * component lookup, mount crossing, open-by-path and read-by-path for
         * the currently modeled ramfs/devfs/ext2 subset. Symlink follow must
         * keep the Linux 6.12 namei skeleton: detect S_IFLNK, obtain the link
         * target, restart absolute targets at FsStruct.root, restart relative
         * targets at the symlink parent, preserve remaining path components,
         * and cap follow count at MAXSYMLINKS == 40 with an ELOOP-like error.
         * The current implementation slice may limit the backend to read-only
         * ext2 fast symlinks whose target lives in raw i_block bytes and is
         * truncated by inode size. readlinkat is the separate no-follow final
         * symlink operation and may use the same fast-symlink target source
         * while preserving the Linux 6.12 copy/truncate contract; slow symlink
         * page/block reads, nofollow flags beyond readlinkat's final component,
         * magic links, RCU walk, permissions, mount namespace, full dotdot and
         * complete errno semantics remain trimmed/deferred.
         * Relative dirfd/cwd paths, permissions, fd tables and page cache
         * remain deferred.
         */
        arceos_ex_must_vfs_support_minimal_absolute_path_walk_and_read();

        /*
         * Long-term observation checkpoints:
         *
         * Checkpoints used by nightly/stress longitudinal comparison and
         * Linux-like cross comparison must be specified in model/coding
         * before implementation. The first batch covers the user payload
         * image read, VFS path read, ext2 lookup/read, block task-side
         * submit/wait, and virtio-blk completion source. Failure and timeout
         * observations must use structured classes instead of temporary log
         * text so repeated event sequences can be grouped and compared.
         */
        arceos_ex_must_long_term_checkpoints_follow_model_coding_contracts();
        arceos_ex_must_payload_vfs_ext2_read_emit_observation_checkpoints();
        arceos_ex_must_block_io_task_wait_checkpoints_cover_submit_wait_and_timeout();
        arceos_ex_must_block_io_irq_completion_checkpoints_cover_begin_end_failure();
        arceos_ex_must_block_io_completion_source_distinguish_irq_and_task_poll();
        /*
         * Virtio-blk synchronous request lifecycle:
         *
         * The current read-only block path is synchronous from Bio/
         * BufferHead's perspective. Every live virtio-blk read must submit one
         * request, wait for that exact pending token to complete through IRQ
         * or bounded task-side polling, verify the status byte, release the
         * descriptor chain, and return to the caller only after the request is
         * no longer pending. A later read must not merely spin on an inherited
         * pending flag; it must first converge that inherited request or fail
         * with a structured block I/O error.
         */
        arceos_ex_must_virtio_blk_sync_reads_submit_wait_complete_before_return();

        /*
         * Initcall superblock probe convergence:
         *
         * The initcall-time virtio-blk ext2 superblock probe is the first
         * production read request. It must complete before VirtioBlkReady and
         * before RootfsPhase/VFS/ext2 consumers can issue their own reads.
         * This follows Linux's request lifecycle shape where a request handed
         * to the queue is eventually ended before synchronous callers proceed,
         * rather than leaving a fire-and-forget used-ring entry for a later
         * phase to inherit.
         */
        arceos_ex_must_virtio_blk_initcall_superblock_probe_converge_before_ready();

        /*
         * Single completion consumer:
         *
         * IRQ completion and task-side polling are both valid observation
         * sources, but the same pending token may be consumed only once. The
         * implementation must guard the virtqueue/device/static read-buffer
         * completion path so an IRQ handler and the waiting task cannot race
         * through get_buf/status validation/descriptors release for the same
         * request.
         */
        arceos_ex_must_virtio_blk_completion_consumer_be_single_owner();

        /*
         * Virtqueue memory ordering:
         *
         * Publishing a descriptor chain must order descriptor and avail-ring
         * stores before avail idx and MMIO notify. Observing a used-ring idx
         * from the device must acquire-order subsequent reads of the used
         * element, status byte and data buffer. The current static coherent
         * backing keeps cache maintenance deferred, but it must not omit these
         * ordering boundaries.
         */
        arceos_ex_must_virtqueue_publish_avail_before_notify_with_release_order();
        arceos_ex_must_virtqueue_observe_used_with_acquire_order();
        arceos_ex_must_read_path_error_classification_checkpoint_be_structured();

        /*
         * Deferred ext2 scope:
         *
         * Page cache/folios, indirect blocks, slow symlinks, permissions,
         * xattrs, quotas, allocation, writes and remount/error recovery
         * remain explicit deferred scope in this slice. Fast symlink target
         * extraction from inline i_block bytes is part of the current VFS
         * path-walk slice and must not be implemented by treating the symlink
         * inode as a regular file.
         */
        arceos_ex_must_ext2_defer_page_cache_indirect_and_writes();
    }
}

type ArceosExRestInitCodingMust {
    invariant {
        /*
         * Model path:
         *
         * The rest_init path is split into BootInitRestInitPhase,
         * BootInitScheduleHandoffPhase and BootIdleEntryPhase under
         * spec/model/up-multitask/rest-init/. RestInitPhase is only a
         * compatibility wrapper over those three concrete owner-scoped
         * subphases.
         */
        arceos_ex_must_rest_init_model_path_under_up_multitask_phase();

        /*
         * Code path:
         *
         * Phase source layout must follow the model phase tree. The target
         * implementation path for this phase is the up-multitask phase
         * subtree, for example impl/arceos_ex/src/phases/up_multitask/rest_init.rs.
         */
        arceos_ex_must_rest_init_code_path_follow_up_multitask_phase_tree();

        /*
         * Ordering:
         *
         * BootInitRestInitPhase must run after ProcessPreparePhase.Ready;
         * BootInitScheduleHandoffPhase then opens task-concurrency through the
         * first scheduler handoff; BootIdleEntryPhase records the boot idle
         * continuation. The wrapper RestInitPhase may only report that all
         * three subphases are ready.
         */
        arceos_ex_must_rest_init_run_after_process_prepare();

        /*
         * Task creation facts:
         *
         * The implementation must publish explicit object facts for PID 1
         * KernelInitTask and KthreaddTask creation, scheduling eligibility and
         * kthreadd provider binding.
         */
        arceos_ex_must_rest_init_create_kernel_init_and_kthreadd_facts();

        /*
         * Explicit task entries:
         *
         * KernelInitTask must be created through TaskCreationCore with
         * TaskEntry::KernelInit, and KthreaddTask through TaskEntry::Kthreadd.
         * The selected entry determines the task's first execution line:
         * KernelInitTask enters the PreSmpInitPhase chain, while KthreaddTask
         * enters the kthreadd service loop boundary.
         */
        arceos_ex_must_rest_init_create_tasks_with_explicit_entries();

        /*
         * Kthreadd entry loop:
         *
         * BootInitRestInitPhase must publish KthreaddTask entry/provider facts
         * but must not drive the kthreadd service-loop subphase. The current
         * BP records kthreadd schedule-loop execution as deferred until a
         * KthreaddTask-owned runtime phase exists.
         */
        arceos_ex_must_kthreadd_entry_model_minimal_schedule_loop();

        /*
         * System state:
         *
         * rest_init() must publish SystemState.value == SYSTEM_SCHEDULING and
         * the opening of task-concurrency semantics, without implying SMP.
         */
        arceos_ex_must_rest_init_publish_system_scheduling();

        /*
         * Completion:
         *
         * complete(&kthreadd_done) must drive the reusable Completion object
         * carried by KthreaddReadyGate and publish the visible gate fact. It
         * must not directly release KernelInitTask; the release is observed by
         * KernelInitTask's wait side.
         */
        arceos_ex_must_rest_init_complete_kthreadd_ready_gate();

        /*
         * Scheduler dispatch facts:
         *
         * schedule_preempt_disabled() must be split across the owner boundary:
         * BootInitScheduleHandoffPhase performs BootIdlePreemption
         * enable_no_resched() and Scheduler.schedule(); BootIdleEntryPhase
         * enters the post-schedule BootIdleStartupContext. It must not be
         * implemented as a single Scheduler action and must not introduce a
         * KernelInitDispatchGate lifecycle object; the branch point is the
         * combination of Scheduler first-schedule and KernelInitTask dispatch
         * facts. Scheduler.schedule() must derive the
         * current task reference from the current CPU current-task view,
         * pick next from CurrentRunQueueRef, then switch through TaskRef-based core
         * context save/restore and publish the updated CPU-local current task
         * fact. The implementation boundary must pass through the current
         * CPU's CurrentTaskSlot; it must not infer or publish the current task
         * only from Scheduler counters or BootRunQueue.curr.
         *
         * Scheduler lifecycle belongs to SchedInitPhase. RestInit must consume
         * Scheduler.Online and drive Scheduler.Action::Schedule only; it must
         * not create a new Scheduler lifecycle boundary for dispatch. The
         * schedule action must remain covered by the nested within sequence
         * SchedulePreemptionContext -> ScheduleLocalInterruptContext ->
         * ScheduleRunQueueContext, and the wake-up path must keep task pi lock
         * and runqueue lock coverage in WakeUp*TaskContext /
         * EnqueueSelectedRunQueueContext. kthreadd_done remains a Completion
         * Type process, and BootIdleEntryPhase remains covered by
         * BootIdleStartupContext.
         */
        arceos_ex_must_rest_init_publish_scheduler_dispatch_facts();

        /*
         * Boot idle runtime actions:
         *
         * BootIdleRuntime.setup() must only establish the Ready object shell
         * after scheduler dispatch facts exist. It must not collapse
         * PrepareIdleEntry, RunIdleLoop and DoIdleCycle into one setup-time
         * fact update. The phase code must explicitly drive
         * BootIdleRuntime.prepare_idle_entry(), then
         * BootIdleRuntime.run_idle_loop(), with run_idle_loop() committing one
         * representative do_idle_cycle() boundary. This step still keeps the
         * real idle loop and need_resched loop deferred; it only aligns the
         * code shape with the model action boundary so later AI-generated code
         * has named hooks to extend.
         */
        arceos_ex_must_boot_idle_runtime_split_entry_and_loop_actions();

        /*
         * Representative need_resched idle cycle:
         *
         * BootIdleRuntime.do_idle_cycle() must expose the three model action
         * hooks WaitWhileNoNeedResched, ObserveNeedResched and
         * ScheduleIfNeedResched as named implementation boundaries. The first
         * boundary records that the boot idle task enters an abstract
         * no-need-resched wait state with polling/nohz details deferred; the
         * second records that the CPU-visible environment sets need_resched and
         * the idle task leaves the wait state; the third records a
         * schedule_idle request/return and drains the need_resched fact. This
         * remains an object-level representative cycle: it must not add a true
         * infinite loop, real timer/IRQ wakeup source, or cpuidle/WFI path.
         */
        arceos_ex_must_boot_idle_runtime_model_representative_need_resched_cycle();

        /*
         * schedule_idle wrapper:
         *
         * BootIdleRuntime.schedule_if_need_resched() must now drive a concrete
         * Scheduler.schedule_idle() implementation boundary. The wrapper must
         * require the current CPU's CurrentTaskSlot to still target BootIdleTask
         * and the need_resched observation to have been recorded by
         * BootIdleRuntime. It must reuse the existing Scheduler.schedule()
         * pick-next/switch-to skeleton and must not hand-commit a
         * BootIdleTask -> BootIdleTask identity switch when runnable tasks are
         * present. It must publish idle-specific counters/facts separately
         * from ordinary schedule() calls so smoke/KUnit coverage can
         * distinguish the idle path. It must not model the full Linux do {
         * __schedule(SM_IDLE); } while (need_resched()) loop,
         * sched_submit_work() skip details, or the long-running idle loop yet.
         */
        arceos_ex_must_bind_boot_idle_schedule_if_need_resched_to_schedule_idle();

        /*
         * Action lowering ABI:
         *
         * Coding/codegen may lower model actions with explicit parameters and
         * return bindings to a uniform Action(ContextRef, MutPacketRef)
         * implementation ABI. ContextRef is the object graph entry; MutPacketRef
         * is a strongly typed, local, schema-explicit packet for temporary
         * values passed between peer actions. The formal model must still keep
         * explicit action parameters, return values and let bindings. Packets
         * must not store persistent object facts. With this ABI, every action
         * entry and exit is a potential checkpoint, and internal action
         * boundaries may expose packet fields to checkpoint/KUnit. Object
         * methods must not fetch the global Context themselves.
         */
        arceos_ex_must_action_lowering_use_context_ref_and_typed_packet();

        /*
         * Scheduler action checkpoints:
         *
         * Scheduler.schedule() checkpoint/KUnit coverage must proceed from
         * the front of the action chain. First check PickNextTask exit: next_ref
         * has been produced and, for the first rest_init schedule, prev_ref
         * targets BootIdleTask while next_ref targets KernelInitTask or
         * KthreaddTask; the current implementation deterministically prefers
         * KernelInitTask. Then check SwitchTo entry: the recorded
         * prev_ref/next_ref match the pick result and the checkpoint observes
         * the boundary before the current-task switch commit for this
         * invocation. Then check SwitchTo exit: for the first rest_init
         * schedule return, CurrentTaskRef must target the same runnable task
         * selected by PickNextTask, namely KernelInitTask or KthreaddTask. The
         * coarser Scheduler.Schedule.Exit postcondition checkpoint is checked
         * after local_irq_restore(): for the first rest_init schedule return,
         * CurrentTaskRef must still target the selected runnable task,
         * schedule_passes must be committed, and local interrupt
         * save/restore counts must be balanced for this invocation.
         */
        arceos_ex_must_scheduler_action_checkpoints_cover_pick_switch_and_schedule_exit();

        /*
         * BootIdleEntryPhase boot-idle chain:
         *
         * The phase implementation must present the boot-idle tail chain
         * directly in phase order: enter BootIdleStartupContext, then
         * BootIdleRuntime.setup(), BootIdleRuntime.prepare_idle_entry(),
         * BootIdleRuntime.run_idle_loop(), and the BootIdleEntryPhase.Ready
         * checkpoint. It may use one small helper for each named action, but
         * it must not hide the whole chain behind a single setup_boot_idle_tail()
         * helper or collapse the model action order into one opaque phase call.
         */
        arceos_ex_must_rest_init_setup_show_boot_idle_tail_chain();

        /*
         * Smoke/KUnit coverage:
         *
         * The rest_init smoke case and the checkpoint KUnit smoke reuse must
         * validate the boot idle schedule relation, not only non-zero facts:
         * the representative idle cycle records exactly one
         * Scheduler.schedule_idle() pass in the current BP implementation;
         * idle schedule request/return counters match each other; ordinary
         * schedule/switch/current-task switch counters include that idle pass;
         * and idle identity counters remain zero while runnable boot tasks are
         * present.
         */
        arceos_ex_must_rest_init_smoke_cover_idle_schedule_relations();

        /*
         * CPU instance model:
         *
         * The implementation must realize the model as one reusable CPU object
         * type with one instance per logical CPU. BootCPU is the logical-id-0
         * CPU instance with a bootstrap role, not a separate CPU type.
         */
        arceos_ex_must_model_cpu_instances_with_unified_cpu_type();

        /*
         * CpuGroup indexing:
         *
         * CpuGroup must expose a logical-id indexed CPU reference view:
         * CpuGroup.Cpu[0] targets BootCPU and later entries target secondary
         * CPU instances. Generated code must not create a separate CpuIdMap
         * object; CpuGroup itself carries the index and possible CPU boundary.
         */
        arceos_ex_must_cpu_group_index_cpu_refs_by_logical_id();

        /*
         * CpuGroup ownership:
         *
         * CpuGroup organizes CpuRef indexes, topology and possible/present/
         * online set views. It must not own CPU bodies, and generated code
         * must not model PossibleCpu/PossibleRunQueue as separate owning CPU
         * objects. A set element is a CPU reference.
         */
        arceos_ex_must_cpu_group_not_own_cpu_bodies();

        /*
         * CPU state facts:
         *
         * hartid, logical_id, possible, present, active and online belong to
         * the CPU instance. CpuGroup maintains set views over CPU references
         * for possible/present/online membership.
         */
        arceos_ex_must_cpu_state_be_instance_facts_and_group_set_views();

        /*
         * AP CurrentCPU boundary:
         *
         * A secondary CPU may be present in CpuGroup's possible/present views
         * before bringup, but generated code must not create a live AP
         * CurrentCPU, LocalInterruptControl, CurrentTaskSlot or
         * PreemptionControl chain before that AP enters secondary entry.
         */
        arceos_ex_must_not_generate_live_ap_current_cpu_before_entry();

        /*
         * DefaultSchedRootDomain coverage:
         *
         * DefaultSchedRootDomain must be generated as the scheduler's default
         * root-domain coverage view. It must derive covered_cpus from
         * CpuGroup.possible_cpus, store/resolve entries as CpuRef values, and
         * must not own CPU bodies or define a separate CPU identity table.
         * Naked possible CPU counts are insufficient as the formal model fact.
         */
        arceos_ex_must_default_root_domain_cover_cpu_group_possible_refs();

        /*
         * RunQueue root-domain attach:
         *
         * RunQueue setup for each possible CPU must attach to
         * DefaultSchedRootDomain only after the runqueue CPU reference is known
         * to be covered by DefaultSchedRootDomain.covered_cpus. The boot
         * runqueue must expose or resolve BootCPURef as its CPU reference.
         * Secondary runqueue metadata may be prepared before AP online, but
         * that must not create a live AP CurrentCPU or runnable AP flow.
         */
        arceos_ex_must_attach_possible_cpu_runqueues_to_default_root_domain();

        /*
         * sched_init wait-bit/radix/maple synchronization facts:
         *
         * BitWaitQueueTable lowering must expose wait_bit_init() as a scoped
         * boot initialization of every bit_wait_table bucket's wait_queue_head:
         * the bucket count matches WAIT_TABLE_SIZE, bucket wait queues are
         * ready, their internal spinlocks are initialized, and their lists are
         * empty. These facts must be guarded in the model with a
         * within BitWaitQueueTableInitContext block rather than represented as
         * loose, unscoped ensures. RadixTree and MapleTree setup must keep
         * their runtime call_rcu() node-free callbacks explicitly deferred;
         * node_api_ready must not be read as meaning those callbacks executed
         * during radix_tree_init()/maple_tree_init().
         */
        arceos_ex_must_sched_init_bit_wait_table_expose_bucket_waitqueue_heads();
        arceos_ex_must_sched_init_bit_wait_table_use_within_context();
        arceos_ex_must_sched_init_radix_maple_rcu_free_callbacks_deferred();

        /*
         * sched_init workqueue early synchronization facts:
         *
         * workqueue_init_early() must be represented as the first workqueue
         * stage only: system workqueues and queue/cancel data structures are
         * prepared, but workers do not run. The lowering must register
         * KMEM_CACHE(pool_workqueue, SLAB_PANIC) as a PoolWorkqueue named
         * cache in SlubCacheRegistry. It must model wq_pool_mutex and
         * workqueue_struct->mutex as explicit guard scopes using
         * within WorkqueuePoolMutexContext { ... } and nested
         * within WorkqueueStructMutexContext { ... }. Worker attach/detach,
         * mayday/rescuer locking and manager_wait behavior remain deferred to
         * the later worker-runtime phases.
         */
        arceos_ex_must_sched_init_workqueue_register_pool_workqueue_cache();
        arceos_ex_must_sched_init_workqueue_use_within_mutex_contexts();
        arceos_ex_must_sched_init_workqueue_keep_worker_runtime_deferred();

        /*
         * sched_init softirq/RCU/tracing boundaries:
         *
         * Softirq.Preset in SchedInitPhase must only create the action-table
         * and per-CPU pending-bit shell needed by rcu_init(); Linux
         * softirq_init(), tasklet queues, TIMER_SOFTIRQ and HRTIMER_SOFTIRQ
         * registration occur after early_irq_init() and are driven by
         * IrqTimeInitPhase. RcuCore.setup() must explicitly register
         * RCU_SOFTIRQ against Softirq instead of treating action_table_ready
         * as sufficient. It must expose rcu_init()'s TREE_RCU node tree
         * locks/waitqueues/work, per-CPU rcu_data binding, kfree_rcu batch
         * workqueue/shrinker setup, PM notifier registration, and
         * tasks_cblist_init_generic() per-flavor/per-CPU callback-list,
         * lock, work, and barrier-head facts. RCU GP kthreads, callback
         * execution and full RCU read-side/context-tracking semantics remain
         * deferred. The active .config enables FTRACE/TRACING, CPU_ISOLATION
         * and CONTEXT_TRACKING/CONTEXT_TRACKING_IDLE, but not
         * CONFIG_FTRACE_MCOUNT_RECORD or CONFIG_CONTEXT_TRACKING_USER_FORCE.
         * Therefore ftrace_init() and context_tracking_init() are
         * trimmed/no-op call points for the current RISC-V64 target, while
         * early_trace_init()/trace_init() and housekeeping_init() remain
         * explicit deferred boundaries whose Linux responsibilities must not
         * be collapsed into the project checkpoint announce or ignored as
         * permanently absent. The implementation must preserve the Linux call
         * order inside the existing SchedInitPhase: poking_init()/ftrace_init()
         * are recorded before Scheduler setup via SchedInitPreludeTrimmedPaths,
         * while trace_init()/context_tracking_init() are recorded after
         * rcu_init() via SchedInitTraceContextBoundaries. These are boundary
         * objects, not formal subphases.
         */
        arceos_ex_must_sched_init_softirq_prepare_shell_only();
        arceos_ex_must_sched_init_rcu_register_rcu_softirq_explicitly();
        arceos_ex_must_sched_init_rcu_expose_tree_and_tasks_init_facts();
        arceos_ex_must_sched_init_trace_housekeeping_context_tracking_classify_by_config();

        /*
         * CPU-owned RunQueue/IdleTask:
         *
         * Generated model and code comments must present RunQueue and IdleTask
         * as objects owned by the corresponding CPU instance:
         * CpuGroup.Cpu[id].RunQueue and CpuGroup.Cpu[id].IdleTask. Scheduler
         * may orchestrate setup and policy, but must not be treated as owning
         * every CPU's runqueue or idle task body. Current Rust lowering may
         * temporarily store BootRunQueue/BootIdleTask inside Scheduler fields
         * only if public facts and smoke checks expose them as BootCPU views.
         */
        arceos_ex_must_model_runqueue_and_idle_task_as_cpu_owned();

        /*
         * Boot scheduler lock ownership:
         *
         * BootRunQueueLock must be lowered as the lock owned by the
         * BootCPU-owned BootRunQueue object, and BootIdlePiLock must be
         * lowered as the pi_lock owned by the BootCPU-owned BootIdleTask
         * object. Scheduler.setup() may orchestrate init_idle() ordering, but
         * must not become the semantic owner of those locks. Public readiness
         * checks may expose transitional Scheduler accessors only as
         * projections back to BootRunQueue.lock and BootIdleTask.pi_lock.
         */
        arceos_ex_must_bind_boot_scheduler_locks_to_owned_objects();

        /*
         * CPU-owned scheduler view lowering:
         *
         * While BootRunQueue and BootIdleTask are still stored inside the
         * Scheduler object, generated Rust must expose a formal boot CPU view
         * of that storage. CpuOwnedSchedulerView is the public implementation
         * surface for CpuGroup.Cpu[0].RunQueue and CpuGroup.Cpu[0].IdleTask;
         * CpuIdleTaskView is the public idle-task half of that view. These
         * views must be derived from CpuGroup.Cpu[0], BootRunQueue and
         * BootIdleTask facts, must confirm BootRunQueue.curr/idle both point
         * at BootIdleTask, and must reject mismatched CPU refs or hart ids.
         * They are not test-only wrappers, and smoke must check them directly.
         * Core object implementations that only need the boot CPU-owned
         * RunQueue/IdleTask facts must consume CpuOwnedSchedulerView instead
         * of directly treating Scheduler.boot_runqueue() or
         * Scheduler.boot_idle_task() as the formal ownership source. Direct
         * accessors may remain as transitional storage/debug observation
         * surfaces and for BootRunQueue/BootIdleTask-local APIs, but not as the
         * primary readiness predicate in rest_init task setup/enable paths.
         * RestInit phase predicates and checkpoint/KUnit handlers that verify
         * boot CPU runqueue membership or task count must consume read-only
         * membership/count facts projected by CpuOwnedSchedulerView, not
         * re-read Scheduler.boot_runqueue() as the formal observation source.
         * RestInit task-creation helpers, including TaskCreationCore.copy_process(),
         * must receive enough CpuGroup context to validate the same formal
         * boot CPU-owned scheduler view instead of using BootRunQueue state as
         * an implicit scheduler-ready shortcut.
         * Smoke tests that assert CPU-owned RunQueue/IdleTask functional facts
         * must prefer CpuOwnedSchedulerView/CpuIdleTaskView observations. A
         * smoke test may compare against Scheduler.boot_runqueue() or
         * Scheduler.boot_idle_task() only when the comparison is explicitly a
         * transitional storage parity check.
         */
        arceos_ex_must_expose_boot_cpu_owned_scheduler_view();
        arceos_ex_must_observe_rest_init_runqueue_facts_through_cpu_view();
        arceos_ex_must_validate_task_creation_through_cpu_owned_scheduler_view();
        arceos_ex_must_observe_smoke_scheduler_facts_through_cpu_view();

        /*
         * Transitional lowering:
         *
         * The current Rust storage may temporarily keep boot_cpu and
         * secondary_cpus fields for implementation convenience, but such a
         * split is a lowering detail. Public object facts, checkpoints and
         * code-generation comments must present the unified CPU instance model
         * and logical-id indexed CpuGroup view.
         */
        arceos_ex_must_cpu_group_lowering_mark_boot_secondary_split_transitional();

        /*
         * CPU/CpuGroup coverage:
         *
         * Smoke/checkpoint coverage must observe CpuGroup.Cpu[0] -> BootCPU,
         * boot CPU possible/present/online facts, secondary possible/present
         * but not-online facts, and unique logical-id/hartid boundaries.
         * Scheduler smoke must also observe the formal CpuOwnedSchedulerView
         * and CpuIdleTaskView rather than only comparing private
         * Scheduler.boot_runqueue()/boot_idle_task() fields.
         */
        arceos_ex_must_cpu_group_smoke_cover_index_and_sets();

        /*
         * CurrentTaskRef scope:
         *
         * CurrentTaskRef must be realized as a private object in the current
         * CPU view. The BP path owns the BootCurrentCPU CurrentTaskRef and must
         * not introduce a descriptive CurrentTask object or a global current
         * task singleton. On task switch, next must become the target of this
         * CPU-local CurrentTaskRef. RISC-V64 code should follow the Linux-style
         * tp register implementation reference through the object-level
         * CurrentTaskSlot boundary, but per-cpu storage remains an
         * implementation term, not the model definition.
         */
        arceos_ex_must_current_task_ref_be_cpu_view_private();

        /*
         * CurrentRunQueueRef scope:
         *
         * CurrentRunQueueRef must be realized as a private reference in the
         * current CPU view. It must not be implemented as a descriptive
         * current-runqueue object or as a global current-runqueue singleton. Code
         * should follow the Linux-style path: derive the current task through
         * CurrentTaskRef, read the task's recorded CPU id, then resolve that
         * CPU's runqueue through CPUGroup/runqueue topology. The current BP
         * implementation may collapse this to the boot runqueue while marking
         * that binding as a temporary UP specialization.
         */
        arceos_ex_must_current_runqueue_ref_be_cpu_view_private();

        /*
         * CurrentRunQueueRef topology lowering:
         *
         * Current Rust lowering must carry the resolved CPU id inside
         * CurrentRunQueueRef even while the only concrete target is
         * BootRunQueue. Scheduler.schedule() lowering must derive that CPU id
         * from the CurrentTaskRef target task's recorded CPU id, then validate
         * it against CpuGroup.Cpu[id], Scheduler.cpu_runqueue(id) metadata and
         * DefaultSchedRootDomain coverage. CpuGroup.boot_cpu() may be used only
         * as a boot CPU consistency check after the current task CPU id is
         * known; it must not be the primary source for resolving the current
         * runqueue. Scheduler.Action::SelectRunQueue lowering is a wake-up
         * selection path: it may currently select the boot runqueue, but
         * RestInit task enable paths must consume the selected_rq result by
         * setting task CPU from selected_rq.cpu_id() and passing selected_rq
         * into the enqueue boundary. BootRunQueue may remain the UP selected
         * target, but BootRunQueue enqueue/pick/dequeue APIs must reject a
         * CurrentRunQueueRef with a mismatched CPU id.
         */
        arceos_ex_must_current_runqueue_ref_carry_resolved_cpu_id();

        /*
         * RunQueueRef / CurrentRunQueueRef type split:
         *
         * Generated Rust must keep selected runqueue references separate from
         * current-CPU runqueue references. Scheduler.Action::SelectRunQueue
         * lowering must return a RunQueueRef value, not CurrentRunQueueRef.
         * RestInit enqueue paths and smoke task enqueue/dequeue helpers must
         * pass RunQueueRef into BootRunQueue enqueue/dequeue APIs. Only the
         * schedule()/pick-next path may use CurrentRunQueueRef, after deriving
         * it from CurrentTaskRef -> task CPU id -> CpuGroup.Cpu[id].RunQueue.
         * Both reference types may currently carry the same boot CPU id in the
         * UP path, but sharing the enum/type is not allowed because the object
         * capabilities differ.
         */
        arceos_ex_must_split_selected_runqueue_ref_from_current_runqueue_ref();

        /*
         * BootRunQueueRef transitional lowering:
         *
         * The model may still name BootRunQueueRef as the current UP
         * SelectRunQueue result, but Rust reference checks must present the
         * capability as a CPU-owned runqueue match: the ref's CPU id must match
         * the target CpuGroup.Cpu[id].RunQueue / BootRunQueue metadata. Public
         * implementation constructors and predicates should not expose
         * `targets_boot_runqueue` or `boot(...)` as the formal semantic API
         * for selected or current runqueue refs; use neutral CPU-owned
         * constructors and matching helpers instead. The boot-backed enum
         * variant may remain as a storage/lowering detail until SMP runqueue
         * variants exist.
         */
        arceos_ex_must_treat_boot_runqueue_ref_as_up_transitional_lowering();

        /*
         * CurrentRunQueueRef API smoke:
         *
         * CurrentRunQueueRef/RunQueue ObjectApiBehavior smoke must exercise
         * formal runqueue enqueue and pick-next boundaries. The implementation
         * must not expose test_* scheduler wrappers for these checks; if the
         * boundary is needed by tests, expose it as a formal RunQueue API and
         * route production enqueue/pick behavior through the same API. This
         * smoke case remains app-smoke-only by default, not checkpoint KUnit.
         */
        arceos_ex_must_current_runqueue_ref_api_smoke_use_formal_runqueue_actions();

        /*
         * Scheduler.schedule() payload smoke:
         *
         * A Scheduler.schedule() smoke case that targets the API itself must be
         * app-smoke-only by default and run from the payload phase, where the
         * caller is KernelInitTask, not the rest_init boot-idle checkpoint. The
         * normal scenario must use a minimal cooperative switch loop:
         * KernelInitTask enqueues a smoke scheduler task, calls schedule() a
         * bounded number of times until that task's entry runs, the smoke task
         * records that it executed and calls schedule()/yield, and control
         * returns to KernelInitTask. This requires schedule() to support
         * non-idle CurrentTaskRef in the payload path and requires switch_to to
         * perform a real cooperative stack/context transfer for the smoke task.
         * It must not reuse or weaken the checkpoint/KUnit expectations for
         * the first rest_init schedule, and it must not add test_* subject APIs;
         * any needed boundary must be a formal scheduler/task API.
         */
        arceos_ex_must_scheduler_schedule_smoke_use_payload_cooperative_switch();

        /*
         * Wake-up task CPU action:
         *
         * KernelInitTask and KthreaddTask wake-up paths must follow the
         * Linux ordering: select the target runqueue, update the task's
         * recorded CPU through a Task-level set_task_cpu boundary, then
         * enqueue the task on that runqueue. The current BP implementation may
         * bind the selected runqueue CPU to BootCPU/BootCPURef, but this is a
         * temporary specialization; future SMP code must resolve cpu_of from
         * the selected RunQueueRef.
         */
        arceos_ex_must_wakeup_set_task_cpu_between_select_and_enqueue();

        /*
         * KernelInitTask affinity action:
         *
         * PID 1 boot CPU pinning must be implemented as a KernelInitTask
         * action that sets the PF_NO_SETAFFINITY-equivalent flag and cpumask
         * facts. It must not be used as the wake-up set_task_cpu action and
         * must not be represented by an independent
         * KernelInitAffinity lifecycle object. The RCU read-side boundary
         * around the Linux pid lookup remains a deferred context-modeling
         * question, not a completed resource-exclusive context.
         */
        arceos_ex_must_rest_init_pin_kernel_init_as_task_action();

        /*
         * Fork dependency:
         *
         * PreSmpInitPhase must depend on the KernelInitTask release/dispatch
         * facts and Scheduler first-schedule fact, not on RestInitPhase.Ready
         * or BootIdleEntryPhase.Ready. RestInitPhase.Ready is only a wrapper
         * fact after the three UP multitask subphases are ready.
         */
        arceos_ex_must_rest_init_not_make_pre_smp_depend_on_rest_init_ready();

        /*
         * No real task switch:
         *
         * The current object-level implementation must not pretend to perform
         * a real task-stack switch, preemptive scheduler context switch or
         * idle loop. It may only publish the rest_init boundary facts.
         */
        arceos_ex_must_rest_init_keep_true_task_switching_deferred();

        /*
         * Deferred runtime:
         *
         * Secondary CPU bringup, workqueue workers, Tasks RCU GP kthreads,
         * KernelInitTask.kernel_init_freeable() and kthreadd request
         * consumption remain later-phase work.
         */
        arceos_ex_must_rest_init_keep_smp_and_later_runtime_deferred();
    }
}

type ArceosExPreSmpInitCodingMust {
    invariant {
        /*
         * Model path:
         *
         * PreSmpInitPhase is SmpRuntimePhase subphase 1. Its formal model
         * path is spec/model/smp-runtime/pre-smp-init/.
         */
        arceos_ex_must_pre_smp_init_model_path_under_smp_runtime_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.
         */
        arceos_ex_must_pre_smp_init_code_path_follow_smp_runtime_phase_tree();

        /*
         * Entry facts:
         *
         * This phase must run from the KernelInitTask release/dispatch facts
         * and Scheduler first-schedule fact, not from RestInitPhase.Ready or
         * BootIdleEntryPhase.Ready.
         */
        arceos_ex_must_pre_smp_init_run_from_scheduler_dispatch_facts();

        /*
         * kthreadd_done wait side:
         *
         * PreSmpInitPhase begins on the KernelInitTask execution line by
         * observing kthreadd_done through the Completion wait side. The release
         * fact must be produced by KernelInitTask observing KthreaddReadyGate,
         * not by BootInitTask's complete side.
         */
        arceos_ex_must_kernel_init_wait_observe_kthreadd_ready_gate();

        /*
         * KernelInitTask entry:
         *
         * This phase must also consume the TaskCreationCore entry contract:
         * KernelInitTask was created with TaskEntry::KernelInit and that entry
         * points at the SmpRuntimePhase execution line whose first child is
         * PreSmpInitPhase.
         */
        arceos_ex_must_pre_smp_init_consume_kernel_init_entry_contract();

        /*
         * Allocation and CPU topology:
         *
         * This phase must open PageAllocator full GFP mask and record pre-SMP
         * CPU topology/present facts without making secondary CPUs online.
         */
        arceos_ex_must_pre_smp_init_open_full_gfp_and_prepare_topology();

        /*
         * Runtime support setup:
         *
         * This phase must setup Workqueue, VmstatCore, TasksRcu and early
         * pre-SMP initcall boundary facts.
         */
        arceos_ex_must_pre_smp_init_setup_workqueue_vmstat_tasks_rcu_and_initcalls();

        /*
         * workqueue_init() synchronization:
         *
         * Workqueue.setup() corresponds to Linux workqueue_init(), which takes
         * wq_pool_mutex while fixing pool node hints and creating rescuers.
         * The implementation must expose this KernelInitTask-side mutex guard
         * and the model must use within WorkqueuePoolMutexContext { ... }.
         */
        arceos_ex_must_pre_smp_init_workqueue_init_use_pool_mutex_context();

        /*
         * Stop before SMP:
         *
         * smp_init() is the next top-level phase boundary and must not be
         * executed or modeled as complete here.
         */
        arceos_ex_must_pre_smp_init_stop_before_smp_init();
    }
}

type ArceosExSmpBringupCodingMust {
    invariant {
        /*
         * Model path:
         *
         * SmpBringupPhase is SMP Runtime Phase subphase 2. Its formal model
         * path is spec/model/smp-runtime/smp-bringup/.
         */
        arceos_ex_must_smp_bringup_model_path_under_smp_runtime_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.
         */
        arceos_ex_must_smp_bringup_code_path_follow_smp_runtime_phase_tree();

        /*
         * BP-side focus:
         *
         * The current implementation should follow the boot processor side of
         * smp_init(), not expand AP entry internals.
         */
        arceos_ex_must_smp_bringup_focus_bp_side_flow();

        /*
         * Synchronization:
         *
         * BP/AP synchronization facts must stay explicit: cpu_running,
         * done_up and done_down placement must be represented even when AP
         * internals are summarized.
         */
        arceos_ex_must_smp_bringup_keep_bp_ap_sync_explicit();

        /*
         * CPU hotplug guards:
         *
         * cpuhp_threads_init() must preserve the cpus_read_lock() and
         * smpboot_threads_lock mutex guards. bringup_nonboot_cpus()/cpu_up()
         * must preserve cpu_add_remove_lock and cpus_write_lock() writer
         * guards before publishing secondary online facts.
         */
        arceos_ex_must_smp_bringup_preserve_hotplug_guards();

        /*
         * Completion wait locks:
         *
         * cpu_running and done_up are completions. Their wait.lock raw
         * spinlock irqsave/irqrestore guard must stay observable even though
         * AP internals are summarized.
         */
        arceos_ex_must_smp_bringup_preserve_completion_wait_locks();

        /*
         * SBI boot data ordering:
         *
         * RISC-V cpu_ops_sbi.cpu_start() uses smp_mb() before and after
         * publishing the secondary task/stack boot data. The implementation
         * must retain an equivalent observable ordering fact.
         */
        arceos_ex_must_smp_bringup_preserve_sbi_boot_data_ordering();

        /*
         * AP local sync summary:
         *
         * riscv_ipi_enable(), AP icache/TLB flush, AP local_irq_enable() and
         * the cpuhp_thread_fun() should_run memory-barrier pair remain
         * summary/deferred facts in this phase and must not be treated as
         * absent.
         */
        arceos_ex_must_smp_bringup_record_ap_local_sync_summary();

        /*
         * Online boundary:
         *
         * This phase must move secondary CPUs from present/not-online to
         * online and publish the opening of SMP concurrency.
         */
        arceos_ex_must_smp_bringup_make_secondary_cpus_online();

        /*
         * AP internals:
         *
         * secondary_start_sbi, smp_callin(), AP local IRQ enable, AP idle and
         * AP hotplug callback details remain deferred summary paths.
         */
        arceos_ex_must_smp_bringup_keep_ap_internals_deferred();

        /*
         * Later runtime:
         *
         * SmpBringupPhase must hand off to RuntimeCorePhase. Later
         * subphases are expanded through their own formal model and code
         * steps rather than being silently assumed complete.
         */
        arceos_ex_must_smp_bringup_handoff_to_runtime_core();
        arceos_ex_must_smp_runtime_expand_later_subphases_explicitly();
    }
}

type ArceosExRuntimeCoreCodingMust {
    invariant {
        /*
         * Model path:
         *
         * RuntimeCorePhase is SMP Runtime Phase subphase 3. Its formal model
         * path is spec/model/smp-runtime/runtime-core/.
         */
        arceos_ex_must_runtime_core_model_path_under_smp_runtime_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.
         */
        arceos_ex_must_runtime_core_code_path_follow_smp_runtime_phase_tree();

        /*
         * Entry gate:
         *
         * RuntimeCorePhase must run after SmpBringupPhase.Ready, with
         * secondary CPUs online and SMP concurrency open.
         */
        arceos_ex_must_runtime_core_run_after_smp_bringup();

        /*
         * Scheduler SMP action:
         *
         * Scheduler.enable_smp() must publish SMP scheduler domains, release
         * PID 1 boot CPU affinity, clear PF_NO_SETAFFINITY, refresh
         * granularity and initialize RT/DL SMP post state under the
         * sched_domains_mutex guard without re-running Scheduler lifecycle
         * enable.
         */
        arceos_ex_must_runtime_core_enable_scheduler_smp_action();
        arceos_ex_must_runtime_core_use_sched_domains_mutex_guard();

        /*
         * Workqueue topology:
         *
         * RuntimeCorePhase must publish workqueue topology facts for CPU/SMT,
         * cache and NUMA pod types and rebind unbound pools while keeping the
         * current object-level Workqueue.Ready historical state stable. The
         * topology action must reuse the existing wq_pool_mutex and aggregate
         * workqueue_struct mutex guards.
         */
        arceos_ex_must_runtime_core_setup_workqueue_topology_action();
        arceos_ex_must_runtime_core_use_workqueue_topology_mutex_guards();

        /*
         * Deferred runtime cores:
         *
         * async_init() and padata_init() must remain explicit deferred
         * boundaries in this step. The deferred facts must preserve async
         * workqueue/min_active and padata hotplug/free-list responsibilities.
         */
        arceos_ex_must_runtime_core_keep_async_and_padata_deferred();

        /*
         * Page allocator late:
         *
         * RuntimeCorePhase must publish page_alloc_init_late() facts,
         * including memory stats, buffer init, memblock private discard, zone
         * contiguous, sysctl and current-config trimmed late paths. Deferred
         * struct page completion/static key, page extension and shuffle late
         * paths must be recorded with config-trimmed reasons.
         */
        arceos_ex_must_runtime_core_setup_page_allocator_late_action();
        arceos_ex_must_runtime_core_record_page_late_trimmed_reasons();
    }
}

type ArceosExInitcallCodingMust {
    invariant {
        /*
         * Model path:
         *
         * InitcallPhase is SMP Runtime Phase subphase 3. Its formal model
         * path is spec/model/smp-runtime/initcall/.
         */
        arceos_ex_must_initcall_model_path_under_smp_runtime_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.
         */
        arceos_ex_must_initcall_code_path_follow_smp_runtime_phase_tree();

        /*
         * Entry gate:
         *
         * InitcallPhase must run after RuntimeCorePhase.Ready and preserve
         * the do_basic_setup() entry boundary.
         */
        arceos_ex_must_initcall_run_after_runtime_core();

        /*
         * Pre-do_initcalls classification:
         *
         * The cpuset_init_smp(), driver_init(), init_irq_proc() and
         * do_ctors() slice must be classified against the current
         * ../linux-6.12/.config before auditing do_initcalls(). Disabled
         * CONFIG_CPUSETS/CONFIG_CGROUPS and CONFIG_CONSTRUCTORS paths are
         * trimmed; enabled driver-core/procfs paths must be deferred or
         * formal explicitly, not treated as no-op.
         */
        arceos_ex_must_initcall_classify_pre_do_initcalls_by_config();

        /*
         * Deferred synchronization:
         *
         * driver_init() deferred paths must still record Linux-visible
         * synchronization responsibilities: devtmpfs req_lock/completion/
         * kthread, of_core_init() of_mutex, and bus_register()'s subsys
         * mutex/klist initialization. The boot-only context does not erase
         * those protocols.
         */
        arceos_ex_must_initcall_record_driver_init_deferred_sync_primitives();

        /*
         * Deferred heavy subsystems:
         *
         * DriverCore and IrqProcView must remain explicit deferred
         * boundaries in this step.
         */
        arceos_ex_must_initcall_keep_driver_core_and_irq_proc_deferred();

        /*
         * Constructors:
         *
         * CtorTable must preserve the do_ctors() table position and record
         * the current trimmed/empty constructor table status.
         */
        arceos_ex_must_initcall_record_ctor_table_boundary();

        /*
         * Entry ABI:
         *
         * InitcallEntryPrototype must lower to a retained static function
         * pointer with the ABI fn(ContextRef) -> InitcallReturn. ContextRef is
         * the object graph entry; in the current Rust target it maps to
         * &mut crate::context::Context. InitcallReturn records the per-entry
         * Linux-like outcome and must be captured by InitcallTable.setup().
         * The entry must not lower to a captured closure, heap object or
         * runtime-dispatched callback that carries hidden payload arguments.
         */
        arceos_ex_must_initcall_entry_lower_to_context_ref_result_function();

        /*
         * Static registration:
         *
         * InitcallTable.Register(level, entry) is abstract in the model, but
         * this target must realize it as a Linux-like initcall declaration
         * macro, e.g. arch_initcall_sync!() or device_initcall!(). The macro
         * emits a retained InitcallEntry element into the section selected by
         * level. It must not lower to a runtime function call that pushes the
         * entry into a second registry.
         */
        arceos_ex_must_initcall_register_lower_to_static_section_entry();

        /*
         * Linker collection:
         *
         * The static sections must be retained by the linker and represented
         * in memory as contiguous arrays of InitcallEntry elements. LDS/KEEP
         * start/end symbols, or a build-generated equivalent with the same
         * observable table boundaries, define each level's array.
         */
        arceos_ex_must_initcall_sections_collected_by_lds_ranges();

        /*
         * Preset collection:
         *
         * InitcallTable.preset() must validate the pre-linked static ranges,
         * level mapping and entry operation bindings. For the Linux-like
         * backend the range itself is the table view; preset must not copy
         * entries into a secondary registration table and must not invoke
         * entries.
         */
        arceos_ex_must_initcall_table_preset_collect_static_ranges();

        /*
         * Setup execution:
         *
         * InitcallTable.setup() must represent do_initcalls() by directly
         * iterating the InitcallEntry arrays in Linux level order. It must
         * record level count, all-level execution, command-line scratch reuse,
         * parameter parsing, filtering and run-context checks without
         * promoting every entry to a top-level object.
         */
        arceos_ex_must_initcall_run_static_initcall_table_summary();

        /*
         * Dispatcher shape:
         *
         * InitcallTable.setup() must stay Linux-like: the dispatcher iterates
         * static ranges in level order and invokes function pointers from the
         * descriptors. It must not branch on entry names, owners or concrete
         * operation identities; target effects belong to the entry wrappers
         * and owner objects.
         */
        arceos_ex_must_initcall_dispatcher_remain_entry_agnostic();

        /*
         * do_one_initcall context repair:
         *
         * Per-entry records must preserve Linux do_one_initcall() observable
         * responsibilities: blacklist/filter check, trace start/finish
         * boundary, return recording, preempt-count snapshot with imbalance
         * repair-or-absent, disabled-IRQ repair-or-absent, and latent entropy
         * accounting. A boot-only context may simplify the implementation but
         * must not erase these facts.
         */
        arceos_ex_must_initcall_record_do_one_initcall_context_repair();

        /*
         * Same-level order:
         *
         * The formal model fixes inter-level order only. Same-level order is
         * not a semantic guarantee unless an entry-effect commutativity proof
         * exists; until then this target records the proof gap and expects a
         * nightly permutation test to compare canonical final facts.
         */
        arceos_ex_must_initcall_same_level_order_independence_defer_to_proof_or_nightly();

        /*
         * Mechanism/effect split:
         *
         * The initcall mechanism specification must stay separate from the
         * concrete side effects of individual entries such as
         * of_platform_default_populate_init(). Concrete targets are bound by
         * entry operation facts and may remain deferred until their owning
         * object model exists.
         */
        arceos_ex_must_initcall_keep_static_mechanism_separate_from_entry_effects();

        /*
         * OF platform default populate source:
         *
         * of_platform_default_populate_init() belongs to PlatformBus and must
         * use the already Ready DeviceTree as its source object. The action is
         * a source-to-target populate boundary from DeviceTree to PlatformBus,
         * not an InitcallTable mechanism detail.
         */
        arceos_ex_must_of_platform_default_populate_use_device_tree_source();

        /*
         * Linux-like traversal:
         *
         * The initial implementation must follow Linux 6.12
         * drivers/of/platform.c: of_platform_default_populate(NULL, ...)
         * resolves root to "/", then of_platform_populate() iterates the
         * root's direct children and calls of_platform_bus_create() with
         * strict=true.
         */
        arceos_ex_must_of_platform_default_populate_follow_linux_root_child_traversal();

        /*
         * Strict compatible:
         *
         * Candidate identification must require a compatible property for each
         * node considered by of_platform_bus_create(strict=true). Nodes without
         * compatible are skipped.
         */
        arceos_ex_must_of_platform_default_populate_require_compatible_strict();

        /*
         * Default bus recursion:
         *
         * After identifying a compatible candidate, recursion into children
         * must occur only for nodes matching the Linux default bus match table:
         * simple-bus, simple-mfd, isa, and config-gated arm,amba-bus if the
         * target later enables that path.
         */
        arceos_ex_must_of_platform_default_populate_recurse_default_bus_matches();

        /*
         * Availability filter:
         *
         * Candidate identification must mirror of_device_is_available(): a node
         * is available when status is absent or is exactly "okay" or "ok".
         */
        arceos_ex_must_of_platform_default_populate_filter_available_nodes();

        /*
         * Candidate printout:
         *
         * For this modeling step the action must print every identified
         * candidate's node name and compatible value so the traversal result is
         * inspectable from smoke/KUnit output.
         */
        arceos_ex_must_of_platform_default_populate_print_candidate_identity();

        /*
         * Action checkpoint naming:
         *
         * Action checkpoints must use Entry for action entry, Exit for action
         * return boundary, and semantic names for necessary middle points.
         * Ambiguous names such as Called must not be used for postcondition
         * or middle checkpoints.
         */
        arceos_ex_must_action_checkpoints_use_entry_exit_and_semantic_points();

        /*
         * Device model naming:
         *
         * The formal DeviceType corresponds to Linux struct device, not Linux
         * struct device_type. Linux struct platform_device must be represented
         * as PlatformDeviceType embedding a core DeviceType member, not as a
         * subtype of the core device object itself.
         */
        arceos_ex_must_device_type_model_linux_struct_device_not_device_type_descriptor();

        /*
         * Platform device core-member lookup:
         *
         * PlatformDeviceType coding must preserve a container_of-like
         * conversion from the embedded DeviceRef back to the owning platform
         * device, exposed through a Rust macro or equivalent typed helper such
         * as to_platform_device!().
         */
        arceos_ex_must_platform_device_embed_device_and_support_container_lookup();

        /*
         * DeviceObject category shell:
         *
         * DeviceObject is only an early object category label. Reusable
         * driver-core types such as DeviceType, BusType, and BusSubsysPrivate
         * must carry their own semantics directly instead of inheriting from
         * DeviceObject.
         */
        arceos_ex_must_device_object_kind_not_define_driver_core_semantics();

        /*
         * device_set_node boundary:
         *
         * DeviceType.SetNode must model Linux device_set_node()/dev.of_node by
         * binding a core device to a DeviceNodeRef. It must not copy the OF
         * compatible property into DeviceType or PlatformDeviceType; later
         * probe/match must reach compatible through the bound DeviceNodeRef.
         * The concrete Rust object must store a stable DeviceNodeId/node-index
         * or equivalent handle, not a long-lived borrowed DeviceNodeRef<'dt>.
         * That id must be resolved through the persistent DeviceTree whenever
         * name, compatible, status, or other OF properties are needed.
         */
        arceos_ex_must_device_set_node_bind_ref_without_copying_compatible();
        arceos_ex_must_device_node_ref_store_stable_handle_not_borrowed_view();
        arceos_ex_must_device_node_id_resolve_through_persistent_device_tree();
        arceos_ex_must_device_and_platform_device_store_node_id_not_borrowed_ref();

        /*
         * Platform device ownership:
         *
         * PlatformBus must own the PlatformDevice objects it creates during OF
         * population. The current arceos_ex backing must provide stable
         * platform-device storage, so DeviceRef entries in klist_devices never
         * outlive their containing PlatformDevice storage and are not
         * invalidated by container growth. A plain Vec<PlatformDevice> is not
         * sufficient if DeviceRef is a borrowed/raw reference to an embedded
         * Device member; use stable handles, arena-style ids, or non-moving
         * owned storage such as pinned/boxed platform devices. The platform
         * device must be inserted into owned storage before its DeviceRef is
         * published to klist_devices.
         */
        arceos_ex_must_of_platform_default_populate_require_dynamic_container_runtime_ready();
        arceos_ex_must_platform_bus_own_platform_devices_with_stable_storage();
        arceos_ex_must_platform_device_creation_use_stable_device_node_id();
        arceos_ex_must_platform_device_storage_keep_device_refs_stable_across_vec_growth();
        arceos_ex_must_platform_bus_insert_owned_platform_device_before_klist_device_ref();

        /*
         * Bus device set storage:
         *
         * The model-level BusSubsysPrivate.klist_devices is a DeviceRefSet.
         * The first arceos_ex backing may use Vec<DeviceRef> as an append and
         * iterate view, not a small action-smoke slot array. A future
         * Linux-like intrusive-list backing must not expose the raw intrusive
         * list as the lifetime owner: raw nodes only express membership. It
         * must be wrapped by a SafeIntrusiveList-like abstraction that couples
         * stable object storage with the raw list, hides raw nodes from public
         * APIs, unlinks before drop, and returns stable DeviceRef views.
         */
        arceos_ex_must_bus_device_ref_set_map_to_klist_like_storage();
        arceos_ex_must_platform_bus_klist_devices_use_vec_device_refs();
        arceos_ex_must_raw_intrusive_bus_list_not_own_device_lifetime();
        arceos_ex_must_future_intrusive_bus_list_wrap_storage_as_safe_intrusive_list();
        arceos_ex_must_safe_intrusive_list_hide_raw_nodes_and_return_stable_device_refs();

        /*
         * OF platform scan completion:
         *
         * The currently tested OF platform checkpoint must be named
         * OfPlatformDefaultPopulate.ScanComplete and must be emitted after
         * candidates have been identified and their name/compatible pairs have
         * been printed. KUnit coverage for candidate facts must attach to this
         * checkpoint, not to an Entry checkpoint.
         */
        arceos_ex_must_of_platform_default_populate_scan_complete_after_candidate_print();
        arceos_ex_must_of_platform_default_populate_emit_entry_scan_devices_exit_checkpoints();

        /*
         * OF platform device creation:
         *
         * After candidate scanning, of_platform_default_populate_init must
         * create PlatformDeviceType instances from candidate nodes, bind each
         * embedded DeviceType to its DeviceNodeRef, register the core device,
         * and add the resulting DeviceRef to PlatformBusSubsysPrivate's
         * DeviceRefSet through BusType.AddDevice. Driver match/probe/bind is
         * outside this OF-populate action and belongs to later driver
         * registration/probe initcalls.
         */
        arceos_ex_must_of_platform_default_populate_create_and_register_platform_devices();
        arceos_ex_must_of_platform_population_smoke_cover_refs_to_nodes();
        arceos_ex_must_of_platform_population_smoke_ignore_unrelated_initcall_entries();

        /*
         * Platform driver registration and probe:
         *
         * The first concrete platform-driver closure must be the real
         * ns16550a-compatible OF serial platform driver, modeled in
         * spec/model/common/ns16550a_driver.spec and implemented in an
         * independent Rust module. Its Linux-like device_initcall!() static
         * entry represents the initcall action. Concrete platform drivers
         * must declare that initcall in the driver's own implementation
         * module, beside the driver descriptor/probe code; the generic
         * initcall core only provides the declaration macros, linker-section
         * table collection, and level-order execution. That action must call
         * platform_driver_register() after OF population has created platform
         * devices; platform_driver_register() then reaches BusType.AddDriver,
         * records a DeviceDriverRef in PlatformBus.klist_drivers, and drives
         * ProbeDriver when autoprobe is active. ProbeDevice remains the
         * symmetric path for devices added after drivers.
         *
         * PlatformBus.klist_drivers stores DeviceDriverRef membership entries.
         * It may be backed by Vec<DeviceDriverRef> in the current target,
         * mirroring the earlier klist_devices compromise. The Vec owns only
         * copyable/stable refs, not driver objects.
         *
         * DeviceDriverType.of_match_table is a static descriptor field, not a
         * runtime setter. The concrete target must represent each registered
         * driver with a stable descriptor carrying name, bus binding,
         * of_match_table and probe function. The OF match table should lower
         * to a static compatible array analogous to Linux
         * struct of_device_id[], and DeviceDriverRef/klist_drivers must
         * reference that descriptor rather than acting as a closed enum of
         * bus-specific special cases. The referenced driver must be owned by
         * static storage, pinned heap storage, or an arena/registry with stable
         * handles; a plain Vec<PlatformDriver> is not valid if refs can point
         * into elements that may move during growth.
         *
         * PlatformBus.match() must be the common matching boundary: it reads
         * driver.of_match_table and resolves device.dev.of_node through the
         * persistent DeviceTree, then compares compatible strings. The
         * ns16550a driver may provide a static descriptor and probe function,
         * but the platform bus implementation must not hard-code an ns16550a
         * compatible branch as the only matching path.
         *
         * Platform driver probe must receive a PlatformProbeContext-style
         * temporary window derived from Context for the single probe action.
         * The probe context is non-owning and must not be stored by drivers.
         * It must not be a public bag of Context fields: fields stay private
         * and driver code reaches subsystems only through narrow production
         * capability methods such as OF node lookup, platform-device MMIO
         * mapping, IRQ binding, and virtio device registration. Platform
         * probe functions must not receive raw &mut Context, and PlatformBus
         * public APIs must not expose virtio-specific or IRQ/MM allocator
         * argument lists.
         */
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

        /*
         * Console/earlycon handoff:
         *
         * The next platform-driver increment must model Linux-like handoff
         * without jumping directly to a full UART backend. DeviceTree must
         * parse /chosen/stdout-path or linux,stdout-path, split optional
         * colon options, and resolve the selected node to a stable
         * DeviceNodeId. The ns16550a probe must then parse resources from the
         * matching PlatformDevice -> Device -> DeviceNodeId path, create a
         * minimal Uart8250Port object, and register a Serial8250Console only
         * when the probed device is the stdout-path device.
         *
         * The console registry must expose register_console()-style policy:
         * real serial console registration switches the printk route and
         * unregisters BootConsole unless keep_bootcon is set. The first
         * implementation may record facts for the route switch before
         * replacing the actual sink with UART MMIO polling writes.
         *
         * EarlyCon is the early SBI backend. BootConsole is the CON_BOOT
         * registry entry wrapping that backend. Serial8250Console is the real
         * console entry from the probed Uart8250Port. ConsoleRegistry owns the
         * route, handoff cursor transfer and keep_bootcon policy; drivers only
         * request registration and must not carry those global facts as
         * private state.
         */
        arceos_ex_must_device_tree_parse_stdout_path_from_chosen();
        arceos_ex_must_device_tree_preserve_stdout_path_options_after_colon();
        arceos_ex_must_device_tree_stdout_path_resolve_to_stable_node_id();
        arceos_ex_must_ns16550a_probe_parse_resources_from_platform_device_node();
        arceos_ex_must_ns16550a_probe_create_uart8250_port_object();
        arceos_ex_must_ns16550a_probe_register_serial8250_console_only_for_stdout_path();
        /*
         * devfs first slice:
         *
         * DevFs must be an InitcallPhase object, not part of the initial
         * ProcessPreparePhase rootfs mount. It must mount /dev only after the
         * initial VFS rootfs exists and the hwrng/block registries plus their
         * live virtio drivers have published current/default device surfaces.
         * The first smoke validation must observe hwrng and block device nodes
         * and registry bindings only; it must not add test-only production APIs
         * and must not require reads through a VFS file path.
         */
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
        /*
         * Model path:
         *
         * RootfsPhase is SMP Runtime Phase subphase 4. Its formal model path
         * is spec/model/smp-runtime/rootfs/.
         */
        arceos_ex_must_rootfs_model_path_under_smp_runtime_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.
         */
        arceos_ex_must_rootfs_code_path_follow_smp_runtime_phase_tree();

        /*
         * Entry gate:
         *
         * RootfsPhase must run after InitcallPhase.Ready and preserve the
         * kunit_run_all_tests() entry position inside RootfsPhase.
         */
        arceos_ex_must_rootfs_run_after_initcall();

        /*
         * KUnit runtime:
         *
         * CONFIG_KUNIT=n in the current Linux-like configuration, so
         * kunit_run_all_tests() must be encoded as a trimmed/no-op object
         * inside RootfsPhase, not as a standalone KUnitPhase.
         */
        arceos_ex_must_rootfs_keep_kunit_trimmed_inside_rootfs_phase();

        /*
         * Deferred initramfs and console details:
         *
         * wait_for_initramfs() and console_on_rootfs() must preserve their
         * Linux order but remain deferred in this round. wait_for_initramfs()
         * must explicitly preserve Linux's async cookie/domain wait boundary;
         * console_on_rootfs() must preserve the /dev/console and PID 1 fd
         * duplication position without pretending the file path is implemented.
         */
        arceos_ex_must_rootfs_keep_initramfs_and_console_deferred();

        /*
         * Required branch checkpoint:
         *
         * init_eaccess(ramdisk_execute_command) must force the supported
         * Linux-like path toward prepare_namespace().
         */
        arceos_ex_must_rootfs_require_prepare_namespace_branch();

        /*
         * RootFS enable:
         *
         * prepare_namespace() must be represented by RootFS.Transition::Enable in
         * this round, not by a separate enable-position object. It must use the
         * already mounted DevFs and the
         * BlockDeviceRegistry default device as the root device candidate,
         * construct the Ext2Driver/Ext2Volume/Ext2FileSystem chain, and mount
         * that ext2 filesystem at Linux's temporary /root mount point.
         * The initial ramfs-backed rootfs mount belongs to ProcessPreparePhase
         * vfs_caches_init()/mnt_init(), not to this RootfsPhase enable step.
         */
        arceos_ex_must_rootfs_prepare_namespace_inputs_use_existing_devfs_and_block_registry();
        arceos_ex_must_rootfs_mount_ext2_at_linux_root_staging_point();

        /*
         * prepare_namespace() path classification:
         *
         * RootfsPhase must keep prepare_namespace() as one formal subphase
         * event, but it must still expose a structured
         * RootfsPrepareNamespacePaths-style fact set for the Linux calls around
         * the supported block-root path. The implementation and smoke observer
         * must distinguish:
         *
         * - root_delay/rootwait as cmdline-absent trimmed paths, with the
         *   root_wait polling protocol deferred rather than erased;
         * - wait_for_device_probe() as deferred, including probe_count atomic,
         *   probe_waitqueue, deferred_probe_work flush, and worker interaction;
         * - md_run_setup() as deferred under CONFIG_MD=y;
         * - initrd_load() as trimmed under CONFIG_BLK_DEV_INITRD=n;
         * - NFS root as deferred under CONFIG_ROOT_NFS=y but inactive for the
         *   current root device, CIFS root as trimmed when CONFIG_CIFS_ROOT=n,
         *   and nodev root as deferred;
         * - Linux CONFIG_EXT2_FS=n / CONFIG_EXT4_USE_FOR_EXT2=y versus the
         *   arceos_ex Ext2Driver substitution;
         * - devtmpfs_mount() as deferred under CONFIG_DEVTMPFS=y, while the
         *   current arceos_ex DevFs is not remounted below the new ext2 root.
         */
        arceos_ex_must_rootfs_classify_prepare_namespace_paths();

        /*
         * Root switch:
         *
         * After the Linux-like temporary /root ext2 staging mount,
         * RootFS.Transition::Enable must drive VfsCore.Action::MoveMountToRoot and
         * FsStruct.Action::ChrootDot as separate model actions. FsStruct owns
         * the task-visible root/pwd dentry refs; VfsCore must not keep a
         * parallel current-root singleton. Smoke must observe current root as
         * ext2 and read /etc/alpine-release directly without remounting /dev
         * under the new root.
         */
        arceos_ex_must_rootfs_move_ext2_mount_and_chroot_dot_as_separate_actions();

        /*
         * Integrity keys:
         *
         * integrity_load_keys() must preserve CONFIG_INTEGRITY=y timing but
         * keep IMA/EVM keyring and certificate loading details deferred. The
         * current CONFIG_IMA=n and CONFIG_EVM=n trimmed facts must be visible
         * in model and implementation checks.
         */
        arceos_ex_must_rootfs_keep_integrity_keys_deferred_only();

        /*
         * Boundary:
         *
         * RootfsBoundary must mark the next boundary as FinalizePhase.
         */
        arceos_ex_must_rootfs_boundary_handoff_to_finalize();
    }
}

type ArceosExFinalizeCodingMust {
    invariant {
        /*
         * Model path:
         *
         * FinalizePhase is SMP Runtime Phase subphase 5. Its formal model
         * path is spec/model/smp-runtime/finalize/.
         */
        arceos_ex_must_finalize_model_path_under_smp_runtime_phase();

        /*
         * Code path:
         *
         * Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.
         */
        arceos_ex_must_finalize_code_path_follow_smp_runtime_phase_tree();

        /*
         * Entry gate:
         *
         * FinalizePhase must run after RootfsPhase.Ready and preserve the
         * kernel_init_freeable() return boundary before PayloadPhase.
         */
        arceos_ex_must_finalize_run_after_rootfs_phase();

        /*
         * Deferred cleanup details:
         *
         * async_synchronize_full(), ftrace/free_initmem, mark_readonly() and
         * do_sysctl_args() must preserve Linux order but remain deferred in
         * this round. async_synchronize_full() must still expose the
         * async_done waitqueue, async_lock irqsave spinlock, entry_count
         * atomic and ASYNC_COOKIE_MAX ordering responsibilities as deferred
         * facts.
         */
        arceos_ex_must_finalize_keep_cleanup_details_deferred();

        /*
         * Trimmed current-config paths:
         *
         * kprobe_free_init_mem(), kgdb_free_init_mem(), exit_boot_config(),
         * pti_finalize() and numa_default_policy() must be recorded as
         * trimmed/no-op under the current RISC-V/default configuration.
         * The numa_default_policy() checkpoint belongs after SYSTEM_RUNNING
         * in FinalizePhase, not in RestInitPhase.
         */
        arceos_ex_must_finalize_record_trimmed_config_paths();

        /*
         * System state:
         *
         * SystemState.enable() must publish the SYSTEM_FREEING_INITMEM window
         * and end with SystemState.state == Online and value == SYSTEM_RUNNING.
         * RcuCore.end_inkernel_boot() must expose rcu_unexpedite_gp() atomic
         * decrement, CONFIG_RCU_LAZY related rcu_async_relax() trimming,
         * rcu_normal_after_boot WRITE_ONCE handling and rcu_boot_ended publish
         * as observable facts.
         */
        arceos_ex_must_finalize_publish_system_running();

        /*
         * RCU boot end:
         *
         * RcuCore.end_inkernel_boot() must record rcu_boot_ended == true
         * without claiming full runtime RCU GP service implementation.
         */
        arceos_ex_must_finalize_end_rcu_inkernel_boot();

        /*
         * Boundary:
         *
         * FinalizeBoundary must mark the next boundary as PayloadPhase.
         */
        arceos_ex_must_finalize_boundary_handoff_to_payload();
    }
}
