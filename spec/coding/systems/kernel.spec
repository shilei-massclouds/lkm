/*
 * Kernel system coding formal rule index
 *
 * Formal entry: keep predicate/type names, rule grouping, and compact labels here.
 * Explanatory rationale, examples, and implementation notes live in kernel.md.
 */

predicate kernel_system_coding_maps_to_arceos_ex_system_module() -> bool;
predicate kernel_system_coding_implements_kernel_lifecycle_boundary() -> bool;
predicate kernel_system_coding_preserves_model_lifecycle_states() -> bool;
predicate kernel_system_coding_owns_runtime_phase_order() -> bool;
predicate kernel_system_coding_uses_kernel_system_state() -> bool;
predicate kernel_system_coding_does_not_reorder_phase_calls() -> bool;
predicate kernel_system_coding_does_not_reassign_phase_ownership() -> bool;
predicate kernel_system_coding_preserves_payload_handoff_boundary() -> bool;
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

type KernelSystemCoding {
    invariant {
        /* Mapping path. */
        kernel_system_coding_maps_to_arceos_ex_system_module();

        /* Kernel lifecycle boundary. */
        kernel_system_coding_implements_kernel_lifecycle_boundary();
        kernel_system_coding_preserves_model_lifecycle_states();

        /* Runtime choreography. */
        kernel_system_coding_owns_runtime_phase_order();

        /* Kernel state. */
        kernel_system_coding_uses_kernel_system_state();

        /* Behavior preservation. */
        kernel_system_coding_does_not_reorder_phase_calls();

        /* Phase ownership. */
        kernel_system_coding_does_not_reassign_phase_ownership();

        /* Payload handoff. */
        kernel_system_coding_preserves_payload_handoff_boundary();
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
