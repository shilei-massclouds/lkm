/*
 * Block I/O coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in block-io.md.
 */

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
predicate arceos_ex_must_ext2_defer_page_cache_indirect_and_writes() -> bool;

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
