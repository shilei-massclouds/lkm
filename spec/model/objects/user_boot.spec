/*
 * First user-mode program bootstrap model.
 *
 * This slice models the shortest Linux-like path from KernelInitFlow's split
 * payload leaves to the first user-mode program. The selected payload variant
 * is UserBootPayload.
 * It follows Linux 6.12 init/main.c::kernel_init() after do_sysctl_args(): the
 * ramdisk init branch is trimmed because CONFIG_BLK_DEV_INITRD is disabled;
 * execute_command from init= is a requested-init branch that runs before
 * CONFIG_DEFAULT_INIT and before the default fallback list. If init= is
 * present, the requested path is tried once; success stops the startup
 * orchestration, and failure is the Linux requested-init panic boundary rather
 * than a fallthrough to /sbin/init. If init= is absent, CONFIG_DEFAULT_INIT is
 * empty in the current configuration, and the default fallback list is tried in
 * Linux order: /sbin/init, /etc/init, /bin/init, then /bin/sh. The first
 * candidate that can be read from the current VFS root and accepted by the
 * current executable loader becomes the selected path. The selected file is
 * treated as an
 * ElfObject, mapped into a UserAddressSpace, paired with a UserStack and
 * UserTrapFrame, then entered in U-mode. Syscalls remain under the existing
 * CurrentCPU.trap.exception.syscall branch of CurrentCPU.trap.exception. CurrentCPU.trap.exception.syscall owns syscall
 * entry validation, argument extraction and dispatch selection; this slice only
 * adds SyscallTable as the minimal action table consumed by that branch.
 *
 * The current root disk image is a whole-disk ext2 filesystem. PartitionTable
 * and BlockPartition objects are intentionally deferred until the disk image
 * format actually includes a partition table.
 */

enum UserInitPathRef {
    /*
     * Historical name for the Linux fallback candidate "/sbin/init"; this is
     * not CONFIG_DEFAULT_INIT, whose current empty value is recorded by
     * ExecSyncBoundaries.
     */
    DefaultInit,
    EtcInit,
    BinInit,
    BinSh,
    RequestedInit,
}

enum UserInitAttemptStage {
    ValidatePath,
    ReadImage,
    PresetElf,
    SetupElf,
    UnsupportedCandidate,
}

enum UserInitAttemptReason {
    InvalidPath,
    ReadFailed,
    ElfPresetFailed,
    ElfSetupFailed,
    CandidateUnsupported,
}


enum UserCloneFirstSliceKind {
    PlainFork,
}

predicate user_boot_payload_selected<T>(payload: T) -> bool;
predicate user_boot_payload_candidates_bound<T>(payload: T) -> bool;
predicate user_boot_payload_default_init_path_bound<T>(payload: T) -> bool;
predicate user_boot_payload_default_init_fallback_order_bound<T>(payload: T) -> bool;
predicate user_boot_payload_default_init_candidate_bound<T, P>(payload: T, path: P) -> bool;
predicate user_boot_payload_requested_init_branch_bound<T>(payload: T) -> bool;
predicate user_boot_payload_requested_init_before_default_fallback<T>(payload: T) -> bool;
predicate user_boot_payload_requested_init_failure_panic_terminal_bound<T>(payload: T) -> bool;
predicate user_boot_payload_requested_init_no_default_fallback<T>(payload: T) -> bool;
predicate user_boot_payload_candidate_failure_nonfatal_for_fallback<T>(payload: T) -> bool;
predicate user_boot_payload_first_successful_candidate_selected<T>(payload: T) -> bool;
predicate user_boot_payload_success_stops_fallback_chain<T>(payload: T) -> bool;
predicate user_boot_payload_success_no_return_to_startup_orchestration<T>(payload: T) -> bool;
predicate user_boot_payload_no_working_init_panic_terminal_bound<T>(payload: T) -> bool;
predicate user_boot_payload_uses_current_fs_struct<T, F>(payload: T, fs: F) -> bool;
predicate user_boot_payload_reads_init_from_vfs<T, V>(payload: T, vfs: V) -> bool;
predicate user_boot_payload_no_partition_dependency<T>(payload: T) -> bool;
predicate user_boot_payload_partition_objects_deferred<T>(payload: T) -> bool;
predicate user_boot_payload_driven_by_kernel_init_task<T, K>(payload: T, task: K) -> bool;
predicate user_boot_payload_try_candidate_bound<T>(payload: T) -> bool;
predicate user_boot_payload_selected_path_bound<T>(payload: T) -> bool;
predicate user_boot_payload_selected_argv0_path_bound<T>(payload: T) -> bool;
predicate user_boot_payload_stdin_probe_ready_data_cleared<T>(payload: T) -> bool;
predicate user_boot_payload_user_smoke_stdin_fixture_only<T>(payload: T) -> bool;
predicate user_boot_payload_distro_init_no_stdin_fixture<T>(payload: T) -> bool;
predicate user_boot_payload_distro_stdin_blocking_wait_enabled<T>(payload: T) -> bool;
predicate user_boot_payload_user_smoke_stdin_blocking_wait_opt_out<T>(payload: T) -> bool;
predicate user_boot_payload_rc_local_direct_inittab_diagnostic_first_slice<T>(payload: T) -> bool;
predicate user_boot_payload_try_candidate_read_init<T, V>(payload: T, vfs: V) -> bool;
predicate user_boot_payload_try_candidate_elf_ready<T, E>(payload: T, elf: E) -> bool;
predicate user_boot_payload_init_attempt_failure_trace_defined<T>(payload: T) -> bool;
predicate user_boot_payload_init_attempt_failure_stage_bound<T, S>(payload: T, stage: S) -> bool;
predicate user_boot_payload_init_attempt_failure_reason_bound<T, R>(payload: T, reason: R) -> bool;
predicate user_boot_payload_init_attempt_failure_path_bound<T, P>(payload: T, path: P) -> bool;
predicate user_boot_payload_init_attempt_failure_requested_terminal<T>(payload: T) -> bool;
predicate user_boot_payload_init_attempt_failure_default_nonfatal<T>(payload: T) -> bool;
predicate user_boot_init_attempt_failure_checkpoint<T>(payload: T) -> bool;
predicate user_boot_payload_enters_user_mode<T>(payload: T) -> bool;
predicate user_boot_payload_no_return_handoff<T>(payload: T) -> bool;
predicate payload_image_read_start_checkpoint<T, P>(payload: T, path: P) -> bool;
predicate payload_image_read_complete_checkpoint<T, V>(payload: T, vfs: V) -> bool;
predicate payload_image_read_failed_checkpoint_defined<T>(payload: T) -> bool;
predicate payload_image_read_error_classification_contract_ready<T>(payload: T) -> bool;


predicate user_clone_deferred_boundaries_ready<T>(boundaries: T) -> bool;
predicate user_clone_linux_6_12_legacy_clone_bound<T>(boundaries: T) -> bool;
predicate user_clone_riscv_abi_argument_order_bound<T>(boundaries: T) -> bool;
predicate user_clone_observed_plain_fork_args_bound<T>(boundaries: T) -> bool;
predicate user_clone_observed_vfork_vm_args_bound<T>(boundaries: T) -> bool;
predicate user_clone_observed_vfork_pidfd_args_bound<T>(boundaries: T) -> bool;
predicate user_clone_plain_fork_first_slice_bound<T>(boundaries: T) -> bool;
predicate user_clone_vfork_vm_first_slice_bound<T>(boundaries: T) -> bool;
predicate user_clone_vfork_pidfd_first_slice_bound<T>(boundaries: T) -> bool;
predicate user_clone_fresh_task_per_child_bound<T>(boundaries: T) -> bool;
predicate user_clone_multiple_independent_tasks_bound<T>(boundaries: T) -> bool;
predicate user_clone_csignal_split_bound<T>(boundaries: T) -> bool;
predicate user_clone_sigchld_exit_signal_bound<T>(boundaries: T) -> bool;
predicate user_clone_newsp_zero_inherits_parent_sp<T>(boundaries: T) -> bool;
predicate user_clone_newsp_sets_child_sp<T>(boundaries: T) -> bool;
predicate user_clone_legacy_pidfd_uses_parent_tidptr<T>(boundaries: T) -> bool;
predicate user_clone_tls_ignored_without_clone_settls<T>(boundaries: T) -> bool;
predicate user_clone_full_clone3_deferred<T>(boundaries: T) -> bool;
predicate user_clone_full_thread_group_deferred<T>(boundaries: T) -> bool;
predicate user_clone_vfork_clone_vm_parent_cpu_serial<T>(boundaries: T) -> bool;
predicate user_clone_vfork_task_local_aggregate<T>(boundaries: T) -> bool;
predicate user_clone_vfork_task_local_parent_completion<T>(boundaries: T) -> bool;
predicate user_clone_vfork_effective_mm_generation_checked<T>(boundaries: T) -> bool;
predicate user_clone_vfork_nested_handoffs_independent<T>(boundaries: T) -> bool;
predicate user_clone_vfork_exec_detaches_without_retiring_parent_mm<T>(boundaries: T) -> bool;
predicate user_clone_vfork_exit_wakes_without_releasing_parent_mm<T>(boundaries: T) -> bool;
predicate user_clone_plain_fork_resolves_current_mm_binding<T>(boundaries: T) -> bool;
predicate user_clone_independent_mm_cow_bound<T>(boundaries: T) -> bool;
predicate user_clone_cow_private_frame_sharing_bound<T>(boundaries: T) -> bool;
predicate user_clone_cow_parent_child_ro_lowered<T>(boundaries: T) -> bool;
predicate user_clone_cow_readonly_shared_without_promotion<T>(boundaries: T) -> bool;
predicate user_clone_cow_prepare_before_commit<T>(boundaries: T) -> bool;
predicate user_clone_cow_publish_after_commit<T>(boundaries: T) -> bool;
predicate user_clone_cow_failure_atomic<T>(boundaries: T) -> bool;
predicate user_clone_cow_teardown_refcount_conserving<T>(boundaries: T) -> bool;
predicate user_clone_full_pidfd_file_ops_deferred<T>(boundaries: T) -> bool;
predicate user_clone_full_ptrace_hooks_deferred<T>(boundaries: T) -> bool;
predicate user_clone_full_seccomp_hooks_deferred<T>(boundaries: T) -> bool;
predicate user_clone_full_cgroup_hooks_deferred<T>(boundaries: T) -> bool;
predicate user_clone_full_audit_hooks_deferred<T>(boundaries: T) -> bool;
predicate user_clone_full_namespace_deferred<T>(boundaries: T) -> bool;
predicate user_clone_full_robust_futex_deferred<T>(boundaries: T) -> bool;
predicate user_clone_full_clear_child_tid_wake_deferred<T>(boundaries: T) -> bool;
predicate user_clone_wait_sleep_wakeup_cross_cpu<T>(boundaries: T) -> bool;
predicate user_clone_zombie_reap_accounting_cross_cpu<T>(boundaries: T) -> bool;
predicate user_clone_online_cpu_sequence_frozen<T>(boundaries: T) -> bool;
predicate user_clone_pid1_cpu0_bound<T>(boundaries: T) -> bool;
predicate user_clone_plain_fork_pid_mod_cpu_placement<T>(boundaries: T) -> bool;
predicate user_clone_cpu_ownership_immutable<T>(boundaries: T) -> bool;
predicate user_clone_max_concurrent_user_tasks_32<T>(boundaries: T) -> bool;
predicate user_clone_max_online_cpus_16<T>(boundaries: T) -> bool;
predicate user_clone_fork_failure_eagain_or_enomem<T>(boundaries: T) -> bool;
predicate user_clone_fork_failure_no_partial_publication<T>(boundaries: T) -> bool;
predicate user_clone_shared_mm_cross_cpu_deferred<T>(boundaries: T) -> bool;
predicate user_clone_full_job_control_deferred<T>(boundaries: T) -> bool;
predicate user_clone_unsupported_flags_first_slice<T>(boundaries: T) -> bool;


predicate user_address_space_allocated<T>(space: T) -> bool;
predicate user_address_space_first_instance<T>(space: T) -> bool;
predicate user_address_space_bound_to_kernel_init_task<T, K>(space: T, task: K) -> bool;
predicate kernel_init_task_first_user_address_space_bound<K, T>(task: K, space: T) -> bool;
predicate user_address_space_low_half_private<T>(space: T) -> bool;
predicate user_address_space_high_half_shares_swapper<T, S>(space: T, swapper: S) -> bool;
predicate user_address_space_kernel_pages_u_disabled<T>(space: T) -> bool;
predicate user_address_space_user_pages_u_enabled<T>(space: T) -> bool;
predicate user_address_space_elf_segments_mapped<T, E>(space: T, elf: E) -> bool;
predicate user_address_space_interpreter_segments_mapped<T, E>(space: T, interpreter: E) -> bool;
predicate user_address_space_stack_mapped<T, S>(space: T, stack: S) -> bool;
predicate user_address_space_heap_arena_mapped<T>(space: T) -> bool;
predicate user_address_space_elf_load_plan_consumed<T, E>(space: T, elf: E) -> bool;
predicate user_address_space_interpreter_load_plan_consumed<T, E>(space: T, interpreter: E) -> bool;
predicate user_address_space_segment_mappings_bound<T>(space: T) -> bool;
predicate user_address_space_entry_mapping_executable<T>(space: T) -> bool;
predicate user_address_space_bss_zero_plan_consumed<T, E>(space: T, elf: E) -> bool;
predicate user_address_space_backing_pages_allocated<T>(space: T) -> bool;
predicate user_address_space_elf_file_bytes_copied<T, E>(space: T, elf: E) -> bool;
predicate user_address_space_bss_bytes_zeroed<T, E>(space: T, elf: E) -> bool;
predicate user_address_space_page_table_view_ready<T>(space: T) -> bool;
predicate user_address_space_real_page_table_allocated<T>(space: T) -> bool;
predicate user_address_space_user_leaf_ptes_installed<T>(space: T) -> bool;
predicate user_address_space_high_half_root_entries_shared<T, S>(space: T, swapper: S) -> bool;
predicate user_address_space_satp_token_ready<T>(space: T) -> bool;
predicate user_address_space_prepared_but_not_current<T>(space: T) -> bool;
predicate user_address_space_runtime_ready<T>(space: T) -> bool;
predicate user_address_space_brk_window_bound<T>(space: T) -> bool;
predicate user_address_space_mmap_anonymous_window_bound<T>(space: T) -> bool;
predicate user_address_space_mmap_fixed_heap_base_compat_bound<T>(space: T) -> bool;
predicate user_address_space_mprotect_accepts_mapped_user_range<T>(space: T) -> bool;
predicate user_address_space_munmap_accepts_mapped_user_range<T>(space: T) -> bool;
predicate user_address_space_munmap_anonymous_private_whole_vma<T>(space: T) -> bool;
predicate user_address_space_munmap_removes_leaves_backing_and_vma<T>(space: T) -> bool;
predicate user_address_space_munmap_range_and_slot_reusable<T>(space: T) -> bool;
predicate user_address_space_munmap_rejects_partial_or_nonanonymous_atomically<T>(space: T) -> bool;
predicate user_address_space_vmas_nonoverlapping<T>(space: T) -> bool;
predicate user_address_space_heap_demand_paged<T>(space: T) -> bool;
predicate user_address_space_anonymous_private_demand_paged<T>(space: T) -> bool;
predicate user_address_space_fault_request_task_mm_bound<T>(space: T) -> bool;
predicate user_address_space_terminal_handoff_fault_request_uses_committed_parent_task<T>(space: T) -> bool;
predicate user_address_space_fault_access_classified<T>(space: T) -> bool;
predicate user_address_space_fault_classes_total_and_exclusive<T>(space: T) -> bool;
predicate user_address_space_fault_not_present_bound<T>(space: T) -> bool;
predicate user_address_space_fault_cow_write_protect_bound<T>(space: T) -> bool;
predicate user_address_space_fault_protection_bound<T>(space: T) -> bool;
predicate user_address_space_fault_unmapped_bound<T>(space: T) -> bool;
predicate user_address_space_fault_retry_preserves_sepc<T>(space: T) -> bool;
predicate user_address_space_fault_targeted_tlb_flush<T>(space: T) -> bool;
predicate user_address_space_fault_failure_atomic<T>(space: T) -> bool;
predicate user_address_space_fault_diagnostic_stable<T>(space: T) -> bool;
predicate user_address_space_fault_cow_requires_original_private_writable_vma<T>(space: T) -> bool;
predicate user_address_space_fault_cow_shared_copy_bound<T, M>(space: T, metadata_map: M) -> bool;
predicate user_address_space_fault_cow_unique_restore_bound<T, M>(space: T, metadata_map: M) -> bool;
predicate user_address_space_fault_cow_refcount_conserved<T, M>(space: T, metadata_map: M) -> bool;
predicate user_address_space_fault_cow_commit_failure_preserves_old_leaf<T, M>(space: T, metadata_map: M) -> bool;
predicate user_address_space_fault_cow_oom_terminal_bound<T>(space: T) -> bool;
predicate user_address_space_fault_cow_diagnostics_nonzero<T>(space: T) -> bool;
predicate user_address_space_user_frame_refs_conserved<T, M>(space: T, metadata_map: M) -> bool;
predicate user_address_space_owner_task_bound<T, K>(space: T, task: K) -> bool;
predicate user_address_space_aggregate_identity_stable<T>(space: T) -> bool;
predicate user_address_space_not_swapped_through_global_carrier<T>(space: T) -> bool;
predicate user_address_space_lease_checked_for_current_task_cpu<T>(space: T) -> bool;
predicate user_address_space_satp_committed_on_dispatch<T>(space: T) -> bool;
predicate user_address_space_local_sfence_after_satp<T>(space: T) -> bool;
predicate user_address_space_exec_replacement_atomic<T>(space: T) -> bool;
predicate user_address_space_fault_kernel_extable_isolated<T>(space: T) -> bool;
predicate user_address_space_fault_unmapped_segv_maperr_bound<T>(space: T) -> bool;
predicate user_address_space_fault_protection_segv_accerr_bound<T>(space: T) -> bool;
predicate user_address_space_fault_sigsegv_addr_exact<T>(space: T) -> bool;
predicate user_address_space_fault_sigsegv_no_sepc_advance<T>(space: T) -> bool;
predicate user_address_space_fault_sigsegv_task_terminal_bound<T>(space: T) -> bool;
predicate user_address_space_fault_sigsegv_usercopy_still_efault<T>(space: T) -> bool;
predicate swapper_vm_remains_kernel_shared_instance<T>(swapper: T) -> bool;

predicate user_trap_frame_allocated<T>(frame: T) -> bool;
predicate user_trap_frame_entry_bound<T, E>(frame: T, elf: E) -> bool;
predicate user_trap_frame_entry_uses_interpreter_when_present<T, E, I>(frame: T, elf: E, interpreter: I) -> bool;
predicate user_trap_frame_sp_bound<T, S>(frame: T, stack: S) -> bool;
predicate user_trap_frame_sstatus_user_mode<T>(frame: T) -> bool;
predicate user_trap_frame_fpu_initial<T>(frame: T) -> bool;
predicate user_trap_frame_fpu_context_switch_deferred<T>(frame: T) -> bool;
predicate user_trap_frame_sret_ready<T>(frame: T) -> bool;
predicate user_trap_frame_address_space_bound<T, A>(frame: T, space: A) -> bool;
predicate user_trap_frame_prepared_but_not_entered<T>(frame: T) -> bool;

predicate syscall_table_ready<T>(table: T) -> bool;
predicate syscall_table_bound_to_exception<T, E>(table: T, exception: E) -> bool;
predicate syscall_table_write_supported<T>(table: T) -> bool;
predicate syscall_table_writev_supported<T>(table: T) -> bool;
predicate syscall_table_openat_supported<T>(table: T) -> bool;
predicate syscall_table_chdir_supported<T>(table: T) -> bool;
predicate syscall_table_read_supported<T>(table: T) -> bool;
predicate syscall_table_ppoll_supported<T>(table: T) -> bool;
predicate syscall_table_close_supported<T>(table: T) -> bool;
predicate syscall_table_newfstatat_supported<T>(table: T) -> bool;
predicate syscall_table_readlinkat_supported<T>(table: T) -> bool;
predicate syscall_table_dup3_supported<T>(table: T) -> bool;
predicate syscall_table_pipe2_supported<T>(table: T) -> bool;
predicate syscall_table_fchown_supported<T>(table: T) -> bool;
predicate syscall_table_fchmod_supported<T>(table: T) -> bool;
predicate syscall_table_fcntl_supported<T>(table: T) -> bool;
predicate syscall_table_ioctl_supported<T>(table: T) -> bool;
predicate syscall_table_getrandom_supported<T>(table: T) -> bool;
predicate syscall_table_getuid_supported<T>(table: T) -> bool;
predicate syscall_table_getgid_supported<T>(table: T) -> bool;
predicate syscall_table_getpgid_supported<T>(table: T) -> bool;
predicate syscall_table_getsid_supported<T>(table: T) -> bool;
predicate syscall_table_setpgid_supported<T>(table: T) -> bool;
predicate syscall_table_setsid_supported<T>(table: T) -> bool;
predicate syscall_table_setuid_supported<T>(table: T) -> bool;
predicate syscall_table_setgid_supported<T>(table: T) -> bool;
predicate syscall_table_getgroups_supported<T>(table: T) -> bool;
predicate syscall_table_setgroups_supported<T>(table: T) -> bool;
predicate syscall_table_rt_sigprocmask_supported<T>(table: T) -> bool;
predicate syscall_table_rt_sigaction_supported<T>(table: T) -> bool;
predicate syscall_table_rt_sigtimedwait_supported<T>(table: T) -> bool;
predicate syscall_table_clock_gettime_supported<T>(table: T) -> bool;
predicate syscall_table_gettimeofday_supported<T>(table: T) -> bool;
predicate syscall_table_nanosleep_supported<T>(table: T) -> bool;
predicate syscall_table_brk_supported<T>(table: T) -> bool;
predicate syscall_table_mmap_supported<T>(table: T) -> bool;
predicate syscall_table_mprotect_supported<T>(table: T) -> bool;
predicate syscall_table_munmap_supported<T>(table: T) -> bool;
predicate syscall_table_set_tid_address_supported<T>(table: T) -> bool;
predicate syscall_table_clone_supported<T>(table: T) -> bool;
predicate syscall_table_execve_supported<T>(table: T) -> bool;
predicate syscall_table_wait4_supported<T>(table: T) -> bool;
predicate syscall_table_exit_supported<T>(table: T) -> bool;
predicate syscall_table_exit_group_supported<T>(table: T) -> bool;
predicate syscall_write_usercopy_ready<T>(table: T) -> bool;
predicate syscall_writev_usercopy_ready<T>(table: T) -> bool;
predicate syscall_read_usercopy_ready<T>(table: T) -> bool;
predicate syscall_fixed_usercopy_checks_mapping_permissions<T, A>(table: T, address_space: A) -> bool;
predicate syscall_getrandom_usercopy_ready<T>(table: T) -> bool;
predicate syscall_signal_mask_usercopy_ready<T>(table: T) -> bool;
predicate syscall_signal_action_usercopy_ready<T>(table: T) -> bool;
predicate syscall_time_usercopy_ready<T>(table: T) -> bool;
predicate syscall_path_usercopy_ready<T>(table: T) -> bool;
predicate syscall_stat_usercopy_ready<T>(table: T) -> bool;
predicate syscall_write_routes_to_console<T>(table: T) -> bool;
predicate syscall_writev_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_openat_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_openat_builtin_grandchild_dev_null_redirect_bound<T, F>(table: T, files: F) -> bool;
predicate syscall_getcwd_builtin_grandchild_absolute_pwd_bound<T, F, V>(table: T, fs: F, vfs: V) -> bool;
predicate files_struct_ltp_runtest_syscalls_read_capacity_bound<T>(files: T) -> bool;
predicate syscall_chdir_routes_to_fs_struct<T, F>(table: T, fs: F) -> bool;
predicate syscall_chdir_linux_6_12_path_walk_bound<T>(table: T) -> bool;
predicate syscall_chdir_root_first_slice<T>(table: T) -> bool;
predicate syscall_chdir_permissions_lsm_deferred<T>(table: T) -> bool;
predicate syscall_read_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_read_stdin_ready_data_first_slice<T>(table: T) -> bool;
predicate syscall_read_no_ready_blocking_out_of_slice<T>(table: T) -> bool;
predicate syscall_read_pipe_empty_blocks_while_writer_live<T>(table: T) -> bool;
predicate syscall_read_pipe_wait_uses_fixed_cpu_scheduler<T>(table: T) -> bool;
predicate syscall_read_pipe_wait_retries_after_wake<T>(table: T) -> bool;
predicate syscall_read_pipe_wait_no_lost_wake<T>(table: T) -> bool;
predicate syscall_read_tty_input_wait_first_slice<T>(table: T) -> bool;
predicate syscall_read_n_tty_line_discipline_first_slice<T>(table: T) -> bool;
predicate syscall_read_tty_read_wait_entry_first_slice<T>(table: T) -> bool;
predicate syscall_read_tty_read_wait_finish_first_slice<T>(table: T) -> bool;
predicate syscall_read_tty_blocking_deferred<T>(table: T) -> bool;
predicate syscall_ppoll_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_ppoll_pollfd_usercopy_ready<T>(table: T) -> bool;
predicate syscall_ppoll_ready_data_first_slice<T>(table: T) -> bool;
predicate syscall_ppoll_timeout_parse_first_slice<T>(table: T) -> bool;
predicate syscall_ppoll_sigmask_deferred<T>(table: T) -> bool;
predicate syscall_ppoll_no_ready_blocking_out_of_slice<T>(table: T) -> bool;
predicate syscall_ppoll_tty_input_wait_first_slice<T>(table: T) -> bool;
predicate syscall_ppoll_n_tty_readiness_first_slice<T>(table: T) -> bool;
predicate syscall_ppoll_poll_table_first_slice<T>(table: T) -> bool;
predicate syscall_ppoll_poll_freewait_first_slice<T>(table: T) -> bool;
predicate syscall_ppoll_blocking_wait_deferred<T>(table: T) -> bool;
predicate syscall_close_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_newfstatat_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_readlinkat_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_dup3_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_pipe2_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_pipe2_flags_zero_first_slice<T>(table: T) -> bool;
predicate syscall_pipe2_atomic_fd_and_usercopy_rollback<T>(table: T) -> bool;
predicate syscall_pipe2_full_linux_model_deferred<T>(table: T) -> bool;
predicate syscall_fchown_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_fchmod_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_fchown_fchmod_fd_local_first_slice<T>(table: T) -> bool;
predicate syscall_fchown_fchmod_full_linux_model_deferred<T>(table: T) -> bool;
predicate syscall_unsupported_socket_diagnostic_first_slice<T>(table: T) -> bool;
predicate syscall_unsupported_connect_sockaddr_diagnostic_first_slice<T>(table: T) -> bool;
predicate syscall_connect_supported<T>(table: T) -> bool;
predicate syscall_connect_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_connect_af_unix_pathname_failure_first_slice<T>(table: T) -> bool;
predicate syscall_socket_supported<T>(table: T) -> bool;
predicate syscall_socket_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_socket_af_unix_stream_first_slice<T>(table: T) -> bool;
predicate syscall_socket_backend_full_linux_model_deferred<T>(table: T) -> bool;
predicate syscall_fcntl_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_ioctl_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_ioctl_usercopy_ready<T>(table: T) -> bool;
predicate syscall_ioctl_tcgets_termios_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tcsets_termios_mutation_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tiocgpgrp_foreground_pgrp_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tiocspgrp_foreground_pgrp_update_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tiocgsid_session_id_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tiocsctty_controlling_tty_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tty_full_linux_model_deferred<T>(table: T) -> bool;
predicate syscall_getrandom_routes_to_hwrng_core<T, H>(table: T, hwrng: H) -> bool;
predicate syscall_getrandom_not_vfs_or_devfs_path<T>(table: T) -> bool;
predicate syscall_getrandom_flags_first_slice_bound<T>(table: T) -> bool;
predicate syscall_getrandom_full_random_core_deferred<T>(table: T) -> bool;
predicate syscall_getuid_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_getgid_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_getpgid_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_getsid_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_setpgid_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_setpgid_child_plain_fork_first_slice<T, P>(table: T, process: P) -> bool;
predicate syscall_setsid_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_setsid_process_group_leader_eperm_first_slice<T>(table: T) -> bool;
predicate syscall_setsid_child_success_first_slice<T, P>(table: T, process: P) -> bool;
predicate syscall_setuid_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_setgid_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_getgroups_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_getgroups_bounded_supplementary_groups_first_slice<T>(table: T) -> bool;
predicate syscall_setgroups_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_setgroups_root_first_slice<T>(table: T) -> bool;
predicate syscall_setgroups_bounded_supplementary_groups_first_slice<T>(table: T) -> bool;
predicate syscall_credentials_full_linux_model_deferred<T>(table: T) -> bool;
predicate syscall_rt_sigprocmask_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_rt_sigprocmask_sigsetsize_bound<T>(table: T) -> bool;
predicate syscall_rt_sigprocmask_unblockable_signals_cleared<T>(table: T) -> bool;
predicate syscall_rt_sigaction_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_rt_sigaction_routes_to_signal_action_table<T, P>(table: T, process: P) -> bool;
predicate syscall_rt_sigaction_sigsetsize_bound<T>(table: T) -> bool;
predicate syscall_rt_sigaction_layout_bound<T>(table: T) -> bool;
predicate syscall_rt_sigaction_unblockable_signals_cleared<T>(table: T) -> bool;
predicate syscall_rt_sigaction_kernel_only_signals_rejected<T>(table: T) -> bool;
predicate syscall_rt_sigtimedwait_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_rt_sigtimedwait_sigsetsize_bound<T>(table: T) -> bool;
predicate syscall_rt_sigtimedwait_copies_wait_mask<T>(table: T) -> bool;
predicate syscall_rt_sigtimedwait_uinfo_null_no_copyout_first_slice<T>(table: T) -> bool;
predicate syscall_rt_sigtimedwait_uts_null_infinite_wait_first_slice<T>(table: T) -> bool;
predicate syscall_rt_sigtimedwait_empty_pending_wait_boundary<T>(table: T) -> bool;
predicate syscall_rt_sigtimedwait_waitqueue_sleep_first_slice<T>(table: T) -> bool;
predicate syscall_rt_sigtimedwait_sigchld_pending_first_slice<T>(table: T) -> bool;
predicate syscall_rt_sigtimedwait_return_signal_first_slice<T>(table: T) -> bool;
predicate syscall_signal_delivery_deferred<T>(table: T) -> bool;
predicate syscall_clock_gettime_routes_to_timer_provider<T, P>(table: T, provider: P) -> bool;
predicate syscall_gettimeofday_routes_to_timer_provider<T, P>(table: T, provider: P) -> bool;
predicate syscall_nanosleep_routes_to_timer_provider<T, P>(table: T, provider: P) -> bool;
predicate syscall_clock_gettime_clockid_first_slice_bound<T>(table: T) -> bool;
predicate syscall_time_struct_layout_bound<T>(table: T) -> bool;
predicate syscall_nanosleep_usercopy_ready<T>(table: T) -> bool;
predicate syscall_nanosleep_timespec_validated<T>(table: T) -> bool;
predicate syscall_nanosleep_short_relative_first_slice<T>(table: T) -> bool;
predicate syscall_nanosleep_full_hrtimer_deferred<T>(table: T) -> bool;
predicate syscall_time_full_linux_model_deferred<T>(table: T) -> bool;
predicate syscall_brk_routes_to_user_address_space<T, A>(table: T, space: A) -> bool;
predicate syscall_mmap_routes_to_user_address_space<T, A>(table: T, space: A) -> bool;
predicate syscall_mmap_fixed_anonymous_prot_none_first_slice<T>(table: T) -> bool;
predicate syscall_mmap_full_vma_model_deferred<T>(table: T) -> bool;
predicate syscall_mprotect_routes_to_user_address_space<T, A>(table: T, space: A) -> bool;
predicate syscall_munmap_routes_to_user_address_space<T, A>(table: T, space: A) -> bool;
predicate syscall_set_tid_address_routes_to_user_task<T, P>(table: T, process: P) -> bool;
predicate syscall_clone_routes_to_task_creation_core<T, C>(table: T, core: C) -> bool;
predicate syscall_clone_routes_to_user_clone_deferred_boundaries<T, B>(table: T, boundaries: B) -> bool;
predicate syscall_clone_legacy_args_decoded<T>(table: T) -> bool;
predicate syscall_clone_plain_fork_first_slice<T>(table: T) -> bool;
predicate syscall_clone_plain_fork_from_serial_vfork_current_mm<T>(table: T) -> bool;
predicate syscall_clone_vfork_vm_first_slice<T>(table: T) -> bool;
predicate syscall_clone_vfork_pidfd_first_slice<T>(table: T) -> bool;
predicate syscall_clone_parent_returns_child_pid<T, P>(table: T, process: P) -> bool;
predicate syscall_clone_child_return_zero_bound<T, P>(table: T, process: P) -> bool;
predicate syscall_clone_pidfd_copyout_bound<T, F>(table: T, files: F) -> bool;
predicate syscall_clone_vfork_parent_frame_saved<T, C>(table: T, child: C) -> bool;
predicate syscall_clone_vfork_child_handoff<T, C>(table: T, child: C) -> bool;
predicate syscall_clone_vfork_next_child_accepted<T, C>(table: T, child: C) -> bool;
predicate syscall_clone_vfork_parent_scheduler_blocked<T, C>(table: T, child: C) -> bool;
predicate syscall_clone_vfork_immediate_parent_wake_exactly_once<T, C>(table: T, child: C) -> bool;
predicate syscall_clone_wake_up_new_task_shape<T, S>(table: T, scheduler: S) -> bool;
predicate fork_snapshot_parent_state_matrix<P: Task, F: TaskFlow, R: UserAppRuntime>(parent: P, flow: F, runtime: R) -> bool;
predicate fork_snapshot_child_state_matrix<C: Task, F: TaskFlow, R: UserAppRuntime>(child: C, flow: F, runtime: R) -> bool;
predicate fork_snapshot_identities_all_fresh<P: Task, PF: TaskFlow, PR: UserAppRuntime, C: Task, TR: TaskRef, CF: TaskFlow, FR: TaskFlowRef, CR: UserAppRuntime, A: ApplicationInstance>(parent: P, parent_flow: PF, parent_runtime: PR, child: C, child_ref: TR, child_flow: CF, child_flow_ref: FR, child_runtime: CR, child_application: A) -> bool;
predicate fork_snapshot_child_has_no_parent_overlay_authority<C: Task>(child: C) -> bool;
predicate fork_snapshot_post_fork_continuation_ready<C: Task>(child: C) -> bool;
predicate syscall_execve_linux_6_12_do_execveat_common_bound<T>(table: T) -> bool;
predicate syscall_execve_observed_shell_ls_args_bound<T>(table: T) -> bool;
predicate syscall_execve_observed_busybox_init_getty_args_bound<T>(table: T) -> bool;
predicate syscall_execve_child_continuation_first_slice<T, C>(table: T, child: C) -> bool;
predicate syscall_execve_builtin_grandchild_runtime_exec_bound<T, C>(table: T, child: C) -> bool;
predicate syscall_execve_retired_image_retention_classified<T, C, A, S>(table: T, child: C, space: A, stack: S) -> bool;
predicate syscall_execve_builtin_grandchild_first_parent_image_retained<T, C, A, S>(table: T, child: C, space: A, stack: S) -> bool;
predicate syscall_execve_builtin_grandchild_subsequent_image_released<T, C, A, S>(table: T, child: C, space: A, stack: S) -> bool;
predicate syscall_execve_builtin_grandchild_failure_atomic<T, C, F>(table: T, child: C, files: F) -> bool;
predicate syscall_execve_reuses_user_boot_payload_elf_loader<T, P>(table: T, payload: P) -> bool;
predicate syscall_execve_replaces_user_address_space_first_slice<T, A>(table: T, space: A) -> bool;
predicate syscall_execve_context_staging_address_space_bound<T, A>(table: T, space: A) -> bool;
predicate syscall_execve_sets_start_thread_frame_first_slice<T, R>(table: T, frame: R) -> bool;
predicate syscall_execve_bounded_argv_first_slice<T>(table: T) -> bool;
predicate syscall_execve_vector_copy_uses_current_effective_mm<T, C>(table: T, child: C) -> bool;
predicate syscall_execve_stage_checkpoints_bound<T>(table: T) -> bool;
predicate syscall_execve_return_path_diagnostic_bound<T>(table: T) -> bool;
predicate syscall_execve_envp_full_copy_deferred<T>(table: T) -> bool;
predicate syscall_execve_close_on_exec_deferred<T>(table: T) -> bool;
predicate syscall_execve_old_user_backing_reclaimed_first_slice<T>(table: T) -> bool;
predicate syscall_execve_old_mm_reclaim_deferred<T>(table: T) -> bool;
predicate syscall_execve_full_linux_model_deferred<T>(table: T) -> bool;
predicate syscall_wait4_linux_6_12_kernel_wait4_bound<T>(table: T) -> bool;
predicate syscall_wait4_observed_shell_args_bound<T>(table: T) -> bool;
predicate syscall_wait4_pid_minus_one_all_children_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_positive_pid_exact_child_first_slice<T, P: Task, C: Task>(table: T, parent: P, child: C) -> bool;
predicate syscall_wait4_options_wuntraced_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_options_zero_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_wnohang_no_waitable_child_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_completed_child_record_reap_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_parent_wait_chldexit_boundary<T>(table: T) -> bool;
predicate syscall_wait4_current_parent_task_bound<T, P: Task, C: Task>(table: T, parent: P, child: C) -> bool;
predicate syscall_wait4_owner_scheduler_bound<T, P: Task, S: Scheduler>(table: T, parent: P, scheduler: S) -> bool;
predicate syscall_wait4_yields_to_user_child_continuation<T, P>(table: T, process: P) -> bool;
predicate syscall_wait4_child_exit_status_copyout_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_observed_child_reap_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_no_child_echild_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_blocking_sleep_deferred<T>(table: T) -> bool;
predicate syscall_read_builtin_grandchild_pipe_block_handoff<T, C>(table: T, child: C) -> bool;
predicate syscall_wait4_builtin_grandchild_completed_reap<T, C>(table: T, child: C) -> bool;
predicate syscall_sync_rootfs_barrier_first_slice<T>(table: T) -> bool;
predicate syscall_reboot_poweroff_magic_first_slice<T>(table: T) -> bool;
predicate syscall_exit_records_status<T>(table: T) -> bool;
predicate syscall_exit_group_pid1_shutdown_child_wait4_split<T>(table: T) -> bool;
predicate syscall_exception_dispatches_via_table<T, S>(exception: T, table: S) -> bool;
predicate syscall_exception_extracts_arguments<T>(exception: T) -> bool;
predicate user_trap_return_ready() -> bool;
predicate user_trap_return_switches_satp() -> bool;
predicate user_trap_return_sfence_vma_after_satp() -> bool;
predicate user_trap_return_sret_handoff() -> bool;
predicate user_trap_return_context_used<T, R>(process: T, frame: R) -> bool;
predicate user_trap_entry_uses_kernel_stack<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_linux_riscv_config_bound<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_thread_size_bound<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_vmap_alignment_bound<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_vmapped_bound<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_vmap_guard_page_bound<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_overflow_stack_bound<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_kernel_context_early_overflow_check_bound<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_user_context_bit_test_bypassed<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_early_check_registers_preserved<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_overflow_frame_complete<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_overflow_terminal_panic_bound<T>(frame: T) -> bool;
predicate user_kernel_trap_stack_irq_stack_switch_deferred<T>(frame: T) -> bool;
predicate syscall_table_write_observed<T>(table: T) -> bool;
predicate syscall_table_writev_observed<T>(table: T) -> bool;
predicate syscall_table_openat_observed<T>(table: T) -> bool;
predicate syscall_table_chdir_observed<T>(table: T) -> bool;
predicate syscall_table_read_observed<T>(table: T) -> bool;
predicate syscall_read_trace_probe_observes_result_without_side_effect<T>(table: T) -> bool;
predicate syscall_trace_probe_observes_returns_without_side_effect<T>(table: T) -> bool;
predicate syscall_table_ppoll_observed<T>(table: T) -> bool;
predicate syscall_table_close_observed<T>(table: T) -> bool;
predicate syscall_table_newfstatat_observed<T>(table: T) -> bool;
predicate syscall_table_readlinkat_observed<T>(table: T) -> bool;
predicate syscall_table_dup3_observed<T>(table: T) -> bool;
predicate syscall_table_pipe2_observed<T>(table: T) -> bool;
predicate syscall_table_fchown_observed<T>(table: T) -> bool;
predicate syscall_table_fchmod_observed<T>(table: T) -> bool;
predicate syscall_table_fcntl_observed<T>(table: T) -> bool;
predicate syscall_table_ioctl_observed<T>(table: T) -> bool;
predicate syscall_table_getrandom_observed<T>(table: T) -> bool;
predicate syscall_table_getuid_observed<T>(table: T) -> bool;
predicate syscall_table_getgid_observed<T>(table: T) -> bool;
predicate syscall_table_getpgid_observed<T>(table: T) -> bool;
predicate syscall_table_getsid_observed<T>(table: T) -> bool;
predicate syscall_table_setpgid_observed<T>(table: T) -> bool;
predicate syscall_table_setsid_observed<T>(table: T) -> bool;
predicate syscall_table_setuid_observed<T>(table: T) -> bool;
predicate syscall_table_setgid_observed<T>(table: T) -> bool;
predicate syscall_table_getgroups_observed<T>(table: T) -> bool;
predicate syscall_table_setgroups_observed<T>(table: T) -> bool;
predicate syscall_table_rt_sigprocmask_observed<T>(table: T) -> bool;
predicate syscall_table_rt_sigaction_observed<T>(table: T) -> bool;
predicate syscall_table_rt_sigtimedwait_observed<T>(table: T) -> bool;
predicate syscall_table_clock_gettime_observed<T>(table: T) -> bool;
predicate syscall_table_gettimeofday_observed<T>(table: T) -> bool;
predicate syscall_table_nanosleep_observed<T>(table: T) -> bool;
predicate syscall_table_set_tid_address_observed<T>(table: T) -> bool;
predicate syscall_table_clone_observed<T>(table: T) -> bool;
predicate syscall_table_execve_observed<T>(table: T) -> bool;
predicate syscall_table_wait4_observed<T>(table: T) -> bool;
predicate syscall_table_exit_observed<T>(table: T) -> bool;
predicate user_task_enter_user_mode_observed<T, R>(process: T, frame: R) -> bool;

predicate user_task_online<T>(process: T) -> bool;
predicate user_task_pid1_preserved<T, K>(process: T, task: K) -> bool;
predicate user_task_exec_identity_handoff<T, K>(process: T, task: K) -> bool;
predicate user_task_no_new_task_struct<T>(process: T) -> bool;
predicate user_task_kernel_init_not_destroyed<T, K>(process: T, task: K) -> bool;
predicate user_task_path_bound<T, P>(process: T, path: P) -> bool;
predicate user_task_address_space_bound<T, A>(process: T, space: A) -> bool;
predicate user_task_fs_struct_inherited<T, F>(process: T, fs: F) -> bool;
predicate user_task_files_struct_inherited<T, F>(process: T, files: F) -> bool;
predicate user_task_trap_frame_bound<T, R>(process: T, frame: R) -> bool;
predicate user_task_syscall_context_bound<T, E, S>(process: T, exception: E, table: S) -> bool;
predicate user_task_credentials_inherited<T, K>(process: T, task: K) -> bool;
predicate user_task_root_credentials_bound<T>(process: T) -> bool;
predicate user_task_supplementary_groups_bound<T>(process: T) -> bool;
predicate user_task_supplementary_groups_read_observed<T>(process: T) -> bool;
predicate user_task_credentials_capability_model_deferred<T>(process: T) -> bool;
predicate user_task_signal_state_inherited<T, K>(process: T, task: K) -> bool;
predicate user_task_signal_runtime_bound<T>(process: T) -> bool;
predicate user_task_thread_signal_state_bound<T>(process: T) -> bool;
predicate user_task_process_signal_state_deferred<T>(process: T) -> bool;
predicate user_task_signal_action_table_bound<T>(process: T) -> bool;
predicate user_task_signal_action_table_layout_bound<T>(process: T) -> bool;
predicate user_task_blocked_signal_mask_bound<T>(process: T) -> bool;
predicate user_task_pending_signal_set_empty_first_slice<T>(process: T) -> bool;
predicate user_task_signal_delivery_deferred<T>(process: T) -> bool;
predicate user_task_clear_child_tid_bound<T>(process: T) -> bool;
predicate user_task_session_leader_first_slice<T>(process: T) -> bool;
predicate user_task_process_group_leader_first_slice<T>(process: T) -> bool;
predicate user_task_process_group_read_observed<T>(process: T) -> bool;
predicate user_task_process_group_set_observed<T>(process: T) -> bool;
predicate user_task_child_same_session_pgrp_join_first_slice<T, C>(process: T, child: C) -> bool;
predicate user_task_session_id_read_observed<T>(process: T) -> bool;
predicate user_task_setsid_eperm_observed<T>(process: T) -> bool;
predicate user_task_child_setsid_success_observed<T, C>(process: T, child: C) -> bool;
predicate user_task_child_controlling_tty_clear_on_setsid_first_slice<T, C>(process: T, child: C) -> bool;
predicate user_task_child_process_group_visible<T, C>(process: T, child: C) -> bool;
predicate user_task_child_process_group_set_observed<T, C>(process: T, child: C) -> bool;
predicate user_task_pending_plain_fork_child_parent_visible<T, C>(process: T, child: C) -> bool;
predicate user_task_pending_plain_fork_child_inherits_pgrp_session<T, C>(process: T, child: C) -> bool;
predicate user_task_pending_plain_fork_child_setpgid_first_slice<T, C>(process: T, child: C) -> bool;
predicate user_task_pending_plain_fork_child_setpgid_errno_bound<T, C>(process: T, child: C) -> bool;
predicate user_task_pending_plain_fork_child_consumed_on_wait_handoff<T, C>(process: T, child: C) -> bool;
predicate user_task_pending_plain_fork_child_cleared_on_parent_restore<T, C>(process: T, child: C) -> bool;
predicate user_task_observed_child_visible_pid_only_restore<T, C>(process: T, child: C) -> bool;
predicate user_task_controlling_tty_bound<T>(process: T) -> bool;
predicate user_task_foreground_pgrp_bound<T>(process: T) -> bool;
predicate user_task_foreground_pgrp_read_observed<T>(process: T) -> bool;
predicate user_task_foreground_pgrp_set_observed<T>(process: T) -> bool;
predicate user_task_foreground_pgrp_accepts_child_pgrp_first_slice<T, C>(process: T, child: C) -> bool;
predicate user_task_foreground_pgrp_accepts_shell_inherited_pgrp_first_slice<T, C>(process: T, child: C) -> bool;
predicate user_task_tty_session_id_read_observed<T>(process: T) -> bool;
predicate user_task_tiocsctty_observed<T>(process: T) -> bool;
predicate user_task_child_controlling_tty_bound<T, C>(process: T, child: C) -> bool;
predicate user_child_process_prepared<T>(process: T) -> bool;
predicate user_child_process_parent_pid1<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_parent_pid1_or_current_child<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_pid_allocated<T, N>(process: T, pid_ns: N) -> bool;
predicate user_child_process_tgid_equals_pid<T>(process: T) -> bool;
predicate user_child_process_process_group_visible_to_parent<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_exit_signal_sigchld<T>(process: T) -> bool;
predicate user_child_process_files_struct_copied<T, F>(process: T, files: F) -> bool;
predicate user_child_process_fd_table_independent<T, F>(process: T, files: F) -> bool;
predicate user_child_process_open_backings_aliased<T, F>(process: T, files: F) -> bool;
predicate user_child_process_fs_struct_copied<T, F>(process: T, fs: F) -> bool;
predicate user_child_process_parent_fd_snapshot_saved<T, F>(process: T, files: F) -> bool;
predicate user_child_process_parent_fd_snapshot_restored<T, F>(process: T, files: F) -> bool;
predicate user_child_process_credentials_copied<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_signal_state_copied<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_independent_mm_owned<T, A>(process: T, space: A) -> bool;
predicate user_child_process_root_and_satp_distinct<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_cow_private_pages_shared<T, A>(process: T, space: A) -> bool;
predicate user_child_process_dup_mm_failure_atomic<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_mm_published_after_copy<T>(process: T) -> bool;
predicate user_child_process_task_mm_switched<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_exit_mm_released_once<T>(process: T) -> bool;
predicate user_child_process_parent_mm_resumed_without_byte_restore<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_trap_frame_copied<T, R>(process: T, frame: R) -> bool;
predicate user_child_process_trap_frame_child_return_zero<T>(process: T) -> bool;
predicate user_child_process_trap_frame_child_sp_set<T>(process: T) -> bool;
predicate user_child_process_tls_inherited<T>(process: T) -> bool;
predicate user_child_process_enqueued<T, R>(process: T, runqueue: R) -> bool;
predicate user_child_process_wait4_parent_wait_observed<T>(process: T) -> bool;
predicate user_child_process_child_continuation_taken<T>(process: T) -> bool;
predicate user_child_process_vfork_parent_frame_saved<T>(process: T) -> bool;
predicate user_child_process_vfork_child_handoff<T>(process: T) -> bool;
predicate user_child_process_vfork_parent_resumed<T>(process: T) -> bool;
predicate user_child_process_nested_vfork_child_handoff<T>(process: T) -> bool;
predicate user_child_process_observed_child_plain_fork_bound<T>(process: T) -> bool;
predicate user_child_process_observed_child_plain_fork_parent_saved<T>(process: T) -> bool;
predicate user_child_process_observed_child_plain_fork_child_pid_bound<T>(process: T) -> bool;
predicate user_child_process_observed_child_plain_fork_no_second_task<T>(process: T) -> bool;
predicate user_child_process_observed_child_plain_fork_parent_restored<T>(process: T) -> bool;
predicate user_child_process_builtin_grandchild_parent_continuation_snapshot_bound<T, A, F>(process: T, space: A, files: F) -> bool;
predicate user_child_process_outer_pid1_snapshot_ownership_preserved<T, A, F>(process: T, space: A, files: F) -> bool;
predicate user_child_process_builtin_grandchild_pipe_data_shared<T, F>(process: T, files: F) -> bool;
predicate user_child_process_builtin_grandchild_snapshot_failure_atomic<T, F>(process: T, files: F) -> bool;
predicate user_child_process_observed_child_transient_state_cleared<T>(process: T) -> bool;
predicate user_child_process_observed_shell_runqueue_preserved<T, R>(process: T, runqueue: R) -> bool;
predicate user_child_process_pidfd_copyout_observed<T>(process: T) -> bool;
predicate user_child_process_next_child_pid_bound<T>(process: T) -> bool;
predicate user_child_process_completed_record_archived<T>(process: T) -> bool;
predicate user_child_process_completed_record_unreaped<T>(process: T) -> bool;
predicate user_child_process_completed_record_reaped<T>(process: T) -> bool;
predicate user_child_process_completed_record_released<T>(process: T) -> bool;
predicate user_child_process_plain_fork_reaped_slot_released<T, R>(process: T, runqueue: R) -> bool;
predicate user_child_process_vfork_next_child_accepted<T>(process: T) -> bool;
predicate user_child_process_wait4_handoff_frame_diagnostic_bound<T>(process: T) -> bool;
predicate user_child_process_parent_wait_frame_saved<T>(process: T) -> bool;
predicate user_child_process_parent_exec_objects_retained<T, A, S>(process: T, space: A, stack: S) -> bool;
predicate user_child_process_replaced_child_exec_backing_released<T>(process: T) -> bool;
predicate user_child_process_parent_user_stack_restored<T, S>(process: T, stack: S) -> bool;
predicate user_child_process_parent_wait_register_checkpoint_bound<T>(process: T) -> bool;
predicate user_child_process_parent_wait_stack_window_checkpoint_bound<T>(process: T) -> bool;
predicate user_child_process_parent_wait_stack_window_compared<T>(process: T) -> bool;
predicate user_child_process_exit_status_observed<T>(process: T) -> bool;
predicate user_child_process_wait4_status_copied<T>(process: T) -> bool;
predicate user_child_process_parent_wait_resumed<T>(process: T) -> bool;
predicate user_child_process_parent_wait_resume_checkpoint_bound<T>(process: T) -> bool;
predicate user_address_space_fault_mapping_diagnostic_bound<T>(space: T) -> bool;
predicate user_task_uid_read_observed<T>(process: T) -> bool;
predicate user_task_gid_read_observed<T>(process: T) -> bool;
predicate user_task_uid_set_observed<T>(process: T) -> bool;
predicate user_task_gid_set_observed<T>(process: T) -> bool;
predicate user_task_setgroups_observed<T>(process: T) -> bool;
predicate user_task_rt_sigprocmask_observed<T>(process: T) -> bool;
predicate user_task_rt_sigaction_observed<T>(process: T) -> bool;
predicate user_task_rt_sigtimedwait_observed<T>(process: T) -> bool;
predicate user_task_rt_sigtimedwait_mask_observed<T>(process: T) -> bool;
predicate user_task_rt_sigtimedwait_uinfo_null<T>(process: T) -> bool;
predicate user_task_rt_sigtimedwait_uts_null<T>(process: T) -> bool;
predicate user_task_rt_sigtimedwait_pending_match_empty<T>(process: T) -> bool;
predicate user_task_rt_sigtimedwait_infinite_wait<T>(process: T) -> bool;
predicate user_task_pending_sigchld_first_slice<T>(process: T) -> bool;
predicate user_task_rt_sigtimedwait_waiter_enqueued<T>(process: T) -> bool;
predicate user_task_rt_sigtimedwait_sleep_reason_bound<T>(process: T) -> bool;
predicate user_task_rt_sigtimedwait_woken_by_sigchld<T>(process: T) -> bool;
predicate user_task_rt_sigtimedwait_dequeued_sigchld<T>(process: T) -> bool;
predicate user_task_user_entry_ready<T>(process: T) -> bool;
predicate user_task_trap_return_bound<T, R>(process: T, frame: R) -> bool;
predicate kernel_init_task_execve_to_pid1_user_app<K, T>(task: K, process: T) -> bool;
predicate kernel_init_task_pid1_identity_preserved<K>(task: K) -> bool;
predicate kernel_init_task_user_app_flow_online<K>(task: K) -> bool;
predicate kernel_init_task_pid1_exec_flow_handoff_complete<K>(task: K) -> bool;
predicate kernel_init_task_user_mm_attached<K, A>(task: K, space: A) -> bool;
predicate kernel_init_task_user_trap_frame_attached<K, R>(task: K, frame: R) -> bool;
predicate syscall_clone_returns_task_ref<T, R: TaskRef, C: Task>(
    table: T,
    task_ref: R,
    child: C
) -> bool;
predicate syscall_setsid_user_process_registry_child_success_first_slice<T, S: TaskSet>(
    table: T,
    registry: S
) -> bool;
predicate syscall_setpgid_user_process_registry_current_or_child_first_slice<T, S: TaskSet>(
    table: T,
    registry: S
) -> bool;
predicate syscall_ioctl_user_process_registry_current_job_control_first_slice<T, S: TaskSet>(
    table: T,
    registry: S
) -> bool;

predicate files_struct_stdin_ready_data_bound<T>(files: T) -> bool;
predicate files_struct_stdin_char_device_read_observed<T>(files: T) -> bool;
predicate files_struct_tty_termios_state_bound<T>(files: T) -> bool;
predicate files_struct_tty_termios_mutation_observed<T>(files: T) -> bool;
predicate files_struct_open_path_routes_to_vfs<T, V>(files: T, vfs: V) -> bool;
predicate files_struct_regular_fd_installed<T>(files: T) -> bool;
predicate files_struct_directory_fd_installed<T>(files: T) -> bool;
predicate files_struct_null_fd_installed<T>(files: T) -> bool;
predicate files_struct_null_device_read_eof_observed<T>(files: T) -> bool;
predicate files_struct_null_device_write_discard_observed<T>(files: T) -> bool;
predicate files_struct_null_device_fstat_device_node<T>(files: T) -> bool;
predicate files_struct_null_device_tty_ioctl_enotty<T>(files: T) -> bool;
predicate file_backend_char_device_read_returns_ready_data<T>(backend: T) -> bool;
predicate tty_flip_buffer_ready_data_bound<T>(buffer: T) -> bool;
predicate tty_flip_buffer_ready_data_consumed<T>(buffer: T) -> bool;
predicate tty_n_tty_blocking_read_deferred<T>(buffer: T) -> bool;

context UserModeTrapReturnContext: Context {
    /*
     * Runtime application commit is the final architecture trap-return handoff. The
     * kernel binds the user-trap entry context and prepared return context,
     * switches to the user execution context and does not return. The user
     * application beyond this boundary is intentionally opaque. Register
     * lowering belongs to the target coding specification.
     */
    guard {
        entered_by {
            KernelInitFlow.Action::CommitPayloadHandoff;
        }

        exited_by {
            Never;
        }
    }

    obj_refs {
        KernelInitTask;
        UserAddressSpace;
        UserTrapFrame;
    }
}


object UserCloneDeferredBoundaries: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PayloadParam.state == State::Ready;
                    KernelInitTask.state == State::OnCpu;
                    task_execution_authority_is(
                        KernelInitTask,
                        TaskExecutionAuthority::Live
                    );
                    SystemState.state == State::Online;
                    system_state_running(SystemState);
                }

                drives {
                    UserProcessRegistry.Transition::Setup;
                }

                ensures {
                    UserProcessRegistry.state == State::Ready;
                    user_process_registry_capacity_is_32(UserProcessRegistry);
                    user_process_registry_pid1_stable(UserProcessRegistry);
                    user_process_registry_allows_independent_aggregates(UserProcessRegistry);
                    user_process_registry_members_fresh_and_independent(UserProcessRegistry);
                    user_clone_deferred_boundaries_ready(self);
                    user_clone_linux_6_12_legacy_clone_bound(self);
                    user_clone_riscv_abi_argument_order_bound(self);
                    user_clone_observed_plain_fork_args_bound(self);
                    user_clone_observed_vfork_vm_args_bound(self);
                    user_clone_observed_vfork_pidfd_args_bound(self);
                    user_clone_plain_fork_first_slice_bound(self);
                    user_clone_vfork_vm_first_slice_bound(self);
                    user_clone_vfork_pidfd_first_slice_bound(self);
                    user_clone_fresh_task_per_child_bound(self);
                    user_clone_multiple_independent_tasks_bound(self);
                    user_clone_csignal_split_bound(self);
                    user_clone_sigchld_exit_signal_bound(self);
                    user_clone_newsp_zero_inherits_parent_sp(self);
                    user_clone_newsp_sets_child_sp(self);
                    user_clone_legacy_pidfd_uses_parent_tidptr(self);
                    user_clone_tls_ignored_without_clone_settls(self);
                    user_clone_full_clone3_deferred(self);
                    user_clone_full_thread_group_deferred(self);
                    user_clone_vfork_clone_vm_parent_cpu_serial(self);
                    user_clone_vfork_task_local_aggregate(self);
                    user_clone_vfork_task_local_parent_completion(self);
                    user_clone_vfork_effective_mm_generation_checked(self);
                    user_clone_vfork_nested_handoffs_independent(self);
                    user_clone_vfork_exec_detaches_without_retiring_parent_mm(self);
                    user_clone_vfork_exit_wakes_without_releasing_parent_mm(self);
                    user_clone_plain_fork_resolves_current_mm_binding(self);
                    user_clone_independent_mm_cow_bound(self);
                    user_clone_cow_private_frame_sharing_bound(self);
                    user_clone_cow_parent_child_ro_lowered(self);
                    user_clone_cow_readonly_shared_without_promotion(self);
                    user_clone_cow_prepare_before_commit(self);
                    user_clone_cow_publish_after_commit(self);
                    user_clone_cow_failure_atomic(self);
                    user_clone_cow_teardown_refcount_conserving(self);
                    user_clone_full_pidfd_file_ops_deferred(self);
                    user_clone_full_ptrace_hooks_deferred(self);
                    user_clone_full_seccomp_hooks_deferred(self);
                    user_clone_full_cgroup_hooks_deferred(self);
                    user_clone_full_audit_hooks_deferred(self);
                    user_clone_full_namespace_deferred(self);
                    user_clone_full_robust_futex_deferred(self);
                    user_clone_full_clear_child_tid_wake_deferred(self);
                    user_clone_wait_sleep_wakeup_cross_cpu(self);
                    user_clone_zombie_reap_accounting_cross_cpu(self);
                    user_clone_online_cpu_sequence_frozen(self);
                    user_clone_pid1_cpu0_bound(self);
                    user_clone_plain_fork_pid_mod_cpu_placement(self);
                    user_clone_cpu_ownership_immutable(self);
                    user_clone_max_concurrent_user_tasks_32(self);
                    user_clone_max_online_cpus_16(self);
                    user_clone_fork_failure_eagain_or_enomem(self);
                    user_clone_fork_failure_no_partial_publication(self);
                    user_clone_shared_mm_cross_cpu_deferred(self);
                    user_clone_full_job_control_deferred(self);
                    user_clone_unsupported_flags_first_slice(self);
                }

                deferred user_clone.001 {
                    category: DeferredCategory::Feature;
                    summary: "Implement the complete clone3 syscall ABI and validation path.";
                    evidence { user_clone_full_clone3_deferred(self); }
                    close_when: "clone3 argument validation, task creation, rollback and Linux differential tests pass.";
                }

                deferred user_clone.002 {
                    category: DeferredCategory::Feature;
                    summary: "Implement complete thread-group clone semantics.";
                    evidence { user_clone_full_thread_group_deferred(self); }
                    close_when: "CLONE_THREAD/TGID/signal-sharing lifecycle and multi-thread tests pass.";
                }

                deferred user_clone.003 {
                    category: DeferredCategory::Protocol;
                    summary: "Implement shared-mm execution across CPUs beyond the serial same-CPU vfork boundary.";
                    evidence { user_clone_shared_mm_cross_cpu_deferred(self); }
                    close_when: "Shared-mm scheduling has ASIDs or remote TLB invalidation and concurrent Linux differential tests.";
                }

                deferred user_clone.005 {
                    category: DeferredCategory::Feature;
                    summary: "Implement complete pidfs/pidfd file operations and PIDFD wait/signal APIs.";
                    evidence { user_clone_full_pidfd_file_ops_deferred(self); }
                    close_when: "pidfd file lifecycle, send-signal/getfd and waitid(P_PIDFD) tests match Linux.";
                }

                deferred user_clone.006 {
                    category: DeferredCategory::Protocol;
                    summary: "Implement clone and exit ptrace hooks.";
                    evidence { user_clone_full_ptrace_hooks_deferred(self); }
                    close_when: "Ptrace clone/exec/exit event ordering and differential tests pass.";
                }

                deferred user_clone.007 {
                    category: DeferredCategory::Protocol;
                    summary: "Implement clone seccomp inheritance and filter hooks.";
                    evidence { user_clone_full_seccomp_hooks_deferred(self); }
                    close_when: "Seccomp filter inheritance and clone denial tests match Linux.";
                }

                deferred user_clone.008 {
                    category: DeferredCategory::Protocol;
                    summary: "Implement clone cgroup admission and accounting hooks.";
                    evidence { user_clone_full_cgroup_hooks_deferred(self); }
                    close_when: "Cgroup fork admission, rollback and accounting tests pass.";
                }

                deferred user_clone.009 {
                    category: DeferredCategory::Protocol;
                    summary: "Implement clone and exit audit hooks.";
                    evidence { user_clone_full_audit_hooks_deferred(self); }
                    close_when: "Audit records and ordering for clone/exit match Linux fixtures.";
                }

                deferred user_clone.010 {
                    category: DeferredCategory::Feature;
                    summary: "Implement complete clone namespace creation and sharing semantics.";
                    evidence { user_clone_full_namespace_deferred(self); }
                    close_when: "Supported namespace flags, ownership, rollback and isolation tests pass.";
                }

                deferred user_clone.011 {
                    category: DeferredCategory::Protocol;
                    summary: "Implement robust-futex inheritance and exit cleanup.";
                    evidence { user_clone_full_robust_futex_deferred(self); }
                    close_when: "Robust-list inheritance, owner-death cleanup and wake tests pass.";
                }

                deferred user_clone.012 {
                    category: DeferredCategory::Protocol;
                    summary: "Implement clear_child_tid store and futex wake on task exit.";
                    evidence { user_clone_full_clear_child_tid_wake_deferred(self); }
                    close_when: "clear_child_tid copyout and futex wake behavior matches Linux success and fault tests.";
                }

                deferred user_clone.015 {
                    category: DeferredCategory::Feature;
                    summary: "Implement complete session, controlling-TTY, process-group, PTY and job-control signal semantics.";
                    evidence { user_clone_full_job_control_deferred(self); }
                    close_when: "setsid, controlling-TTY detach, orphan pgrp, PTY and job-control signal tests match Linux.";
                }

                deferred user_clone.016 {
                    category: DeferredCategory::AlternatePath;
                    summary: "Handle clone flags and wait options outside the observed first-slice combinations.";
                    evidence { user_clone_unsupported_flags_first_slice(self); }
                    close_when: "Every supported flag/option has formal semantics and tests, and every rejected combination has Linux-compatible errno.";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            user_clone_deferred_boundaries_ready(self);
            user_clone_linux_6_12_legacy_clone_bound(self);
            user_clone_riscv_abi_argument_order_bound(self);
            user_clone_observed_plain_fork_args_bound(self);
            user_clone_observed_vfork_vm_args_bound(self);
            user_clone_observed_vfork_pidfd_args_bound(self);
            user_clone_plain_fork_first_slice_bound(self);
            user_clone_vfork_vm_first_slice_bound(self);
            user_clone_vfork_pidfd_first_slice_bound(self);
            user_clone_fresh_task_per_child_bound(self);
            user_clone_multiple_independent_tasks_bound(self);
            user_clone_csignal_split_bound(self);
            user_clone_sigchld_exit_signal_bound(self);
            user_clone_newsp_zero_inherits_parent_sp(self);
            user_clone_newsp_sets_child_sp(self);
            user_clone_legacy_pidfd_uses_parent_tidptr(self);
            user_clone_tls_ignored_without_clone_settls(self);
            user_clone_full_clone3_deferred(self);
            user_clone_full_thread_group_deferred(self);
            user_clone_vfork_clone_vm_parent_cpu_serial(self);
            user_clone_vfork_task_local_aggregate(self);
            user_clone_vfork_task_local_parent_completion(self);
            user_clone_vfork_effective_mm_generation_checked(self);
            user_clone_vfork_nested_handoffs_independent(self);
            user_clone_vfork_exec_detaches_without_retiring_parent_mm(self);
            user_clone_vfork_exit_wakes_without_releasing_parent_mm(self);
            user_clone_plain_fork_resolves_current_mm_binding(self);
            user_clone_independent_mm_cow_bound(self);
            user_clone_cow_private_frame_sharing_bound(self);
            user_clone_cow_parent_child_ro_lowered(self);
            user_clone_cow_readonly_shared_without_promotion(self);
            user_clone_cow_prepare_before_commit(self);
            user_clone_cow_publish_after_commit(self);
            user_clone_cow_failure_atomic(self);
            user_clone_cow_teardown_refcount_conserving(self);
            user_clone_full_pidfd_file_ops_deferred(self);
            user_clone_full_ptrace_hooks_deferred(self);
            user_clone_full_seccomp_hooks_deferred(self);
            user_clone_full_cgroup_hooks_deferred(self);
            user_clone_full_audit_hooks_deferred(self);
            user_clone_full_namespace_deferred(self);
            user_clone_full_robust_futex_deferred(self);
            user_clone_full_clear_child_tid_wake_deferred(self);
            user_clone_wait_sleep_wakeup_cross_cpu(self);
            user_clone_zombie_reap_accounting_cross_cpu(self);
            user_clone_online_cpu_sequence_frozen(self);
            user_clone_pid1_cpu0_bound(self);
            user_clone_plain_fork_pid_mod_cpu_placement(self);
            user_clone_cpu_ownership_immutable(self);
            user_clone_max_concurrent_user_tasks_32(self);
            user_clone_max_online_cpus_16(self);
            user_clone_fork_failure_eagain_or_enomem(self);
            user_clone_fork_failure_no_partial_publication(self);
            user_clone_shared_mm_cross_cpu_deferred(self);
            user_clone_full_job_control_deferred(self);
            user_clone_unsupported_flags_first_slice(self);
        }
    }
}

object UserAddressSpace: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    SwapperVm.state == State::Ready;
                    KernelAddrSpace.state == State::Online;
                    Vm.state == State::Online;
                    PageAllocator.state == State::Ready;
                    KernelGlobalAllocator.state == State::Ready;
                    KernelInitTask.state == State::OnCpu;
                    task_execution_authority_is(
                        KernelInitTask,
                        TaskExecutionAuthority::Live
                    );
                }

                ensures {
                    user_address_space_allocated(self);
                    user_address_space_first_instance(self);
                    user_address_space_bound_to_kernel_init_task(self, KernelInitTask);
                    kernel_init_task_first_user_address_space_bound(KernelInitTask, self);
                    user_address_space_low_half_private(self);
                    user_address_space_high_half_shares_swapper(self, SwapperVm);
                    user_address_space_kernel_pages_u_disabled(self);
                    swapper_vm_remains_kernel_shared_instance(SwapperVm);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            user_address_space_allocated(self);
            user_address_space_first_instance(self);
            user_address_space_bound_to_kernel_init_task(self, KernelInitTask);
            kernel_init_task_first_user_address_space_bound(KernelInitTask, self);
            user_address_space_low_half_private(self);
            user_address_space_high_half_shares_swapper(self, SwapperVm);
            user_address_space_kernel_pages_u_disabled(self);
            swapper_vm_remains_kernel_shared_instance(SwapperVm);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    ElfObject.state == State::Ready;
                    UserStack.state == State::Ready;
                }

                ensures {
                    user_address_space_user_pages_u_enabled(self);
                    user_address_space_elf_load_plan_consumed(self, ElfObject);
                    user_address_space_segment_mappings_bound(self);
                    user_address_space_entry_mapping_executable(self);
                    user_address_space_bss_zero_plan_consumed(self, ElfObject);
                    user_address_space_backing_pages_allocated(self);
                    user_address_space_elf_file_bytes_copied(self, ElfObject);
                    user_address_space_bss_bytes_zeroed(self, ElfObject);
                    user_address_space_page_table_view_ready(self);
                    user_address_space_elf_segments_mapped(self, ElfObject);
                    user_address_space_stack_mapped(self, UserStack);
                    user_address_space_heap_arena_mapped(self);
                    elf_object_mapped_to_user_address_space(ElfObject, self);
                    elf_object_bss_zeroed(ElfObject);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            user_address_space_allocated(self);
            user_address_space_first_instance(self);
            user_address_space_bound_to_kernel_init_task(self, KernelInitTask);
            kernel_init_task_first_user_address_space_bound(KernelInitTask, self);
            user_address_space_low_half_private(self);
            user_address_space_high_half_shares_swapper(self, SwapperVm);
            user_address_space_kernel_pages_u_disabled(self);
            user_address_space_user_pages_u_enabled(self);
            user_address_space_elf_load_plan_consumed(self, ElfObject);
            user_address_space_segment_mappings_bound(self);
            user_address_space_entry_mapping_executable(self);
            user_address_space_bss_zero_plan_consumed(self, ElfObject);
            user_address_space_backing_pages_allocated(self);
            user_address_space_elf_file_bytes_copied(self, ElfObject);
            user_address_space_bss_bytes_zeroed(self, ElfObject);
            user_address_space_page_table_view_ready(self);
            user_address_space_elf_segments_mapped(self, ElfObject);
            user_address_space_stack_mapped(self, UserStack);
            user_address_space_heap_arena_mapped(self);
            elf_object_mapped_to_user_address_space(ElfObject, self);
            elf_object_bss_zeroed(ElfObject);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    UserTrapFrame.state == State::Ready;
                    SwapperVm.state == State::Ready;
                    KernelAddrSpace.state == State::Online;
                    Vm.state == State::Online;
                    PageAllocator.state == State::Ready;
                    PageMetadataMap.state == State::Ready;
                }

                ensures {
                    user_address_space_real_page_table_allocated(self);
                    user_address_space_user_leaf_ptes_installed(self);
                    user_address_space_high_half_root_entries_shared(self, SwapperVm);
                    user_address_space_satp_token_ready(self);
                    user_address_space_prepared_but_not_current(self);
                    user_address_space_runtime_ready(self);
                    user_address_space_vmas_nonoverlapping(self);
                    user_address_space_heap_demand_paged(self);
                    user_address_space_anonymous_private_demand_paged(self);
                    user_address_space_user_frame_refs_conserved(self, PageMetadataMap);
                }
            }
        }
    }

    state State::Online {
        invariant {
            user_address_space_real_page_table_allocated(self);
            user_address_space_user_leaf_ptes_installed(self);
            user_address_space_high_half_root_entries_shared(self, SwapperVm);
            user_address_space_satp_token_ready(self);
            user_address_space_prepared_but_not_current(self);
            user_address_space_runtime_ready(self);
            user_address_space_first_instance(self);
            user_address_space_bound_to_kernel_init_task(self, KernelInitTask);
            kernel_init_task_first_user_address_space_bound(KernelInitTask, self);
            user_address_space_heap_arena_mapped(self);
            user_address_space_vmas_nonoverlapping(self);
            user_address_space_heap_demand_paged(self);
            user_address_space_anonymous_private_demand_paged(self);
            user_address_space_user_frame_refs_conserved(self, PageMetadataMap);
        }

        actions {
            on Action::Brk {
                ensures {
                    user_address_space_heap_arena_mapped(self);
                    user_address_space_brk_window_bound(self);
                }
            }

            on Action::Mmap {
                ensures {
                    user_address_space_heap_arena_mapped(self);
                    user_address_space_mmap_anonymous_window_bound(self);
                    user_address_space_mmap_fixed_heap_base_compat_bound(self);
                }
            }

            on Action::Mprotect {
                ensures {
                    user_address_space_mprotect_accepts_mapped_user_range(self);
                }
            }

            on Action::Munmap {
                ensures {
                    user_address_space_munmap_accepts_mapped_user_range(self);
                    user_address_space_munmap_anonymous_private_whole_vma(self);
                    user_address_space_munmap_removes_leaves_backing_and_vma(self);
                    user_address_space_munmap_range_and_slot_reusable(self);
                    user_address_space_munmap_rejects_partial_or_nonanonymous_atomically(self);
                }
            }

            on Action::ResolveUserFault {
                ensures {
                    user_address_space_fault_request_task_mm_bound(self);
                    user_address_space_terminal_handoff_fault_request_uses_committed_parent_task(self);
                    user_address_space_fault_access_classified(self);
                    user_address_space_fault_classes_total_and_exclusive(self);
                    user_address_space_fault_not_present_bound(self);
                    user_address_space_fault_cow_write_protect_bound(self);
                    user_address_space_fault_protection_bound(self);
                    user_address_space_fault_unmapped_bound(self);
                    user_address_space_fault_retry_preserves_sepc(self);
                    user_address_space_fault_targeted_tlb_flush(self);
                    user_address_space_fault_failure_atomic(self);
                    user_address_space_fault_diagnostic_stable(self);
                    user_address_space_fault_cow_requires_original_private_writable_vma(self);
                    user_address_space_fault_cow_shared_copy_bound(self, PageMetadataMap);
                    user_address_space_fault_cow_unique_restore_bound(self, PageMetadataMap);
                    user_address_space_fault_cow_refcount_conserved(self, PageMetadataMap);
                    user_address_space_fault_cow_commit_failure_preserves_old_leaf(self, PageMetadataMap);
                    user_address_space_fault_cow_oom_terminal_bound(self);
                    user_address_space_fault_cow_diagnostics_nonzero(self);
                    user_address_space_owner_task_bound(self, KernelInitTask);
                    user_address_space_aggregate_identity_stable(self);
                    user_address_space_not_swapped_through_global_carrier(self);
                    user_address_space_lease_checked_for_current_task_cpu(self);
                    user_address_space_satp_committed_on_dispatch(self);
                    user_address_space_local_sfence_after_satp(self);
                    user_address_space_exec_replacement_atomic(self);
                    user_address_space_fault_kernel_extable_isolated(self);
                    user_address_space_fault_unmapped_segv_maperr_bound(self);
                    user_address_space_fault_protection_segv_accerr_bound(self);
                    user_address_space_fault_sigsegv_addr_exact(self);
                    user_address_space_fault_sigsegv_no_sepc_advance(self);
                    user_address_space_fault_sigsegv_task_terminal_bound(self);
                    user_address_space_fault_sigsegv_usercopy_still_efault(self);
                }
            }
        }
    }
}

object UserTrapFrame: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    UserAddressSpace.state == State::Ready;
                    ElfObject.state == State::Ready;
                    UserStack.state == State::Ready;
                    KernelInitTask.state == State::OnCpu;
                    task_execution_authority_is(
                        KernelInitTask,
                        TaskExecutionAuthority::Live
                    );
                }

                ensures {
                    user_trap_frame_allocated(self);
                    user_trap_frame_entry_bound(self, ElfObject);
                    user_trap_frame_sp_bound(self, UserStack);
                    user_trap_frame_sstatus_user_mode(self);
                    user_trap_frame_fpu_initial(self);
                    user_trap_frame_fpu_context_switch_deferred(self);
                    user_trap_frame_sret_ready(self);
                    user_trap_frame_address_space_bound(self, UserAddressSpace);
                    user_trap_frame_prepared_but_not_entered(self);
                    user_trap_return_ready();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            user_trap_frame_allocated(self);
            user_trap_frame_entry_bound(self, ElfObject);
            user_trap_frame_sp_bound(self, UserStack);
            user_trap_frame_sstatus_user_mode(self);
            user_trap_frame_fpu_initial(self);
            user_trap_frame_fpu_context_switch_deferred(self);
            user_trap_frame_sret_ready(self);
            user_trap_frame_address_space_bound(self, UserAddressSpace);
            user_trap_frame_prepared_but_not_entered(self);
            user_trap_return_ready();
        }
    }
}

object SyscallTable: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    ConsoleRegistry.state == State::Ready;
                    CurrentCPU.trap.exception.syscall.state == State::Ready;
                }

                ensures {
                    syscall_table_ready(self);
                    syscall_table_bound_to_exception(self, CurrentCPU.trap.exception.syscall);
                    syscall_table_write_supported(self);
                    syscall_table_writev_supported(self);
                    syscall_table_openat_supported(self);
                    syscall_table_chdir_supported(self);
                    syscall_table_read_supported(self);
                    syscall_table_ppoll_supported(self);
                    syscall_table_close_supported(self);
                    syscall_table_newfstatat_supported(self);
                    syscall_table_readlinkat_supported(self);
                    syscall_table_dup3_supported(self);
                    syscall_table_pipe2_supported(self);
                    syscall_table_fchown_supported(self);
                    syscall_table_fchmod_supported(self);
                    syscall_table_fcntl_supported(self);
                    syscall_table_ioctl_supported(self);
                    syscall_table_getrandom_supported(self);
                    syscall_table_getuid_supported(self);
                    syscall_table_getgid_supported(self);
                    syscall_table_getpgid_supported(self);
                    syscall_table_getsid_supported(self);
                    syscall_table_setpgid_supported(self);
                    syscall_table_setsid_supported(self);
                    syscall_table_setuid_supported(self);
                    syscall_table_setgid_supported(self);
                    syscall_table_getgroups_supported(self);
                    syscall_table_setgroups_supported(self);
                    syscall_table_rt_sigprocmask_supported(self);
                    syscall_table_rt_sigaction_supported(self);
                    syscall_table_rt_sigtimedwait_supported(self);
                    syscall_table_clock_gettime_supported(self);
                    syscall_table_gettimeofday_supported(self);
                    syscall_table_nanosleep_supported(self);
                    syscall_table_brk_supported(self);
                    syscall_table_mmap_supported(self);
                    syscall_table_mprotect_supported(self);
                    syscall_table_munmap_supported(self);
                    syscall_table_set_tid_address_supported(self);
                    syscall_table_clone_supported(self);
                    syscall_table_execve_supported(self);
                    syscall_table_wait4_supported(self);
                    syscall_table_exit_supported(self);
                    syscall_table_exit_group_supported(self);
                    syscall_write_usercopy_ready(self);
                    syscall_writev_usercopy_ready(self);
                    syscall_read_usercopy_ready(self);
                    syscall_fixed_usercopy_checks_mapping_permissions(self, UserAddressSpace);
                    syscall_ppoll_pollfd_usercopy_ready(self);
                    syscall_getrandom_usercopy_ready(self);
                    syscall_signal_mask_usercopy_ready(self);
                    syscall_signal_action_usercopy_ready(self);
                    syscall_time_usercopy_ready(self);
                    syscall_nanosleep_usercopy_ready(self);
                    syscall_ioctl_usercopy_ready(self);
                    syscall_mmap_fixed_anonymous_prot_none_first_slice(self);
                    syscall_mmap_full_vma_model_deferred(self);
                    syscall_path_usercopy_ready(self);
                    syscall_stat_usercopy_ready(self);
                    syscall_write_routes_to_console(self);
                    syscall_read_no_ready_blocking_out_of_slice(self);
                    syscall_read_tty_read_wait_entry_first_slice(self);
                    syscall_read_tty_read_wait_finish_first_slice(self);
                    syscall_ppoll_timeout_parse_first_slice(self);
                    syscall_ppoll_sigmask_deferred(self);
                    syscall_ppoll_no_ready_blocking_out_of_slice(self);
                    syscall_ppoll_poll_table_first_slice(self);
                    syscall_ppoll_poll_freewait_first_slice(self);
                    syscall_ppoll_blocking_wait_deferred(self);
                    syscall_ioctl_tty_full_linux_model_deferred(self);
                    syscall_fchown_fchmod_fd_local_first_slice(self);
                    syscall_fchown_fchmod_full_linux_model_deferred(self);
                    syscall_pipe2_flags_zero_first_slice(self);
                    syscall_pipe2_atomic_fd_and_usercopy_rollback(self);
                    syscall_pipe2_full_linux_model_deferred(self);
                    /*
                     * Current BusyBox init login evidence has moved past post-auth
                     * fchown(55)/fchmod(52) and classifies
                     * socket(AF_UNIX, SOCK_STREAM|SOCK_CLOEXEC, 0) as the
                     * direct login failure boundary. Linux 6.12 differential
                     * comparison shows the minimal successful path is
                     * __sys_socket_create() flag stripping/validation,
                     * AF_UNIX unix_create(), then sock_map_fd() installing an
                     * fd carrying close-on-exec. The first implementation
                     * slice therefore supports only that AF_UNIX stream
                     * creation path and routes it to FilesStruct. Focused
                     * rerun after the slice observes the post-auth socket
                     * returning fd 3, with connect(203) as the next first
                     * boundary. The current connect slice follows Linux 6.12
                     * net/socket.c::move_addr_to_kernel() only far enough to
                     * copy at most sockaddr_storage-sized user bytes, then
                     * accepts only AF_UNIX pathname sockaddr_un values that
                     * satisfy net/unix/af_unix.c::unix_validate_addr(), are not
                     * abstract paths, and whose fd currently names
                     * UnixSocket0. That path routes existence lookup through
                     * current FsStruct.root and VFS without opening,
                     * allocating FileRef or mutating fd state. The lookup may
                     * follow existing fast symlinks and . / .. path components
                     * clamped at FsStruct.root, covering /var/run -> ../run
                     * before the final socket pathname miss. Missing
                     * pathname returns ENOENT; existing pathname returns
                     * ECONNREFUSED because peer/listener state is still
                     * unmodeled. Focused rerun after the missing
                     * /var/run/nscd/socket case reaches setgroups(159)
                     * with gidsetsize=1 and a user gid_t list pointer; the
                     * SetGroups action closes that shape. The current
                     * SetGid slice then accepts setgid(144) gid=100 while the
                     * effective uid remains 0 and updates only the bounded
                     * KernelInitTask gid credential fields.
                     * Copy fault returns EFAULT. All invalid
                     * length/family, non-AF_UNIX, abstract, non-UnixSocket0
                     * and non-pathname shapes still return ENOSYS with the
                     * stable connect diagnostic. The focused diagnostic run
                     * classifies the early BusyBox-init 110-byte sockaddr calls as
                     * pathname sockets under /run/utmps, while the post-auth
                     * fchown/fchmod boundary was a 24-byte pathname sockaddr
                     * for /var/run/nscd/socket; both satisfy the AF_UNIX
                     * unix_validate_addr() length/family gate, and missing
                     * paths may now return ENOENT, exposing setgroups(159);
                     * after the SetGroups slice, setgid(144) gid=100 returns
                     * 0 in the bounded root-euid credential slice. The
                     * earlier syslog-like AF_UNIX/SOCK_DGRAM path and all
                     * remaining socket operations keep using the unsupported
                     * diagnostic:
                     * name socket, decoded domain/type/protocol, base type
                     * after SOCK_CLOEXEC/SOCK_NONBLOCK removal, and DGRAM
                     * syslog-like classification.
                     */
                    syscall_unsupported_socket_diagnostic_first_slice(self);
                    syscall_unsupported_connect_sockaddr_diagnostic_first_slice(self);
                    syscall_connect_supported(self);
                    syscall_connect_routes_to_files_struct(self, FilesStruct);
                    syscall_connect_af_unix_pathname_failure_first_slice(self);
                    syscall_socket_supported(self);
                    syscall_socket_routes_to_files_struct(self, FilesStruct);
                    syscall_socket_af_unix_stream_first_slice(self);
                    syscall_socket_backend_full_linux_model_deferred(self);
                    syscall_setgroups_routes_to_user_task(self, KernelInitTask);
                    syscall_getgroups_routes_to_user_task(self, KernelInitTask);
                    syscall_getgroups_bounded_supplementary_groups_first_slice(self);
                    syscall_setgroups_root_first_slice(self);
                    syscall_setgroups_bounded_supplementary_groups_first_slice(self);
                    syscall_nanosleep_full_hrtimer_deferred(self);
                    syscall_rt_sigtimedwait_sigsetsize_bound(self);
                    syscall_rt_sigtimedwait_copies_wait_mask(self);
                    syscall_rt_sigtimedwait_uinfo_null_no_copyout_first_slice(self);
                    syscall_rt_sigtimedwait_uts_null_infinite_wait_first_slice(self);
                    syscall_rt_sigtimedwait_empty_pending_wait_boundary(self);
                    syscall_rt_sigtimedwait_waitqueue_sleep_first_slice(self);
                    syscall_rt_sigtimedwait_sigchld_pending_first_slice(self);
                    syscall_rt_sigtimedwait_return_signal_first_slice(self);
                    syscall_execve_envp_full_copy_deferred(self);
                    syscall_execve_close_on_exec_deferred(self);
                    syscall_execve_old_user_backing_reclaimed_first_slice(self);
                    syscall_execve_old_mm_reclaim_deferred(self);
                    syscall_execve_full_linux_model_deferred(self);
                    syscall_wait4_child_exit_status_copyout_first_slice(self);
                    syscall_wait4_observed_child_reap_first_slice(self);
                    syscall_wait4_wnohang_no_waitable_child_first_slice(self);
                    syscall_wait4_no_child_echild_first_slice(self);
                    syscall_wait4_blocking_sleep_deferred(self);
                    /*
                     * canonical rc-local ends with BusyBox poweroff -f. Its
                     * observed Linux ABI sequence is sync(81), followed by
                     * reboot(142) with LINUX_REBOOT_MAGIC1,
                     * LINUX_REBOOT_MAGIC2 and LINUX_REBOOT_CMD_POWER_OFF.
                     * The current in-memory rootfs has no delayed writeback,
                     * so sync is a successful barrier. Reboot accepts only
                     * that exact power-off command; other combinations remain
                     * rejected.
                     */
                    syscall_sync_rootfs_barrier_first_slice(self);
                    syscall_reboot_poweroff_magic_first_slice(self);
                    syscall_exit_records_status(self);
                    syscall_exit_group_pid1_shutdown_child_wait4_split(self);
                    syscall_setsid_process_group_leader_eperm_first_slice(self);
                    syscall_setsid_user_process_registry_child_success_first_slice(self, UserProcessRegistry);
                    syscall_trace_probe_observes_returns_without_side_effect(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            syscall_table_ready(self);
            syscall_table_bound_to_exception(self, CurrentCPU.trap.exception.syscall);
            syscall_table_write_supported(self);
            syscall_table_writev_supported(self);
            syscall_table_openat_supported(self);
            syscall_table_chdir_supported(self);
            syscall_table_read_supported(self);
            syscall_table_ppoll_supported(self);
            syscall_table_close_supported(self);
            syscall_table_newfstatat_supported(self);
            syscall_table_readlinkat_supported(self);
            syscall_table_dup3_supported(self);
            syscall_table_fchown_supported(self);
            syscall_table_fchmod_supported(self);
            syscall_table_fcntl_supported(self);
            syscall_table_ioctl_supported(self);
            syscall_table_getrandom_supported(self);
            syscall_table_getuid_supported(self);
            syscall_table_getgid_supported(self);
            syscall_table_getpgid_supported(self);
            syscall_table_getsid_supported(self);
            syscall_table_setpgid_supported(self);
            syscall_table_setsid_supported(self);
            syscall_table_setuid_supported(self);
            syscall_table_setgid_supported(self);
            syscall_table_getgroups_supported(self);
            syscall_table_setgroups_supported(self);
            syscall_table_rt_sigprocmask_supported(self);
            syscall_table_rt_sigaction_supported(self);
            syscall_table_rt_sigtimedwait_supported(self);
            syscall_table_clock_gettime_supported(self);
            syscall_table_gettimeofday_supported(self);
            syscall_table_nanosleep_supported(self);
            syscall_table_brk_supported(self);
            syscall_table_mmap_supported(self);
            syscall_table_mprotect_supported(self);
            syscall_table_munmap_supported(self);
            syscall_table_set_tid_address_supported(self);
            syscall_table_clone_supported(self);
            syscall_table_execve_supported(self);
            syscall_table_wait4_supported(self);
            syscall_table_exit_supported(self);
            syscall_table_exit_group_supported(self);
            syscall_write_usercopy_ready(self);
            syscall_writev_usercopy_ready(self);
            syscall_read_usercopy_ready(self);
            syscall_fixed_usercopy_checks_mapping_permissions(self, UserAddressSpace);
            syscall_ppoll_pollfd_usercopy_ready(self);
            syscall_getrandom_usercopy_ready(self);
            syscall_signal_mask_usercopy_ready(self);
            syscall_signal_action_usercopy_ready(self);
            syscall_time_usercopy_ready(self);
            syscall_nanosleep_usercopy_ready(self);
            syscall_ioctl_usercopy_ready(self);
            syscall_path_usercopy_ready(self);
            syscall_stat_usercopy_ready(self);
            syscall_write_routes_to_console(self);
            syscall_read_no_ready_blocking_out_of_slice(self);
            syscall_read_tty_read_wait_entry_first_slice(self);
            syscall_read_tty_read_wait_finish_first_slice(self);
            syscall_ppoll_timeout_parse_first_slice(self);
            syscall_ppoll_sigmask_deferred(self);
            syscall_ppoll_no_ready_blocking_out_of_slice(self);
            syscall_ppoll_poll_table_first_slice(self);
            syscall_ppoll_poll_freewait_first_slice(self);
            syscall_ppoll_blocking_wait_deferred(self);
            syscall_ioctl_tty_full_linux_model_deferred(self);
            syscall_nanosleep_full_hrtimer_deferred(self);
            syscall_rt_sigtimedwait_sigsetsize_bound(self);
            syscall_rt_sigtimedwait_copies_wait_mask(self);
            syscall_rt_sigtimedwait_uinfo_null_no_copyout_first_slice(self);
            syscall_rt_sigtimedwait_uts_null_infinite_wait_first_slice(self);
            syscall_rt_sigtimedwait_empty_pending_wait_boundary(self);
            syscall_rt_sigtimedwait_waitqueue_sleep_first_slice(self);
            syscall_rt_sigtimedwait_sigchld_pending_first_slice(self);
            syscall_rt_sigtimedwait_return_signal_first_slice(self);
            syscall_execve_envp_full_copy_deferred(self);
            syscall_execve_close_on_exec_deferred(self);
            syscall_execve_old_user_backing_reclaimed_first_slice(self);
            syscall_execve_old_mm_reclaim_deferred(self);
            syscall_execve_full_linux_model_deferred(self);
            syscall_wait4_child_exit_status_copyout_first_slice(self);
            syscall_wait4_observed_child_reap_first_slice(self);
            syscall_wait4_no_child_echild_first_slice(self);
            syscall_wait4_blocking_sleep_deferred(self);
            syscall_sync_rootfs_barrier_first_slice(self);
            syscall_reboot_poweroff_magic_first_slice(self);
            syscall_exit_records_status(self);
            syscall_exit_group_pid1_shutdown_child_wait4_split(self);
            syscall_setsid_process_group_leader_eperm_first_slice(self);
            syscall_setsid_user_process_registry_child_success_first_slice(self, UserProcessRegistry);
            syscall_setgroups_routes_to_user_task(self, KernelInitTask);
            syscall_getgroups_routes_to_user_task(self, KernelInitTask);
            syscall_getgroups_bounded_supplementary_groups_first_slice(self);
            syscall_setgroups_root_first_slice(self);
            syscall_setgroups_bounded_supplementary_groups_first_slice(self);
        }

        actions {
            on Action::Write {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                }

                drives {
                    FilesStruct.Action::LookupFd(FdRef::Stdout);
                    FilesStruct.Action::WriteNullFd;
                    FileDescriptorTable.Action::Lookup(FdRef::Stdout);
                    OpenFileDescription.Action::Write;
                    FileBackend.Action::WriteCharDevice;
                }

                ensures {
                    syscall_write_usercopy_ready(self);
                    files_struct_fd_lookup_routes_to_table(FilesStruct, FileDescriptorTable);
                    fd_table_lookup_returns(FileDescriptorTable, FdRef::Stdout, OpenFileDescription);
                    open_file_description_write_dispatches_backend(OpenFileDescription, FileBackend);
                    file_backend_write_to_console(FileBackend) ||
                        files_struct_null_device_write_discard_observed(FilesStruct);
                    syscall_table_write_observed(self);
                }
            }

            on Action::Writev {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    syscall_writev_usercopy_ready(self);
                }

                drives {
                    FilesStruct.Action::LookupFd(FdRef::Stderr);
                    FileDescriptorTable.Action::Lookup(FdRef::Stderr);
                    OpenFileDescription.Action::Write;
                    FileBackend.Action::WriteCharDevice;
                }

                ensures {
                    syscall_writev_routes_to_files_struct(self, FilesStruct);
                    files_struct_fd_lookup_routes_to_table(FilesStruct, FileDescriptorTable);
                    open_file_description_write_dispatches_backend(OpenFileDescription, FileBackend);
                    file_backend_write_to_console(FileBackend);
                    syscall_table_writev_observed(self);
                }
            }

            on Action::OpenAt {
                /*
                 * Linux 6.12 sys_openat() forces O_LARGEFILE on 64-bit and
                 * then reaches fs/open.c::build_open_flags(). O_DIRECTORY is
                 * only translated to LOOKUP_DIRECTORY, matching the uapi
                 * comment "must be a directory"; it is not required in order
                 * to open a directory. O_NONBLOCK is a file status flag in the
                 * uapi fcntl header and is observed by tty open paths through
                 * filp->f_flags. This slice therefore resolves ordinary paths
                 * through VfsCore and installs either the regular-file or
                 * directory opened instance according to the target inode,
                 * while /dev/tty and /dev/tty[0-9]+ may install the existing
                 * console-like character-device opened instance. /dev/null is
                 * a separate built-in character-device alias recognized before
                 * ordinary filesystem write-permission rejection: O_RDONLY,
                 * O_WRONLY and O_RDWR with O_LARGEFILE/O_CLOEXEC/O_NONBLOCK
                 * install a Null OFD in the lowest free fd slot and preserve
                 * status flags plus close-on-exec. The LTP builtin-grandchild
                 * stderr redirect additionally accepts exactly the observed
                 * O_WRONLY|O_CREAT|O_TRUNC|O_LARGEFILE shape for /dev/null;
                 * create/truncate remain rejected for every other path. If
                 * O_DIRECTORY is present and the resolved non-TTY target is
                 * not a directory, including /dev/null, the syscall must fail
                 * with ENOTDIR. O_NONBLOCK is consumed only by accepted TTY and
                 * /dev/null opens in this slice; regular and directory opens do
                 * not gain nonblocking read/write semantics. Write/create modes,
                 * O_PATH, O_TMPFILE, nofollow, permissions, LSM hooks, mount
                 * namespaces, real VT/devtmpfs, a generic char-device registry
                 * and full errno detail remain trimmed.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FsStruct.state == State::Ready;
                    VfsCore.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    syscall_path_usercopy_ready(self);
                }

                drives {
                    FilesStruct.Action::OpenPath;
                    FilesStruct.Action::OpenNullPath;
                    FileDescriptorTable.Action::Install(FdRef::Regular0);
                }

                ensures {
                    syscall_openat_routes_to_files_struct(self, FilesStruct);
                    syscall_openat_builtin_grandchild_dev_null_redirect_bound(self, FilesStruct);
                    files_struct_ltp_runtest_syscalls_read_capacity_bound(FilesStruct);
                    files_struct_open_path_routes_to_vfs(FilesStruct, VfsCore);
                    files_struct_regular_fd_installed(FilesStruct) ||
                        files_struct_directory_fd_installed(FilesStruct) ||
                        files_struct_tty_alias_fd_installed(FilesStruct) ||
                        files_struct_null_fd_installed(FilesStruct);
                    fd_table_fd_installed(FileDescriptorTable, FdRef::Regular0, OpenFileDescription);
                    syscall_table_openat_observed(self);
                }
            }

            on Action::Chdir {
                /*
                 * Linux 6.12 fs/open.c::SYSCALL_DEFINE1(chdir) obtains a
                 * filename from user memory, resolves it through
                 * user_path_at(AT_FDCWD, ..., LOOKUP_FOLLOW | LOOKUP_DIRECTORY),
                 * checks path_permission(MAY_EXEC | MAY_CHDIR), then commits
                 * current->fs pwd with set_fs_pwd().
                 *
                 * This first slice is intentionally narrower: it covers the
                 * observed BusyBox-init chdir("/") path after native init has
                 * opened and closed "/", reuses the existing AT_FDCWD/rooted
                 * VfsCore path walk and requires the resolved target to be a
                 * directory through FsStruct.Action::Chdir. Permission checks,
                 * LSM hooks, refcount/seqcount locking and ESTALE retry remain
                 * deferred. Unsupported path-walk shapes stay outside the slice
                 * and may return ENOSYS instead of pretending to be complete
                 * Linux chdir(2).
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FsStruct.state == State::Ready;
                    VfsCore.state == State::Ready;
                    syscall_path_usercopy_ready(self);
                }

                drives {
                    FsStruct.Action::Chdir(Dentry);
                }

                ensures {
                    syscall_chdir_routes_to_fs_struct(self, FsStruct);
                    syscall_chdir_linux_6_12_path_walk_bound(self);
                    syscall_chdir_root_first_slice(self);
                    syscall_chdir_permissions_lsm_deferred(self);
                    fs_struct_pwd_dentry_set(FsStruct, Dentry);
                    syscall_table_chdir_observed(self);
                }
            }

            on Action::Read {
                /*
                 * Linux 6.12 routes read(2) through fs/read_write.c::ksys_read()
                 * / vfs_read() after fs/file.c fd lookup. For a tty fd, the
                 * file operation enters drivers/tty/tty_io.c::tty_read() and
                 * the N_TTY line discipline read path in drivers/tty/n_tty.c.
                 *
                 * This slice supports four already-open fd classes: the
                 * existing Regular0 read-only file, fd0/tty char-device stdin
                 * through the minimal N_TTY line discipline, /dev/null Null
                 * fds, and the bounded shared pipe backing. Null reads return
                 * EOF (0) immediately and do not
                 * enter the TTY input wait path. In canonical mode,
                 * NTtyLineDiscipline only reports a line readable once bounded
                 * TtyFlipBuffer ready data contains a newline and read returns
                 * at most through that newline. In noncanonical mode, existing
                 * byte readiness is preserved. For a nonzero read request on
                 * fd0 with no N_TTY-readable data, Linux would wait unless
                 * nonblocking, hangup, signal or another terminal condition
                 * applies. The current wait boundary mirrors the Linux
                 * n_tty_read() registration shape by making a tty read_wait
                 * entry/finish pair observable, then opens a supervisor
                 * interruptible window for real RX. It still does not model the
                 * scheduler sleep behind wait_woken(). An empty blocking pipe
                 * read with a live writer instead registers the current
                 * TaskRef/fixed CpuRef, declares scheduler sleep and retries
                 * the shared backing after write/last-writer-close wake; the
                 * sleep-before-Suspend race is closed by the Scheduler's
                 * matching pending-wake path. Job control, signal interruption/restart,
                 * echo/erase, special input characters and full poll/ppoll
                 * integration remain deferred. A user-read-trace probe may
                 * observe fd, requested length and the returned read result for
                 * distro debugging, but must not alter this action's return
                 * value, errno path, checkpoint ordering or smoke policy.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    syscall_read_usercopy_ready(self);
                }

                drives {
                    FilesStruct.Action::ReadFd(FdRef::Regular0);
                    FilesStruct.Action::ReadFd(FdRef::Stdin);
                    FilesStruct.Action::ReadPipeFd;
                    FilesStruct.Action::ReadNullFd;
                    FileBackend.Action::ReadCharDevice;
                    NTtyLineDiscipline.Action::ReadLineOrBytes;
                    TtyInputWait.Action::WaitReadable;
                }

                ensures {
                    syscall_read_routes_to_files_struct(self, FilesStruct);
                    files_struct_regular_file_read_observed(FilesStruct);
                    syscall_read_stdin_ready_data_first_slice(self);
                    files_struct_stdin_char_device_read_observed(FilesStruct);
                    files_struct_null_device_read_eof_observed(FilesStruct);
                    open_file_description_read_observed(OpenFileDescription);
                    file_backend_regular_file_read_returns_data(FileBackend);
                    file_backend_char_device_read_returns_ready_data(FileBackend);
                    tty_flip_buffer_ready_data_consumed(TtyFlipBuffer);
                    syscall_read_no_ready_blocking_out_of_slice(self);
                    syscall_read_pipe_empty_blocks_while_writer_live(self);
                    syscall_read_pipe_wait_uses_fixed_cpu_scheduler(self);
                    syscall_read_pipe_wait_retries_after_wake(self);
                    syscall_read_pipe_wait_no_lost_wake(self);
                    syscall_read_tty_input_wait_first_slice(self);
                    syscall_read_n_tty_line_discipline_first_slice(self);
                    syscall_read_tty_read_wait_entry_first_slice(self);
                    syscall_read_tty_read_wait_finish_first_slice(self);
                    tty_input_wait_read_wait_entry_first_slice(TtyInputWait);
                    tty_input_wait_read_wait_finish_first_slice(TtyInputWait);
                    tty_input_wait_scheduler_sleep_deferred(TtyInputWait);
                    n_tty_canonical_read_returns_through_newline(NTtyLineDiscipline);
                    n_tty_noncanonical_byte_readiness_first_slice(NTtyLineDiscipline);
                    n_tty_echo_and_erase_deferred(NTtyLineDiscipline);
                    syscall_read_tty_blocking_deferred(self);
                    n_tty_full_waitqueue_deferred(NTtyLineDiscipline);
                    syscall_read_trace_probe_observes_result_without_side_effect(self);
                    syscall_table_read_observed(self);
                }
            }

            on Action::Ppoll {
                /*
                 * Linux 6.12 routes ppoll(2) through
                 * fs/select.c::sys_ppoll()/do_sys_poll()/do_pollfd().
                 * The core shape copies an array of struct pollfd, treats
                 * negative fd entries as ignored, reports POLLNVAL for invalid
                 * fd entries, filters readiness with events|POLLERR|POLLHUP,
                 * writes back revents, and returns the number of ready entries.
                 *
                 * This first slice observes immediately available readiness
                 * from the existing fd table, char-device N_TTY readiness and
                 * /dev/null entry flags. Null fds report read/write readiness
                 * immediately according to the fd entry access bits and never
                 * enter the TTY input wait path. Canonical mode requires a
                 * newline-terminated bounded input slice before fd0 reports
                 * POLLIN; noncanonical mode preserves byte readiness. It
                 * parses and validates the optional timeout.
                 * If no entry is ready, timeout={0,0} returns 0 as an immediate
                 * timeout, but timeout=NULL or a positive timeout must not be
                 * reported as successful timeout without an event. Sleeping
                 * The current wait boundary mirrors Linux poll_wqueues by
                 * making poll_table registration and poll_freewait observable,
                 * then opens the same supervisor interruptible window for real
                 * RX. The scheduler sleep, remaining timeout update, temporary
                 * signal masks, restart after signal delivery, echo/erase and
                 * full N_TTY wait queues remain deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    syscall_ppoll_pollfd_usercopy_ready(self);
                }

                drives {
                    FilesStruct.Action::LookupFd(FdRef::Stdin);
                    FileDescriptorTable.Action::Lookup(FdRef::Stdin);
                    NTtyLineDiscipline.Action::EvaluateReadiness;
                    TtyInputWait.Action::WaitReadable;
                }

                ensures {
                    syscall_ppoll_routes_to_files_struct(self, FilesStruct);
                    files_struct_fd_lookup_routes_to_table(FilesStruct, FileDescriptorTable);
                    syscall_ppoll_ready_data_first_slice(self);
                    syscall_ppoll_timeout_parse_first_slice(self);
                    syscall_ppoll_sigmask_deferred(self);
                    syscall_ppoll_no_ready_blocking_out_of_slice(self);
                    syscall_ppoll_tty_input_wait_first_slice(self);
                    syscall_ppoll_n_tty_readiness_first_slice(self);
                    syscall_ppoll_poll_table_first_slice(self);
                    syscall_ppoll_poll_freewait_first_slice(self);
                    tty_input_wait_poll_table_first_slice(TtyInputWait);
                    tty_input_wait_poll_freewait_first_slice(TtyInputWait);
                    tty_input_wait_scheduler_sleep_deferred(TtyInputWait);
                    n_tty_canonical_line_readiness_first_slice(NTtyLineDiscipline);
                    n_tty_noncanonical_byte_readiness_first_slice(NTtyLineDiscipline);
                    n_tty_echo_and_erase_deferred(NTtyLineDiscipline);
                    syscall_ppoll_blocking_wait_deferred(self);
                    n_tty_full_waitqueue_deferred(NTtyLineDiscipline);
                    syscall_table_ppoll_observed(self);
                }
            }

            on Action::Close {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(FileDescriptorTable, FdRef::Regular0, OpenFileDescription);
                }

                drives {
                    FilesStruct.Action::CloseFd(FdRef::Regular0);
                }

                ensures {
                    syscall_close_routes_to_files_struct(self, FilesStruct);
                    files_struct_regular_file_closed(FilesStruct);
                    fd_table_fd_closed(FileDescriptorTable, FdRef::Regular0);
                    syscall_table_close_observed(self);
                }
            }

            on Action::NewFstatAt {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FsStruct.state == State::Ready;
                    VfsCore.state == State::Ready;
                    syscall_path_usercopy_ready(self);
                    syscall_stat_usercopy_ready(self);
                }

                drives {
                    FilesStruct.Action::StatPath;
                    FileBackend.Action::StatRegularFile;
                }

                ensures {
                    syscall_newfstatat_routes_to_files_struct(self, FilesStruct);
                    files_struct_stat_path_routes_to_vfs(FilesStruct, VfsCore);
                    files_struct_regular_file_stat_observed(FilesStruct);
                    file_backend_regular_file_stat_returns_metadata(FileBackend);
                    syscall_table_newfstatat_observed(self);
                }
            }

            on Action::ReadlinkAt {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FsStruct.state == State::Ready;
                    VfsCore.state == State::Ready;
                    syscall_path_usercopy_ready(self);
                    syscall_read_usercopy_ready(self);
                }

                drives {
                    FilesStruct.Action::ReadlinkPath;
                }

                ensures {
                    syscall_readlinkat_routes_to_files_struct(self, FilesStruct);
                    files_struct_readlink_path_routes_to_vfs(FilesStruct, VfsCore);
                    syscall_table_readlinkat_observed(self);
                }
            }

            on Action::Dup3 {
                /*
                 * Linux 6.12 RISC-V exposes dup3 as __NR_dup3=24. The syscall
                 * enters fs/file.c::ksys_dup3(), accepts only flags 0 or
                 * O_CLOEXEC, rejects oldfd == newfd with EINVAL, rejects bad
                 * oldfd or out-of-range newfd with EBADF, and otherwise
                 * routes to do_dup2() to replace newfd with the same struct
                 * file as oldfd. This first slice keeps that user-visible
                 * fd-table shape through FilesStruct while deferring rlimit,
                 * expand_files(), EBUSY, locking and file lifecycle details.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                }

                drives {
                    FilesStruct.Action::Dup3Fd(FdRef::Stdin, FdRef::Stdout);
                    FileDescriptorTable.Action::Dup3(FdRef::Stdin, FdRef::Stdout);
                }

                ensures {
                    syscall_dup3_routes_to_files_struct(self, FilesStruct);
                    files_struct_dup3_routes_to_table(FilesStruct, FileDescriptorTable);
                    fd_table_fd_duplicated(FileDescriptorTable, FdRef::Stdin, FdRef::Stdout);
                    syscall_table_dup3_observed(self);
                }
            }

            on Action::Pipe2 {
                /*
                 * Linux asm-generic/RISC-V exposes pipe2 as __NR_pipe2=59.
                 * This bounded slice accepts only flags == 0, routes pair
                 * allocation through FilesStruct, copies two 32-bit fds to
                 * user memory, and rolls both descriptors back on EFAULT or
                 * capacity failure. The bounded blocking read/wake handshake
                 * is included; O_CLOEXEC/O_NONBLOCK, unbounded wait queues and
                 * signal-producing EPIPE semantics stay deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    syscall_fixed_usercopy_checks_mapping_permissions(self, UserAddressSpace);
                }

                drives {
                    FilesStruct.Action::CreatePipe2;
                }

                ensures {
                    syscall_pipe2_routes_to_files_struct(self, FilesStruct);
                    syscall_pipe2_flags_zero_first_slice(self);
                    syscall_pipe2_atomic_fd_and_usercopy_rollback(self);
                    syscall_pipe2_full_linux_model_deferred(self);
                    files_struct_pipe_fd_pair_installed_atomically(FilesStruct);
                    files_struct_pipe_snapshot_preserves_child_data(FilesStruct);
                    syscall_table_pipe2_observed(self);
                }
            }

            on Action::Fchown {
                /*
                 * Linux asm-generic/RISC-V exposes fchown as __NR_fchown=55.
                 * The current BusyBox init login focused run observes BusyBox login
                 * calling fchown(fd=0, uid=1000, gid=100) after password
                 * authentication. This first slice only validates the fd via
                 * FilesStruct/FileDescriptorTable, records fd-local owner
                 * metadata, and returns success. Bad fd returns EBADF through
                 * the common fd lookup path. Full inode ownership mutation,
                 * permission/capability checks, TTY ownership, path chown,
                 * idmapped mounts, namespaces, LSM and full supplementary
                 * group credential semantics remain deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                }

                drives {
                    FilesStruct.Action::FchownFd(FdRef::Stdin);
                    FileDescriptorTable.Action::UpdateFdOwner(FdRef::Stdin);
                }

                ensures {
                    syscall_fchown_routes_to_files_struct(self, FilesStruct);
                    syscall_fchown_fchmod_fd_local_first_slice(self);
                    syscall_fchown_fchmod_full_linux_model_deferred(self);
                    files_struct_fchown_fd_routes_to_table(FilesStruct, FileDescriptorTable);
                    files_struct_fd_owner_recorded(FilesStruct);
                    syscall_table_fchown_observed(self);
                }
            }

            on Action::Fchmod {
                /*
                 * Linux asm-generic/RISC-V exposes fchmod as __NR_fchmod=52.
                 * The current BusyBox init login focused run observes BusyBox login
                 * calling fchmod(fd=0, mode=0600) immediately after fchown.
                 * This first slice only validates the fd through the fd table,
                 * records a fd-local mode override, and lets fstat on that fd
                 * report the overridden permission bits while preserving file
                 * type bits. It does not persist inode mode or enforce chmod
                 * authorization.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                }

                drives {
                    FilesStruct.Action::FchmodFd(FdRef::Stdin);
                    FileDescriptorTable.Action::UpdateFdMode(FdRef::Stdin);
                }

                ensures {
                    syscall_fchmod_routes_to_files_struct(self, FilesStruct);
                    syscall_fchown_fchmod_fd_local_first_slice(self);
                    syscall_fchown_fchmod_full_linux_model_deferred(self);
                    files_struct_fchmod_fd_routes_to_table(FilesStruct, FileDescriptorTable);
                    files_struct_fd_mode_override_recorded(FilesStruct);
                    files_struct_fchmod_mode_visible_to_fstat(FilesStruct);
                    syscall_table_fchmod_observed(self);
                }
            }

            on Action::Fcntl {
                /*
                 * Linux 6.12 routes fcntl(2) through fs/fcntl.c::do_fcntl().
                 * This first slice covers F_GETFL, F_SETFL for the current
                 * console-like TTY O_NONBLOCK status bit, F_GETFD and
                 * F_SETFD. F_SETFL updates file status flags and does not
                 * rewrite the access mode; the close-on-exec bit lives in
                 * FileDescriptorTable state, not in struct file status flags.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                }

                drives {
                    FilesStruct.Action::LookupFd(FdRef::Regular0);
                    FilesStruct.Action::GetFdFlags(FdRef::Regular0);
                    FilesStruct.Action::SetStatusFlags(FdRef::Regular0);
                    FilesStruct.Action::SetFdFlags(FdRef::Regular0);
                }

                ensures {
                    syscall_fcntl_routes_to_files_struct(self, FilesStruct);
                    files_struct_fd_lookup_routes_to_table(FilesStruct, FileDescriptorTable);
                    fd_table_cloexec_bit_returned_by_fgetfd(FileDescriptorTable, FdRef::Regular0);
                    fd_table_cloexec_bit_updated_by_fsetfd(FileDescriptorTable, FdRef::Regular0);
                    syscall_table_fcntl_observed(self);
                }
            }

            on Action::Ioctl(child: Task) {
                /*
                 * Linux 6.12 routes ioctl(2) through fs/ioctl.c before
                 * dispatching tty fds to drivers/tty/tty_io.c::tty_ioctl().
                 * TCGETS and TCSETS are handled by
                 * drivers/tty/tty_ioctl.c::tty_mode_ioctl(); TCSETS copies a
                 * user struct termios and updates the tty's current termios
                 * via set_termios(..., TERMIOS_OLD), while TCGETS copies the
                 * current termios back to user memory.
                 *
                 * The current slice covers console-like char-device fds, the
                 * riscv64/generic 36-byte old struct termios, TCGETS readback,
                 * TCSETS immediate mutation, and TIOCGPGRP/TIOCSPGRP plus
                 * TIOCGSID/TIOCSCTTY against the generation-checked current
                 * process occurrence and the tty's shared session/foreground
                 * state. A registry child that successfully creates its own
                 * session may acquire the tty and select its same-session pgrp.
                 * TIOCSPGRP accepts PID1 pgrp, the current visible child pgrp,
                 * and the BusyBox init login shell's inherited same-session
                 * child pgrp after observed /bin/ls completion;
                 * unknown pgrp stays ESRCH and cross-session pgrp stays EPERM.
                 * /dev/null validates as an fd but is not a TTY; TTY
                 * ioctl helpers must return ENOTTY for it. TCSETSW/TCSETSF,
                 * drain/flush, driver and line-discipline set_termios hooks,
                 * canonical N_TTY behavior, pty and real TTY locking remain
                 * deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    child.state == State::Online;
                    user_process_registry_contains(UserProcessRegistry, child);
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    syscall_ioctl_usercopy_ready(self);
                }

                drives {
                    FilesStruct.Action::LookupFd(FdRef::Stdout);
                    FilesStruct.Action::ReadTermios(FdRef::Stdout);
                    FilesStruct.Action::SetTermios(FdRef::Stdout);
                }

                ensures {
                    syscall_ioctl_routes_to_files_struct(self, FilesStruct);
                    files_struct_fd_lookup_routes_to_table(FilesStruct, FileDescriptorTable);
                    syscall_ioctl_tcgets_termios_first_slice(self);
                    syscall_ioctl_tcsets_termios_mutation_first_slice(self);
                    syscall_ioctl_tiocgpgrp_foreground_pgrp_first_slice(self);
                    syscall_ioctl_tiocspgrp_foreground_pgrp_update_first_slice(self);
                    syscall_ioctl_tiocgsid_session_id_first_slice(self);
                    syscall_ioctl_tiocsctty_controlling_tty_first_slice(self);
                    syscall_ioctl_user_process_registry_current_job_control_first_slice(
                        self,
                        UserProcessRegistry
                    );
                    files_struct_tty_termios_state_bound(FilesStruct);
                    files_struct_tty_termios_mutation_observed(FilesStruct);
                    files_struct_null_device_tty_ioctl_enotty(FilesStruct);
                    user_task_controlling_tty_bound(KernelInitTask);
                    user_task_foreground_pgrp_read_observed(KernelInitTask);
                    user_task_foreground_pgrp_set_observed(KernelInitTask);
                    user_task_foreground_pgrp_accepts_child_pgrp_first_slice(KernelInitTask, child);
                    user_task_tty_session_id_read_observed(KernelInitTask);
                    user_task_tiocsctty_observed(KernelInitTask);
                    user_task_child_controlling_tty_bound(KernelInitTask, child);
                    syscall_ioctl_tty_full_linux_model_deferred(self);
                    syscall_table_ioctl_observed(self);
                }
            }

            on Action::GetRandom {
                /*
                 * Linux 6.12 exposes getrandom(2) as syscall number 278 in
                 * include/uapi/asm-generic/unistd.h. The syscall body in
                 * drivers/char/random.c::sys_getrandom validates flags, waits
                 * for crng readiness when required, imports the user buffer
                 * and fills it through get_random_bytes_user(). This is not a
                 * /dev/random or /dev/urandom pathname operation.
                 *
                 * The current slice serves sh_probe/BusyBox startup probing:
                 * small user buffers and flags == 0 are routed through the
                 * existing Linux-like HwRngCore current-device read path,
                 * backed by virtio-rng when present. Full CRNG pool state,
                 * blocking wait queues, GRND_RANDOM, GRND_INSECURE,
                 * GRND_NONBLOCK/EAGAIN and large iov iteration are deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    HwRngCore.state == State::Ready;
                    syscall_getrandom_usercopy_ready(self);
                }

                drives {
                    HwRngCore.Action::ReadCurrent;
                }

                ensures {
                    syscall_getrandom_routes_to_hwrng_core(self, HwRngCore);
                    syscall_getrandom_not_vfs_or_devfs_path(self);
                    syscall_getrandom_flags_first_slice_bound(self);
                    syscall_getrandom_full_random_core_deferred(self);
                    hwrng_core_current_rng_read_invoked(HwRngCore, HwRngDevice);
                    hwrng_core_read_copies_from_current(HwRngCore, HwRngDevice);
                    syscall_table_getrandom_observed(self);
                }
            }

            on Action::GetUid {
                /*
                 * Linux 6.12 RISC-V exposes getuid(2) as syscall number 174
                 * in include/uapi/asm-generic/unistd.h; the implementation in
                 * kernel/sys.c::sys_getuid() reads current_uid() from the
                 * current task credentials.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::OnCpu;
                    task_execution_authority_is(
                        KernelInitTask,
                        TaskExecutionAuthority::Live
                    );
                }


                ensures {
                    syscall_getuid_routes_to_user_task(self, KernelInitTask);
                    user_task_uid_read_observed(KernelInitTask);
                    syscall_table_getuid_observed(self);
                }
            }

            on Action::GetGid {
                /*
                 * Linux 6.12 RISC-V exposes getgid(2) as syscall number 176;
                 * kernel/sys.c::sys_getgid() reads current_gid() from current
                 * task credentials.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                }


                ensures {
                    syscall_getgid_routes_to_user_task(self, KernelInitTask);
                    user_task_gid_read_observed(KernelInitTask);
                    syscall_table_getgid_observed(self);
                }
            }

            on Action::GetPid {
                /*
                 * Linux 6.12 kernel/sys.c::sys_getpid() returns
                 * task_tgid_vnr(current). The bounded first slice reads the
                 * current syscall identity: PID1 current returns the preserved
                 * PID1 task identity, while a current independent child Task
                 * returns that Task's own pid.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                }


                ensures {
                    syscall_getpid_routes_to_user_task(self, KernelInitTask);
                    user_task_pid_read_observed(KernelInitTask);
                    syscall_table_getpid_observed(self);
                }
            }

            on Action::GetPpid {
                /*
                 * Linux 6.12 kernel/sys.c::sys_getppid() reads
                 * current->real_parent under rcu_read_lock(). PID1 is created
                 * from the boot idle/init_task parent, so the visible parent
                 * tgid in init_pid_ns is 0. The current slice records that
                 * visible-PID0 parent fact without modeling the full task tree.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                }


                ensures {
                    syscall_getppid_routes_to_user_task(self, KernelInitTask);
                    user_task_ppid_zero_first_slice(KernelInitTask);
                    user_task_ppid_read_observed(KernelInitTask);
                    syscall_table_getppid_observed(self);
                }
            }

            on Action::GetPgid {
                /*
                 * Linux 6.12 kernel/sys.c::do_getpgid() treats pid==0 as
                 * current and otherwise looks up the visible task under RCU.
                 * The current first slice exposes PID1 and the plain-fork
                 * child produced by clone(220); both remain in the same
                 * session, with child pgrp initially visible to the parent
                 * for BusyBox job-control probing.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                }


                ensures {
                    syscall_getpgid_routes_to_user_task(self, KernelInitTask);
                    user_task_process_group_read_observed(KernelInitTask);
                    syscall_table_getpgid_observed(self);
                }
            }

            on Action::GetSid {
                /*
                 * Linux 6.12 kernel/sys.c::SYSCALL_DEFINE1(getsid) treats
                 * pid==0 as current and otherwise looks up the visible task's
                 * session. The current first slice exposes PID1 and the
                 * observed child identity; successful child setsid updates
                 * only that bounded child SID.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                }


                ensures {
                    syscall_getsid_routes_to_user_task(self, KernelInitTask);
                    user_task_session_id_read_observed(KernelInitTask);
                    syscall_table_getsid_observed(self);
                }
            }

            on Action::SetPgid(child: Task) {
                /*
                 * Linux 6.12 sys_setpgid() normalizes pid==0 to current and
                 * pgid==0 to the normalized pid. After plain fork, the parent
                 * may set its not-yet-exec child into a process group whose id
                 * equals the child pid, and the child may perform the matching
                 * setpgid(0, child_pid). The current first slice resolves the
                 * exact generation-checked current registry occurrence and
                 * either that occurrence or its exact published child. A pgrp
                 * equal to the target PID is created directly; any other pgrp
                 * must already have a live same-session registry member. The
                 * target update commits under the registry lock, so subsequent
                 * TIOCSPGRP observes the same identity index. Unknown pid stays
                 * ESRCH, negative pgid stays EINVAL, and a session leader or
                 * unsupported/cross-session pgrp stays EPERM. The full
                 * tasklist/RCU, PF_FORKNOEXEC transition, security hooks and
                 * thread groups remain deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                    child.state == State::Online;
                    user_process_registry_contains(UserProcessRegistry, child);
                }


                ensures {
                    syscall_setpgid_routes_to_user_task(self, KernelInitTask);
                    syscall_setpgid_child_plain_fork_first_slice(self, child);
                    syscall_setpgid_user_process_registry_current_or_child_first_slice(
                        self,
                        UserProcessRegistry
                    );
                    user_task_process_group_set_observed(KernelInitTask);
                    user_task_child_process_group_set_observed(KernelInitTask, child);
                    user_task_pending_plain_fork_child_setpgid_first_slice(KernelInitTask, child);
                    user_task_pending_plain_fork_child_setpgid_errno_bound(KernelInitTask, child);
                    syscall_table_setpgid_observed(self);
                }
            }

            on Action::SetSid(child: Task) {
                /*
                 * Linux 6.12 kernel/sys.c::ksys_setsid() fails with EPERM
                 * when the current group leader is already a session leader
                 * or when a process-group id equal to the proposed session id
                 * exists. The current first slice preserves PID1's existing
                 * session-leader/process-group-leader EPERM result, and also
                 * admits the observed getty child success path when no same
                 * pid pgrp exists; full tasklist lookup and controlling tty
                 * detach remain deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                    child.state == State::Online;
                    user_process_registry_contains(UserProcessRegistry, child);
                }


                ensures {
                    syscall_setsid_routes_to_user_task(self, KernelInitTask);
                    syscall_setsid_process_group_leader_eperm_first_slice(self);
                    syscall_setsid_child_success_first_slice(self, child);
                    user_task_setsid_eperm_observed(KernelInitTask);
                    user_task_child_setsid_success_observed(KernelInitTask, child);
                    syscall_table_setsid_observed(self);
                }
            }

            on Action::GetEuid {
                /*
                 * Linux 6.12 kernel/sys.c::sys_geteuid() reads current_euid().
                 * The first slice routes it to the current KernelInitTask
                 * root credentials substate.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                }


                ensures {
                    syscall_geteuid_routes_to_user_task(self, KernelInitTask);
                    user_task_euid_read_observed(KernelInitTask);
                    syscall_table_geteuid_observed(self);
                }
            }

            on Action::GetEgid {
                /*
                 * Linux 6.12 kernel/sys.c::sys_getegid() reads current_egid().
                 * The first slice routes it to the current KernelInitTask
                 * root credentials substate.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                }


                ensures {
                    syscall_getegid_routes_to_user_task(self, KernelInitTask);
                    user_task_egid_read_observed(KernelInitTask);
                    syscall_table_getegid_observed(self);
                }
            }

            on Action::GetResUid {
                /*
                 * Linux 6.12 kernel/sys.c::sys_getresuid() snapshots real,
                 * effective and saved uid from current_cred(), then writes the
                 * three uid_t values to user memory in order. User pointer
                 * failure returns EFAULT. The first slice keeps all three root
                 * ids on KernelInitTask and writes riscv64 uid_t-sized
                 * values.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                    syscall_credentials_usercopy_ready(self);
                }


                ensures {
                    syscall_getresuid_routes_to_user_task(self, KernelInitTask);
                    user_task_resuid_read_observed(KernelInitTask);
                    syscall_table_getresuid_observed(self);
                }
            }

            on Action::GetResGid {
                /*
                 * Linux 6.12 kernel/sys.c::sys_getresgid() mirrors getresuid
                 * for real/effective/saved gid. The current slice writes the
                 * three root gid_t values from KernelInitTask.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                    syscall_credentials_usercopy_ready(self);
                }


                ensures {
                    syscall_getresgid_routes_to_user_task(self, KernelInitTask);
                    user_task_resgid_read_observed(KernelInitTask);
                    syscall_table_getresgid_observed(self);
                }
            }

            on Action::GetGroups {
                /*
                 * Linux 6.12 RISC-V exposes getgroups(2) as syscall number
                 * 158 in include/uapi/asm-generic/unistd.h; the implementation
                 * in kernel/groups.c::SYSCALL_DEFINE2(getgroups) returns the
                 * current supplementary group count when gidsetsize is 0,
                 * returns EINVAL when the supplied buffer is smaller than the
                 * current group count, and copies gid_t entries to userspace
                 * otherwise. The current BusyBox init login-shell evidence reaches
                 * getgroups(32, <user gid_t *>) after login has dropped to
                 * uid=1000/gid=100 and after setgroups(1, {100}) populated the
                 * bounded KernelInitTask supplementary group view. This first
                 * slice only reads back that fixed-capacity view; it does not
                 * allocate or sort Linux group_info, expand NGROUPS_MAX, or
                 * connect group membership to permission, inode or TTY
                 * ownership semantics.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                    syscall_credentials_usercopy_ready(self);
                }


                ensures {
                    syscall_getgroups_routes_to_user_task(self, KernelInitTask);
                    syscall_getgroups_bounded_supplementary_groups_first_slice(self);
                    syscall_credentials_full_linux_model_deferred(self);
                    user_task_supplementary_groups_bound(KernelInitTask);
                    user_task_supplementary_groups_read_observed(KernelInitTask);
                    syscall_table_getgroups_observed(self);
                }
            }

            on Action::Uname {
                /*
                 * Linux 6.12 RISC-V maps __NR_uname=160 to sys_newuname().
                 * kernel/sys.c::sys_newuname() copies struct new_utsname,
                 * whose uapi layout is six 65-byte fields. The current slice
                 * exposes the local ../linux-6.12 init_uts_ns generated
                 * values as a static first UTS namespace and defers writable
                 * UTS namespaces, sethostname/setdomainname and personality
                 * release override handling.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    syscall_utsname_usercopy_ready(self);
                }

                ensures {
                    syscall_uname_new_utsname_layout_bound(self);
                    syscall_uname_static_init_uts_namespace_first_slice(self);
                    syscall_uname_full_uts_namespace_deferred(self);
                    syscall_table_uname_observed(self);
                }
            }

            on Action::GetCwd {
                /*
                 * Linux 6.12 fs/d_path.c::sys_getcwd() snapshots
                 * current->fs root/pwd, builds a NUL-terminated path, returns
                 * the copied byte count including NUL, and returns ERANGE when
                 * the user buffer is too small. In addition to the boot root
                 * path, the builtin command-substitution child serializes its
                 * current absolute pwd from the existing dentry parent chain;
                 * this covers `/opt/ltp` after bounded cd/pwd without adding
                 * mount-namespace or concurrent fs_struct semantics.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                    FsStruct.state == State::Ready;
                    VfsCore.state == State::Ready;
                    syscall_getcwd_usercopy_ready(self);
                }


                ensures {
                    syscall_getcwd_routes_to_user_task(self, KernelInitTask);
                    user_task_root_cwd_first_slice(KernelInitTask, FsStruct);
                    syscall_getcwd_returns_root_with_nul(self);
                    syscall_getcwd_builtin_grandchild_absolute_pwd_bound(self, FsStruct, VfsCore);
                    syscall_table_getcwd_observed(self);
                }
            }

            on Action::SetUid {
                /*
                 * Linux 6.12 kernel/sys.c::__sys_setuid() prepares and commits
                 * new credentials for current. The current first slice keeps a
                 * bounded PID1 credential view on KernelInitTask. Following
                 * focused BusyBox-init evidence, it treats euid==0 as the temporary
                 * CAP_SETUID proxy and accepts 32-bit uid targets, syncing
                 * uid/euid/suid/fsuid. Namespaces, full capability checks, LSM
                 * hooks, user accounting and credential COW/RCU are explicit
                 * deferred facts; saved-id/capability regain after dropping to
                 * uid 1000 is not modeled in this slice. The post-slice
                 * BusyBox-init focused rerun confirms setuid(146, uid=1000) returns
                 * 0 and records the next fatal boundary as login shell
                 * execve(221) returning EFAULT; that execve boundary is
                 * deferred to a later slice.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                }


                ensures {
                    syscall_setuid_routes_to_user_task(self, KernelInitTask);
                    syscall_credentials_full_linux_model_deferred(self);
                    user_task_uid_set_observed(KernelInitTask);
                    syscall_table_setuid_observed(self);
                }
            }

            on Action::SetGid {
                /*
                 * Linux 6.12 kernel/sys.c::__sys_setgid() mirrors setuid for
                 * group credentials. The current first slice treats
                 * effective uid 0 as the bounded CAP_SETGID proxy and accepts
                 * the observed gid=100 transition, synchronizing only
                 * KernelInitTask gid/egid/sgid/fsgid. Full capabilities,
                 * user namespaces, LSM hooks and credential COW/RCU remain
                 * deferred. The post-SetGid focused rerun records the
                 * setuid(146) uid=1000 boundary that the SetUid slice closes.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                }


                ensures {
                    syscall_setgid_routes_to_user_task(self, KernelInitTask);
                    syscall_credentials_full_linux_model_deferred(self);
                    user_task_gid_set_observed(KernelInitTask);
                    syscall_table_setgid_observed(self);
                }
            }

            on Action::SetGroups {
                /*
                 * Linux 6.12 RISC-V exposes setgroups(2) as syscall number
                 * 159 in include/uapi/asm-generic/unistd.h, implemented by
                 * kernel/groups.c::SYSCALL_DEFINE2(setgroups). Linux checks
                 * may_setgroups(), bounds gidsetsize by NGROUPS_MAX, copies a
                 * gid_t list from userspace, sorts it and commits a new group
                 * info through current credentials. The current BusyBox init login
                 * evidence reaches setgroups(gidsetsize=1, grouplist=<user
                 * gid_t *>) immediately after the /var/run/nscd/socket
                 * connect(203) ENOENT fallback. This first slice keeps only a
                 * bounded supplementary group view on KernelInitTask: root
                 * effective uid may clear the list with size 0 or copy one
                 * 32-bit gid_t from userspace with size 1; copy fault returns
                 * EFAULT, non-root returns EPERM, and size > 1 remains an
                 * unsupported diagnostic/ENOSYS boundary rather than claiming
                 * full NGROUPS_MAX credentials support. The focused rerun
                 * after this slice confirms /var/run/nscd/socket still
                 * returns ENOENT and setgroups returns 0; the new direct
                 * boundary is setgid(144) with gid=100. The following SetGid
                 * slice accepts that observed root-euid transition without
                 * implementing full credential semantics; the post-SetGid
                 * focused rerun records setuid(146) uid=1000 EPERM, which is
                 * closed by the bounded SetUid slice.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                    syscall_credentials_usercopy_ready(self);
                }


                ensures {
                    syscall_setgroups_routes_to_user_task(self, KernelInitTask);
                    syscall_setgroups_root_first_slice(self);
                    syscall_setgroups_bounded_supplementary_groups_first_slice(self);
                    syscall_credentials_full_linux_model_deferred(self);
                    user_task_supplementary_groups_bound(KernelInitTask);
                    user_task_setgroups_observed(KernelInitTask);
                    syscall_table_setgroups_observed(self);
                }
            }

            on Action::RtSigprocmask {
                /*
                 * Linux 6.12 RISC-V exposes rt_sigprocmask(2) as syscall
                 * number 135. kernel/signal.c::sys_rt_sigprocmask() checks
                 * sigsetsize == sizeof(sigset_t), snapshots current->blocked,
                 * copies an optional new mask, clears SIGKILL/SIGSTOP from it,
                 * applies SIG_BLOCK/SIG_UNBLOCK/SIG_SETMASK via sigprocmask(),
                 * and finally copies the old mask to user memory when oset is
                 * non-null.
                 *
                 * The current slice stores the blocked mask on KernelInitTask
                 * because the PID1 task is the exec-transformed
                 * KernelInitTask. Full signal delivery, shared sighand,
                 * pending queues, restart, thread-group semantics and
                 * siglock/IRQ locking are deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                    syscall_signal_mask_usercopy_ready(self);
                }


                ensures {
                    syscall_rt_sigprocmask_routes_to_user_task(self, KernelInitTask);
                    syscall_rt_sigprocmask_sigsetsize_bound(self);
                    syscall_rt_sigprocmask_unblockable_signals_cleared(self);
                    syscall_signal_delivery_deferred(self);
                    user_task_rt_sigprocmask_observed(KernelInitTask);
                    syscall_table_rt_sigprocmask_observed(self);
                }
            }

            on Action::RtSigaction {
                /*
                 * Linux 6.12 RISC-V exposes rt_sigaction(2) as syscall
                 * number 134. kernel/signal.c::sys_rt_sigaction() first
                 * checks sigsetsize == sizeof(sigset_t), copies the optional
                 * user struct sigaction into a kernel k_sigaction, delegates
                 * validation and table update to do_sigaction(), and copies
                 * the old action back to user memory when oact is non-null.
                 *
                 * do_sigaction() rejects invalid signal numbers and attempts
                 * to install handlers for kernel-only signals such as SIGKILL
                 * and SIGSTOP. For an accepted new action it clears unsupported
                 * userspace flags and removes SIGKILL/SIGSTOP from the stored
                 * action mask before updating sighand->action[sig - 1].
                 *
                 * The current slice models this as Task -> SignalRuntime ->
                 * SignalActionTable, folded into the current KernelInitTask
                 * implementation for single PID1/single-thread execution.
                 * ProcessSignalState shared pending queues, ThreadSignalState
                 * pending delivery, siglock/RCU, restart handling, signal
                 * frame construction and rt_sigreturn remain deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                    syscall_signal_action_usercopy_ready(self);
                }


                ensures {
                    syscall_rt_sigaction_routes_to_user_task(self, KernelInitTask);
                    syscall_rt_sigaction_routes_to_signal_action_table(self, KernelInitTask);
                    syscall_rt_sigaction_sigsetsize_bound(self);
                    syscall_rt_sigaction_layout_bound(self);
                    syscall_rt_sigaction_unblockable_signals_cleared(self);
                    syscall_rt_sigaction_kernel_only_signals_rejected(self);
                    syscall_signal_delivery_deferred(self);
                    user_task_rt_sigaction_observed(KernelInitTask);
                    syscall_table_rt_sigaction_observed(self);
                }
            }

            on Action::RtSigtimedwait {
                /*
                 * Linux 6.12 RISC-V exposes rt_sigtimedwait(2) as syscall
                 * number 137. kernel/signal.c::sys_rt_sigtimedwait() checks
                 * sigsetsize == sizeof(sigset_t), copies the user signal set,
                 * then waits for a matching pending signal or timeout. A
                 * NULL uinfo does not require siginfo_t copyout. A NULL uts
                 * is an infinite wait rather than an immediate EAGAIN.
                 *
                 * The current BusyBox-init evidence reaches
                 * rt_sigtimedwait(uthese, NULL, NULL, 8) after setsid(157).
                 * This first slice copies and records the wait mask, supports
                 * the Linux sigset bit rule 1 << (sig - 1), and recognizes
                 * SIGCHLD as signal 17. If no matching pending SIGCHLD exists,
                 * it records a Linux-like wait reason, enqueues the single
                 * PID1 signal waiter, and stops at the observable waitqueue
                 * sleep boundary rather than returning success, EAGAIN or
                 * ENOSYS. Failed fork or unsupported clone paths are not
                 * signal sources and must not fabricate SIGCHLD.
                 *
                 * A real observed dynamic child exit/exit_group may set
                 * PID1 pending SIGCHLD. If PID1 is sleeping in this
                 * rt_sigtimedwait shape and the saved mask contains
                 * SIGCHLD, the waiter is woken, SIGCHLD is dequeued and the
                 * syscall returns 17 without siginfo_t copyout because uinfo
                 * is NULL. uinfo copyout, uts timeout/remaining timeout,
                 * restart, fatal delivery, signal handlers, shared pending
                 * queues, multithreaded signal semantics and general signal
                 * delivery remain deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                    syscall_signal_mask_usercopy_ready(self);
                }


                ensures {
                    syscall_rt_sigtimedwait_routes_to_user_task(self, KernelInitTask);
                    syscall_rt_sigtimedwait_sigsetsize_bound(self);
                    syscall_rt_sigtimedwait_copies_wait_mask(self);
                    syscall_rt_sigtimedwait_uinfo_null_no_copyout_first_slice(self);
                    syscall_rt_sigtimedwait_uts_null_infinite_wait_first_slice(self);
                    syscall_rt_sigtimedwait_empty_pending_wait_boundary(self);
                    syscall_rt_sigtimedwait_waitqueue_sleep_first_slice(self);
                    syscall_rt_sigtimedwait_sigchld_pending_first_slice(self);
                    syscall_rt_sigtimedwait_return_signal_first_slice(self);
                    user_task_rt_sigtimedwait_observed(KernelInitTask);
                    user_task_rt_sigtimedwait_pending_match_empty(KernelInitTask);
                    user_task_rt_sigtimedwait_infinite_wait(KernelInitTask);
                    user_task_pending_sigchld_first_slice(KernelInitTask);
                    user_task_rt_sigtimedwait_waiter_enqueued(KernelInitTask);
                    user_task_rt_sigtimedwait_sleep_reason_bound(KernelInitTask);
                    user_task_rt_sigtimedwait_woken_by_sigchld(KernelInitTask);
                    user_task_rt_sigtimedwait_dequeued_sigchld(KernelInitTask);
                    syscall_table_rt_sigtimedwait_observed(self);
                }
            }

            on Action::ClockGettime {
                /*
                 * Linux 6.12 RISC-V exposes clock_gettime(2) as syscall
                 * number 113 in include/uapi/asm-generic/unistd.h. The
                 * syscall body in kernel/time/posix-stubs.c::sys_clock_gettime()
                 * calls do_clock_gettime(), then copies a
                 * struct __kernel_timespec to user memory.
                 *
                 * The current slice serves BusyBox/musl startup probing:
                 * CLOCK_REALTIME and CLOCK_MONOTONIC are read from the existing
                 * Timekeeper/RiscvTimerProvider time-read boundary. Full POSIX
                 * timer, vDSO, time namespace, seqcount retry and wall-clock
                 * calibration are deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    Timekeeper.state == State::Ready;
                    RiscvTimerProvider.state == State::Ready;
                    syscall_time_usercopy_ready(self);
                }

                drives {
                    RiscvTimerProvider.Action::ReadTime;
                }

                ensures {
                    syscall_clock_gettime_routes_to_timer_provider(self, RiscvTimerProvider);
                    syscall_clock_gettime_clockid_first_slice_bound(self);
                    syscall_time_struct_layout_bound(self);
                    syscall_time_full_linux_model_deferred(self);
                    syscall_table_clock_gettime_observed(self);
                }
            }

            on Action::Gettimeofday {
                /*
                 * Linux 6.12 RISC-V exposes gettimeofday(2) as syscall number
                 * 169. kernel/time/time.c::sys_gettimeofday() reads
                 * CLOCK_REALTIME via ktime_get_real_ts64(), copies
                 * __kernel_old_timeval when tv is non-null, and copies sys_tz
                 * when tz is non-null.
                 *
                 * The current slice writes timeval from the same timer-provider
                 * read boundary and uses zero timezone fields. Timezone update,
                 * RTC/NTP wall-clock calibration and compat layouts are
                 * deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    Timekeeper.state == State::Ready;
                    RiscvTimerProvider.state == State::Ready;
                    syscall_time_usercopy_ready(self);
                }

                drives {
                    RiscvTimerProvider.Action::ReadTime;
                }

                ensures {
                    syscall_gettimeofday_routes_to_timer_provider(self, RiscvTimerProvider);
                    syscall_time_struct_layout_bound(self);
                    syscall_time_full_linux_model_deferred(self);
                    syscall_table_gettimeofday_observed(self);
                }
            }

            on Action::Nanosleep {
                /*
                 * Linux 6.12 RISC-V exposes nanosleep(2) as syscall number
                 * 101. kernel/time/hrtimer.c::sys_nanosleep() copies a
                 * 64-bit struct __kernel_timespec from rqtp, validates
                 * timespec64, sets restart-block state and enters
                 * hrtimer_nanosleep() against CLOCK_MONOTONIC in relative
                 * mode.
                 *
                 * The current slice is driven by observed BusyBox /bin/sh
                 * evidence: rqtp={0, 20ms}. It supports bounded short
                 * relative sleeps by polling the existing RiscvTimerProvider
                 * time counter until the target tick is reached. rmtp is not
                 * written on success, matching Linux's completed-sleep path.
                 * Signal interruption, remaining-time copyout, restart blocks,
                 * timer slack, wait queues, scheduler sleep and clock_nanosleep
                 * remain deferred.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    Timekeeper.state == State::Ready;
                    RiscvTimerProvider.state == State::Ready;
                    syscall_nanosleep_usercopy_ready(self);
                }

                drives {
                    RiscvTimerProvider.Action::ReadTime;
                }

                ensures {
                    syscall_nanosleep_routes_to_timer_provider(self, RiscvTimerProvider);
                    syscall_nanosleep_timespec_validated(self);
                    syscall_nanosleep_short_relative_first_slice(self);
                    syscall_nanosleep_full_hrtimer_deferred(self);
                    syscall_table_nanosleep_observed(self);
                }
            }

            on Action::Brk {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    UserAddressSpace.state == State::Online;
                }

                drives {
                    UserAddressSpace.Action::Brk;
                }

                ensures {
                    syscall_table_brk_supported(self);
                    syscall_brk_routes_to_user_address_space(self, UserAddressSpace);
                }
            }

            on Action::Mmap {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    UserAddressSpace.state == State::Online;
                }

                drives {
                    UserAddressSpace.Action::Mmap;
                }

                ensures {
                    syscall_table_mmap_supported(self);
                    syscall_mmap_routes_to_user_address_space(self, UserAddressSpace);
                    syscall_mmap_fixed_anonymous_prot_none_first_slice(self);
                    syscall_mmap_full_vma_model_deferred(self);
                }
            }

            on Action::Mprotect {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    UserAddressSpace.state == State::Online;
                }

                drives {
                    UserAddressSpace.Action::Mprotect;
                }

                ensures {
                    syscall_table_mprotect_supported(self);
                    syscall_mprotect_routes_to_user_address_space(self, UserAddressSpace);
                }
            }

            on Action::Munmap {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    UserAddressSpace.state == State::Online;
                }

                drives {
                    UserAddressSpace.Action::Munmap;
                }

                ensures {
                    syscall_table_munmap_supported(self);
                    syscall_munmap_routes_to_user_address_space(self, UserAddressSpace);
                }
            }

            on Action::SetTidAddress {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::Online;
                }


                ensures {
                    syscall_set_tid_address_routes_to_user_task(self, KernelInitTask);
                    user_task_clear_child_tid_bound(KernelInitTask);
                    syscall_table_set_tid_address_observed(self);
                }
            }

            on Action::Clone -> TaskRef {
                /*
                 * Linux clone/fork atomically materializes a fresh Task identity,
                 * TaskRef, ordinary lifetime TaskFlow, FlowRef, private
                 * UserAppRuntime and ApplicationInstance on every execution.
                 *
                 * BusyBox command sequencing and other application behavior
                 * are historical test evidence only. Clone ABI validation,
                 * task/resource copying, scheduling and rollback remain kernel
                 * contracts here.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    TaskCreationCore.state == State::Ready;
                    KernelInitTask.state == State::OnCpu;
                    KernelInitFlow.state == State::Online;
                    KernelInitUserAppRuntime.state == State::Online;
                    UserProcessRegistry.state == State::Ready;
                    user_process_registry_allows_independent_aggregates(UserProcessRegistry);
                    UserCloneDeferredBoundaries.state == State::Ready;
                    Cpu0Scheduler.state == State::Online;
                    RootPidNamespace.state == State::Ready;
                    FsStruct.state == State::Ready;
                    FilesStruct.state == State::Ready;
                    UserAddressSpace.state == State::Online;
                    UserTrapFrame.state == State::Ready;
                }

                drives {
                    declare target_cpu of CpuRef;
                    declare target_scheduler of Scheduler;
                    snapshot fork_child {
                        materialize child of Task at State::Online;
                        materialize child_ref of TaskRef;
                        materialize child_flow of TaskFlow at State::Online;
                        materialize child_flow_ref of TaskFlowRef;
                        materialize child_runtime of UserAppRuntime at State::Online;
                        materialize child_application of ApplicationInstance;
                        materialize activation_ordinal of SchedulerInboxOrdinal;
                        child_ref.Action::Bind(task: child);
                        child_flow.Action::BindOwner(
                            owner_task: child,
                            flow_ref: child_flow_ref
                        );
                        child_runtime.Action::Bind(
                            flow: child_flow,
                            application: child_application
                        );
                        TaskCreationCore.Action::CopyUserProcess(
                            src_process: KernelInitTask,
                            src_ref: KernelInitTaskRef,
                            dst_process: child,
                            dst_ref: child_ref,
                            flow: child_flow,
                            flow_ref: child_flow_ref,
                            runtime: child_runtime,
                            application: child_application,
                            pid_ns: RootPidNamespace,
                            scheduler: target_scheduler,
                            fs: FsStruct,
                            files: FilesStruct,
                            address_space: UserAddressSpace,
                            trap_frame: UserTrapFrame,
                            boundaries: UserCloneDeferredBoundaries
                        );
                        materialize child_aggregate of UserProcessAggregate;
                        materialize child_generation of UserProcessGeneration;
                        UserProcessRegistry.Action::ReserveFork(
                            task: child,
                            aggregate: child_aggregate,
                            generation: child_generation
                        );
                        target_scheduler.Action::ReserveInbound(
                            task_ref: child_ref,
                            target_cpu: target_cpu
                        );
                        UserProcessRegistry.Action::PublishFork(task: child, task_ref: child_ref);
                        target_scheduler.Action::PublishInbound(
                            task_ref: child_ref,
                            target_cpu: target_cpu,
                            ordinal: activation_ordinal
                        );
                        FilesStruct.Action::SaveParentFdSnapshot(child: child);
                    }
                }

                ensures {
                    syscall_clone_routes_to_task_creation_core(self, TaskCreationCore);
                    syscall_clone_routes_to_user_clone_deferred_boundaries(self, UserCloneDeferredBoundaries);
                    syscall_clone_legacy_args_decoded(self);
                    syscall_clone_plain_fork_first_slice(self);
                    syscall_clone_plain_fork_from_serial_vfork_current_mm(self);
                    syscall_clone_vfork_vm_first_slice(self);
                    syscall_clone_vfork_pidfd_first_slice(self);
                    syscall_clone_parent_returns_child_pid(self, KernelInitTask);
                    syscall_clone_child_return_zero_bound(self, child);
                    syscall_clone_pidfd_copyout_bound(self, FilesStruct);
                    syscall_clone_vfork_parent_frame_saved(self, child);
                    syscall_clone_vfork_child_handoff(self, child);
                    syscall_clone_vfork_next_child_accepted(self, child);
                    syscall_clone_vfork_parent_scheduler_blocked(self, child);
                    syscall_clone_vfork_immediate_parent_wake_exactly_once(self, child);
                    syscall_clone_wake_up_new_task_shape(self, target_scheduler);
                    user_child_process_process_group_visible_to_parent(child, KernelInitTask);
                    user_task_child_process_group_visible(KernelInitTask, child);
                    user_child_process_independent_mm_owned(child, UserAddressSpace);
                    user_child_process_root_and_satp_distinct(child, KernelInitTask);
                    user_child_process_cow_private_pages_shared(child, UserAddressSpace);
                    user_child_process_dup_mm_failure_atomic(child, KernelInitTask);
                    user_child_process_mm_published_after_copy(child);
                    user_child_process_parent_fd_snapshot_saved(child, FilesStruct);
                    user_child_process_fd_table_independent(child, FilesStruct);
                    user_child_process_open_backings_aliased(child, FilesStruct);
                    files_struct_parent_fd_snapshot_saved(FilesStruct, child);
                    user_process_registry_contains(UserProcessRegistry, child);
                    user_process_registry_stores_ref(UserProcessRegistry, child_ref);
                    user_process_registry_ref_targets_member(UserProcessRegistry, child_ref, child);
                    user_process_registry_aggregate_owns_process_resources(UserProcessRegistry, child_aggregate);
                    user_process_registry_fork_failure_has_no_residue(UserProcessRegistry);
                    user_clone_plain_fork_pid_mod_cpu_placement(UserCloneDeferredBoundaries);
                    user_clone_cpu_ownership_immutable(UserCloneDeferredBoundaries);
                    user_task_instance_fresh(child);
                    user_task_pid_and_lifecycle_independent(child);
                    task_fixed_flow_is(child, child_flow);
                    task_flow_owner_is(child_flow, child);
                    task_flow_parent_is(child_flow, child);
                    task_flow_owner_exclusive(child_flow);
                    user_app_runtime_owned_by_flow(child_runtime, child_flow);
                    user_app_runtime_unique_for_flow(child_runtime, child_flow);
                    user_app_runtime_not_shared(child_runtime);
                    application_instance_owned_by_runtime(child_application, child_runtime);
                    fork_snapshot_parent_state_matrix(
                        KernelInitTask,
                        KernelInitFlow,
                        KernelInitUserAppRuntime
                    );
                    fork_snapshot_child_state_matrix(child, child_flow, child_runtime);
                    fork_snapshot_identities_all_fresh(
                        KernelInitTask,
                        KernelInitFlow,
                        KernelInitUserAppRuntime,
                        child,
                        child_ref,
                        child_flow,
                        child_flow_ref,
                        child_runtime,
                        child_application
                    );
                    fork_snapshot_child_has_no_parent_overlay_authority(child);
                    fork_snapshot_post_fork_continuation_ready(child);
                    task_fixed_flow_binding_complete(child);
                    task_fixed_flow_binding_consistent(child);
                    child.state == State::Online;
                    child_flow.state == State::Online;
                    child_runtime.state == State::Online;
                    syscall_clone_returns_task_ref(self, child_ref, child);
                    syscall_table_clone_observed(self);
                }

                result {
                    Created: Success(child_ref);
                }
            }

            on Action::Execve(
                task: Task,
                flow: TaskFlow,
                runtime: UserAppRuntime
            ) {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    UserBootPayload.state == State::Online;
                    task.state == State::OnCpu;
                    flow.state == State::Online;
                    runtime.state == State::Online;
                    user_process_registry_contains(UserProcessRegistry, task);
                    task_fixed_flow_is(task, flow);
                    user_app_runtime_owned_by_flow(runtime, flow);
                    user_app_runtime_unique_for_flow(runtime, flow);
                    UserAddressSpace.state == State::Online;
                    UserTrapFrame.state == State::Ready;
                    FilesStruct.state == State::Ready;
                    FsStruct.state == State::Ready;
                    VfsCore.state == State::Ready;
                }

                drives {
                    declare next_application of ApplicationInstance;
                    UserBootPayload.Action::TryDefaultInitSequence;
                    ElfObject.Transition::Preset;
                    ElfObject.Transition::Setup;
                    UserStack.Transition::Setup;
                    UserAddressSpace.Transition::Setup;
                    UserAddressSpace.Transition::Enable;
                    UserTrapFrame.Transition::Setup;
                    runtime.Action::ReplaceApplication(next: next_application);
                }

                ensures {
                    syscall_execve_linux_6_12_do_execveat_common_bound(self);
                    syscall_execve_observed_shell_ls_args_bound(self);
                    syscall_execve_observed_busybox_init_getty_args_bound(self);
                    syscall_execve_child_continuation_first_slice(self, task);
                    syscall_execve_builtin_grandchild_runtime_exec_bound(self, task);
                    syscall_execve_retired_image_retention_classified(self, task, UserAddressSpace, UserStack);
                    syscall_execve_builtin_grandchild_first_parent_image_retained(self, task, UserAddressSpace, UserStack);
                    syscall_execve_builtin_grandchild_subsequent_image_released(self, task, UserAddressSpace, UserStack);
                    syscall_execve_builtin_grandchild_failure_atomic(self, task, FilesStruct);
                    syscall_execve_reuses_user_boot_payload_elf_loader(self, UserBootPayload);
                    syscall_execve_replaces_user_address_space_first_slice(self, UserAddressSpace);
                    syscall_execve_context_staging_address_space_bound(self, UserAddressSpace);
                    syscall_execve_sets_start_thread_frame_first_slice(self, UserTrapFrame);
                    syscall_execve_bounded_argv_first_slice(self);
                    syscall_execve_vector_copy_uses_current_effective_mm(self, task);
                    syscall_execve_stage_checkpoints_bound(self);
                    syscall_execve_return_path_diagnostic_bound(self);
                    syscall_execve_envp_full_copy_deferred(self);
                    syscall_execve_close_on_exec_deferred(self);
                    syscall_execve_old_user_backing_reclaimed_first_slice(self);
                    syscall_execve_old_mm_reclaim_deferred(self);
                    syscall_execve_full_linux_model_deferred(self);
                    task_fixed_flow_is(task, flow);
                    task_flow_owner_is(flow, task);
                    user_app_runtime_identity_preserved(runtime);
                    user_app_runtime_application_replaced(runtime, next_application);
                    user_app_runtime_exec_committed(runtime, next_application);
                    task_fixed_flow_binding_consistent(task);
                    user_process_registry_exec_preserves_identity(UserProcessRegistry, task);
                    syscall_table_execve_observed(self);
                }
            }

            on Action::Wait4(parent: Task, child: Task, scheduler: Scheduler) {
                /*
                 * wait4 operates on independent child Task identities and
                 * their exit/reap records. The explicit parent and child Task
                 * parameters preserve the current-parent identity, Linux
                 * argument, status-copyout and wake/sleep contracts; they do
                 * not encode a reusable child slot or make application
                 * control flow part of the lifetime TaskFlow.
                 */
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    parent.state == State::OnCpu;
                    child.state == State::Online;
                    scheduler.state == State::Online;
                    user_process_registry_contains(UserProcessRegistry, parent);
                    user_process_registry_contains(UserProcessRegistry, child);
                    user_process_registry_parent_child_identity_bound(UserProcessRegistry, parent, child);
                    user_process_registry_slot_state_is(UserProcessRegistry, UserProcessSlotState::Zombie);
                    user_process_registry_reap_excludes_scheduler_and_lease_refs(UserProcessRegistry, child);
                }

                drives { UserProcessRegistry.Action::Reap(child); }

                ensures {
                    syscall_wait4_linux_6_12_kernel_wait4_bound(self);
                    syscall_wait4_observed_shell_args_bound(self);
                    syscall_wait4_pid_minus_one_all_children_first_slice(self);
                    syscall_wait4_positive_pid_exact_child_first_slice(self, parent, child);
                    syscall_wait4_options_wuntraced_first_slice(self);
                    syscall_wait4_options_zero_first_slice(self);
                    syscall_wait4_wnohang_no_waitable_child_first_slice(self);
                    syscall_wait4_completed_child_record_reap_first_slice(self);
                    syscall_wait4_parent_wait_chldexit_boundary(self);
                    syscall_wait4_current_parent_task_bound(self, parent, child);
                    syscall_wait4_owner_scheduler_bound(self, parent, scheduler);
                    user_process_registry_wait_uses_current_parent_identity(UserProcessRegistry, parent, child);
                    user_process_registry_reap_preserves_unselected_parent_state(UserProcessRegistry, parent, child);
                    syscall_wait4_yields_to_user_child_continuation(self, child);
                    user_child_process_wait4_parent_wait_observed(child);
                    user_child_process_child_continuation_taken(child);
                    user_child_process_wait4_handoff_frame_diagnostic_bound(child);
                    user_child_process_parent_wait_frame_saved(child);
                    user_child_process_parent_wait_register_checkpoint_bound(child);
                    user_child_process_parent_wait_stack_window_checkpoint_bound(child);
                    user_child_process_task_mm_switched(child, KernelInitTask);
                    user_task_pending_plain_fork_child_consumed_on_wait_handoff(KernelInitTask, child);
                    user_address_space_fault_mapping_diagnostic_bound(UserAddressSpace);
                    syscall_wait4_child_exit_status_copyout_first_slice(self);
                    syscall_wait4_observed_child_reap_first_slice(self);
                    syscall_wait4_no_child_echild_first_slice(self);
                    syscall_wait4_blocking_sleep_deferred(self);
                    syscall_read_builtin_grandchild_pipe_block_handoff(self, child);
                    syscall_wait4_builtin_grandchild_completed_reap(self, child);
                    user_child_process_completed_record_reaped(child);
                    syscall_table_wait4_observed(self);
                }
            }

            on Action::Exit(
                current_flow: TaskFlow,
                runtime: UserAppRuntime
            ) {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::OnCpu;
                    KernelInitFlow.state == State::Online;
                    current_flow.state == State::Online;
                    runtime.state == State::Online;
                    task_fixed_flow_is(KernelInitTask, current_flow);
                    user_app_runtime_owned_by_flow(runtime, current_flow);
                    user_app_runtime_terminal_quiesced(runtime);
                }

                drives {
                    runtime.Transition::Disable;
                    runtime.Transition::Cleanup;
                    current_flow.Transition::Disable;
                    current_flow.Transition::Cleanup;
                    KernelInitTask.Transition::Disable;
                    KernelInitTask.Transition::Cleanup;
                }

                ensures {
                    syscall_exit_records_status(self);
                    runtime.state == State::Destroyed;
                    current_flow.state == State::Destroyed;
                    KernelInitTask.state == State::Destroyed;
                    task_fixed_flow_destroyed(KernelInitTask);
                    task_destroyed_only_after_flow_cleanup(KernelInitTask);
                    syscall_table_exit_observed(self);
                }
            }

            on Action::Sync {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    RootFS.state == State::Online;
                }

                ensures {
                    syscall_sync_rootfs_barrier_first_slice(self);
                }
            }

            on Action::RebootPowerOff {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                }

                ensures {
                    syscall_reboot_poweroff_magic_first_slice(self);
                }
            }

            on Action::ExitGroup(
                child: Task,
                current_flow: TaskFlow,
                runtime: UserAppRuntime
            ) {
                depends_on {
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    child.state == State::OnCpu;
                    current_flow.state == State::Online;
                    runtime.state == State::Online;
                    user_process_registry_contains(UserProcessRegistry, child);
                    task_fixed_flow_is(child, current_flow);
                    user_app_runtime_owned_by_flow(runtime, current_flow);
                    user_app_runtime_terminal_quiesced(runtime);
                }

                drives {
                    UserProcessRegistry.Action::PublishZombie(child);
                    runtime.Transition::Disable;
                    runtime.Transition::Cleanup;
                    current_flow.Transition::Disable;
                    current_flow.Transition::Cleanup;
                    child.Transition::Disable;
                    child.Transition::Cleanup;
                }

                ensures {
                    syscall_exit_records_status(self);
                    syscall_exit_group_pid1_shutdown_child_wait4_split(self);
                    user_child_process_exit_status_observed(child);
                    user_child_process_wait4_status_copied(child);
                    user_child_process_parent_wait_resumed(child);
                    user_child_process_parent_wait_resume_checkpoint_bound(child);
                    user_child_process_exit_mm_released_once(child);
                    user_child_process_parent_mm_resumed_without_byte_restore(child, KernelInitTask);
                    user_child_process_parent_fd_snapshot_restored(child, FilesStruct);
                    files_struct_parent_fd_snapshot_restored(FilesStruct, child);
                    user_child_process_completed_record_archived(child);
                    user_process_registry_exit_status_published_release(UserProcessRegistry, child);
                    user_process_registry_parent_wake_target_cpu_fixed(UserProcessRegistry, child);
                    runtime.state == State::Destroyed;
                    current_flow.state == State::Destroyed;
                    child.state == State::Destroyed;
                    task_fixed_flow_destroyed(child);
                    task_destroyed_only_after_flow_cleanup(child);
                    syscall_table_exit_observed(self);
                }
            }
        }
    }
}


/*
 * PID 1 occupies the registry's stable first aggregate. Its Task and stable
 * Flow-owned Runtime keep their identity across exec; exec replaces only the
 * aggregate contents and Runtime ApplicationInstance.
 */

object UserBootPayload: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PayloadParam.state == State::Ready;
                    BootParam.state == State::Ready;
                    RootFS.state == State::Online;
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    KernelInitTask.state == State::OnCpu;
                    task_execution_authority_is(
                        KernelInitTask,
                        TaskExecutionAuthority::Live
                    );
                    ExecSyncBoundaries.state == State::Ready;
                }

                ensures {
                    user_boot_payload_selected(self);
                    user_boot_payload_candidates_bound(self);
                    user_boot_payload_default_init_path_bound(self);
                    user_boot_payload_default_init_fallback_order_bound(self);
                    user_boot_payload_default_init_candidate_bound(self, UserInitPathRef::DefaultInit);
                    user_boot_payload_default_init_candidate_bound(self, UserInitPathRef::EtcInit);
                    user_boot_payload_default_init_candidate_bound(self, UserInitPathRef::BinInit);
                    user_boot_payload_default_init_candidate_bound(self, UserInitPathRef::BinSh);
                    user_boot_payload_requested_init_branch_bound(self);
                    user_boot_payload_requested_init_before_default_fallback(self);
                    user_boot_payload_requested_init_failure_panic_terminal_bound(self);
                    user_boot_payload_requested_init_no_default_fallback(self);
                    user_boot_payload_candidate_failure_nonfatal_for_fallback(self);
                    user_boot_payload_success_stops_fallback_chain(self);
                    user_boot_payload_success_no_return_to_startup_orchestration(self);
                    user_boot_payload_no_working_init_panic_terminal_bound(self);
                    user_boot_payload_uses_current_fs_struct(self, FsStruct);
                    user_boot_payload_no_partition_dependency(self);
                    user_boot_payload_partition_objects_deferred(self);
                    user_boot_payload_driven_by_kernel_init_task(self, KernelInitTask);
                    user_boot_payload_try_candidate_bound(self);
                    user_boot_payload_rc_local_direct_inittab_diagnostic_first_slice(self);
                    user_boot_payload_init_attempt_failure_trace_defined(self);
                    payload_image_read_failed_checkpoint_defined(self);
                    payload_image_read_error_classification_contract_ready(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            user_boot_payload_selected(self);
            user_boot_payload_candidates_bound(self);
            user_boot_payload_default_init_path_bound(self);
            user_boot_payload_default_init_fallback_order_bound(self);
            user_boot_payload_default_init_candidate_bound(self, UserInitPathRef::DefaultInit);
            user_boot_payload_default_init_candidate_bound(self, UserInitPathRef::EtcInit);
            user_boot_payload_default_init_candidate_bound(self, UserInitPathRef::BinInit);
            user_boot_payload_default_init_candidate_bound(self, UserInitPathRef::BinSh);
            user_boot_payload_requested_init_branch_bound(self);
            user_boot_payload_requested_init_before_default_fallback(self);
            user_boot_payload_requested_init_failure_panic_terminal_bound(self);
            user_boot_payload_requested_init_no_default_fallback(self);
            user_boot_payload_candidate_failure_nonfatal_for_fallback(self);
            user_boot_payload_success_stops_fallback_chain(self);
            user_boot_payload_success_no_return_to_startup_orchestration(self);
            user_boot_payload_no_working_init_panic_terminal_bound(self);
            user_boot_payload_uses_current_fs_struct(self, FsStruct);
            user_boot_payload_no_partition_dependency(self);
            user_boot_payload_partition_objects_deferred(self);
            user_boot_payload_driven_by_kernel_init_task(self, KernelInitTask);
            user_boot_payload_try_candidate_bound(self);
            user_boot_payload_rc_local_direct_inittab_diagnostic_first_slice(self);
            user_boot_payload_init_attempt_failure_trace_defined(self);
            payload_image_read_failed_checkpoint_defined(self);
            payload_image_read_error_classification_contract_ready(self);
            ExecSyncBoundaries.state == State::Ready;
        }

        actions {
            /*
             * Reversible precommit preparation. The lifetime Flow and Runtime
             * remain Online; the ApplicationInstance is not replaced here.
             */
            on Action::PrepareHandoff {
                depends_on {
                    KernelInitFlow.state == State::Online;
                    KernelInitTask.state == State::OnCpu;
                    CurrentCPU.trap.exception.state == State::Ready;
                    CurrentCPU.trap.exception.syscall.state == State::Prepared;
                    ExecSyncBoundaries.state == State::Ready;
                }

                drives {
                    Path.Transition::Setup;
                    VfsCore.Action::ReadPath(Path, FsStruct);
                    ElfObject.Transition::Preset;
                    ElfObject.Transition::Setup;
                    UserBootPayload.Action::TryDefaultInitSequence;
                    UserAddressSpace.Transition::Preset;
                    UserStack.Transition::Setup;
                    UserAddressSpace.Transition::Setup;
                    UserTrapFrame.Transition::Setup;
                    ElfObject.Transition::Enable;
                    UserAddressSpace.Transition::Enable;
                    CurrentCPU.trap.exception.syscall.Transition::Setup;
                    SyscallTable.Transition::Setup;
                    CurrentCPU.trap.exception.syscall.Transition::Enable;
                    CurrentCPU.trap.exception.Transition::Enable;
                    CurrentCPU.trap.Transition::Enable;
                    FilesStruct.Transition::Setup;
                    FilesStruct.Action::ClearStdinReadyData;
                    FilesStruct.Action::PrepareDefaultStdinReadyData;
                    FilesStruct.Action::EnableStdinBlockingWait;
                }

                ensures {
                    ElfObject.state == State::Online;
                    UserAddressSpace.state == State::Online;
                    UserTrapFrame.state == State::Ready;
                    SyscallTable.state == State::Ready;
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    CurrentCPU.trap.exception.state == State::Online;
                    CurrentCPU.trap.state == State::Online;
                    KernelInitUserAppRuntime.state == State::Online;
                    application_instance_fresh(KernelInitApplicationInstance);
                    task_fixed_flow_is(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_survives_payload_precommit(KernelInitFlow);
                }
            }

            on Action::TryCandidate(path: UserInitPathRef) {
                depends_on {
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    ElfObject.state == State::Ready;
                }

                ensures {
                    payload_image_read_start_checkpoint(self, path);
                    user_boot_payload_try_candidate_read_init(self, VfsCore);
                    payload_image_read_complete_checkpoint(self, VfsCore);
                    user_boot_payload_try_candidate_elf_ready(self, ElfObject);
                    user_boot_payload_selected_path_bound(self);
                }
            }

            /*
             * Linux 6.12 init/main.c::run_init_process() returns only when
             * kernel_execve() failed; try_to_run_init_process() reports
             * non-ENOENT default-candidate failures and continues to the next
             * fallback. The requested init= branch treats the same failed
             * kernel_execve()-equivalent result as terminal and panics instead
             * of falling through to the default list. The implementation must
             * expose a stable failure observation for each failed init attempt:
             * path kind, stable stage, stable reason and whether the failure is
             * terminal or fallback-nonfatal. The observation is a general trace
             * checkpoint, with KUnit as one possible consumer; it is not a
             * test-only API and must not change successful startup sequencing.
             */
            on Action::RecordInitAttemptFailure(
                path: UserInitPathRef,
                stage: UserInitAttemptStage,
                reason: UserInitAttemptReason
            ) {
                ensures {
                    user_boot_payload_init_attempt_failure_trace_defined(self);
                    user_boot_payload_init_attempt_failure_path_bound(self, path);
                    user_boot_payload_init_attempt_failure_stage_bound(self, stage);
                    user_boot_payload_init_attempt_failure_reason_bound(self, reason);
                    user_boot_init_attempt_failure_checkpoint(self);
                }
            }

            /*
             * Linux 6.12 requested init= branch. init_setup() stores
             * execute_command; kernel_init() tries it before CONFIG_DEFAULT_INIT
             * and before the default fallback list. A successful
             * kernel_execve()-equivalent result stops startup orchestration.
             * A failed requested init panics as "Requested init ... failed" and
             * does not continue into /sbin/init, /etc/init, /bin/init or
             * /bin/sh. The current implementation only supports a non-empty
             * absolute requested path that fits its fixed selected-path buffer;
             * full argv_init/envp_init derivation remains outside this slice.
             */
            on Action::TryRequestedInit(path: Path) {
                depends_on {
                    BootParam.state == State::Ready;
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                }

                ensures {
                    user_boot_payload_requested_init_branch_bound(self);
                    user_boot_payload_requested_init_before_default_fallback(self);
                    user_boot_payload_requested_init_failure_panic_terminal_bound(self);
                    user_boot_payload_requested_init_no_default_fallback(self);
                    user_boot_payload_init_attempt_failure_requested_terminal(self);
                    user_boot_payload_success_stops_fallback_chain(self);
                    user_boot_payload_success_no_return_to_startup_orchestration(self);
                    user_boot_payload_selected_path_bound(self);
                    user_boot_payload_selected_argv0_path_bound(self);
                }
            }

            /*
             * Linux 6.12 default fallback list selection. This action models
             * the ordered try_to_run_init_process() chain in init/main.c after
             * init= is absent and CONFIG_DEFAULT_INIT is skipped because it is
             * empty in the current configuration.
             * Candidate failures are nonfatal while later candidates remain;
             * a successful kernel_execve()-equivalent result stops the
             * fallback chain even though Linux returns integer 0 to
             * kernel_init(), because the task now represents the new init
             * image and does not resume the old startup orchestration path.
             * if every candidate fails the terminal behavior is the Linux
             * "No working init found" panic boundary.
             */
            on Action::TryDefaultInitSequence {
                depends_on {
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                }

                ensures {
                    user_boot_payload_default_init_fallback_order_bound(self);
                    user_boot_payload_candidate_failure_nonfatal_for_fallback(self);
                    user_boot_payload_init_attempt_failure_default_nonfatal(self);
                    user_boot_payload_first_successful_candidate_selected(self);
                    user_boot_payload_success_stops_fallback_chain(self);
                    user_boot_payload_success_no_return_to_startup_orchestration(self);
                    user_boot_payload_selected_path_bound(self);
                    user_boot_payload_selected_argv0_path_bound(self);
                    user_boot_payload_no_working_init_panic_terminal_bound(self);
                }
            }
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    PayloadParam.state == State::Ready;
                    BootParam.state == State::Ready;
                    RootFS.state == State::Online;
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    KernelInitTask.state == State::OnCpu;
                    CurrentCPU.trap.exception.state == State::Online;
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    CurrentCPU.trap.state == State::Online;
                    ExecSyncBoundaries.state == State::Ready;
                    KernelInitFlow.state == State::Online;
                    KernelInitUserAppRuntime.state == State::Online;
                }

                drives {
                    KernelInitUserAppRuntime.Action::ReplaceApplication(
                        next: KernelInitApplicationInstance
                    );
                }

                ensures {
                    ElfObject.state == State::Online;
                    UserAddressSpace.state == State::Online;
                    UserTrapFrame.state == State::Ready;
                    SyscallTable.state == State::Ready;
                    CurrentCPU.trap.exception.syscall.state == State::Online;
                    KernelInitTask.state == State::OnCpu;
                    task_execution_authority_is(
                        KernelInitTask,
                        TaskExecutionAuthority::Live
                    );
                    KernelInitUserAppRuntime.state == State::Online;
                    KernelInitFlow.state == State::Online;
                    task_fixed_flow_is(KernelInitTask, KernelInitFlow);
                    task_flow_owner_is(KernelInitFlow, KernelInitTask);
                    user_app_runtime_owned_by_flow(KernelInitUserAppRuntime, KernelInitFlow);
                    user_app_runtime_identity_preserved(KernelInitUserAppRuntime);
                    user_app_runtime_application_replaced(
                        KernelInitUserAppRuntime,
                        KernelInitApplicationInstance
                    );
                    task_fixed_flow_binding_consistent(KernelInitTask);
                    kernel_init_task_execve_to_pid1_user_app(
                        KernelInitTask,
                        KernelInitApplicationInstance
                    );
                    kernel_init_task_pid1_identity_preserved(KernelInitTask);
                    kernel_init_task_user_app_flow_online(KernelInitTask);
                    kernel_init_task_pid1_exec_flow_handoff_complete(KernelInitTask);
                    user_task_address_space_bound(KernelInitTask, UserAddressSpace);
                    user_task_files_struct_inherited(KernelInitTask, FilesStruct);
                    user_task_trap_frame_bound(KernelInitTask, UserTrapFrame);
                    user_task_credentials_inherited(KernelInitTask, KernelInitTask);
                    user_task_syscall_context_bound(
                        KernelInitTask,
                        CurrentCPU.trap.exception.syscall,
                        SyscallTable
                    );
                    user_boot_payload_reads_init_from_vfs(self, VfsCore);
                    user_boot_payload_selected_path_bound(self);
                    user_boot_payload_selected_argv0_path_bound(self);
                    user_boot_payload_stdin_probe_ready_data_cleared(self);
                    files_struct_stdin_probe_ready_data_cleared(FilesStruct);
                    user_boot_payload_user_smoke_stdin_fixture_only(self);
                    user_boot_payload_distro_init_no_stdin_fixture(self);
                    user_boot_payload_distro_stdin_blocking_wait_enabled(self);
                    user_boot_payload_user_smoke_stdin_blocking_wait_opt_out(self);
                    user_boot_payload_enters_user_mode(self);
                    user_boot_payload_no_return_handoff(self);
                    selected_payload_no_return_handoff();
                }
            }
        }
    }

    state State::Online {
        invariant {
            user_boot_payload_selected(self);
            KernelInitTask.state == State::OnCpu;
            task_execution_authority_is(
                KernelInitTask,
                TaskExecutionAuthority::Live
            );
            KernelInitFlow.state == State::Online;
            task_fixed_flow_is(KernelInitTask, KernelInitFlow);
            task_fixed_flow_binding_consistent(KernelInitTask);
            KernelInitUserAppRuntime.state == State::Online;
            user_app_runtime_owned_by_flow(KernelInitUserAppRuntime, KernelInitFlow);
            kernel_init_task_pid1_identity_preserved(KernelInitTask);
            kernel_init_task_user_app_flow_online(KernelInitTask);
            kernel_init_task_pid1_exec_flow_handoff_complete(KernelInitTask);
            user_boot_payload_reads_init_from_vfs(self, VfsCore);
            user_boot_payload_selected_path_bound(self);
            user_boot_payload_selected_argv0_path_bound(self);
            user_boot_payload_driven_by_kernel_init_task(self, KernelInitTask);
            user_boot_payload_enters_user_mode(self);
            user_boot_payload_no_return_handoff(self);
        }
    }
}
