/*
 * First user-mode program bootstrap model.
 *
 * This slice models the shortest Linux-like path from PayloadPhase to the
 * first user-mode program. The selected payload variant is UserBootPayload.
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
 * SyscallException branch of ExceptionStream. SyscallException owns syscall
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
     * PayloadExecSyncBoundaries.
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

enum ElfObjectRole {
    MainExecutable,
    Interpreter,
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

predicate payload_exec_sync_boundaries_ready<T>(boundaries: T) -> bool;
predicate payload_kernel_execve_linux_window_bound<T>(boundaries: T) -> bool;
predicate payload_binfmt_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_cred_guard_mutex_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_update_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_mmap_local_irq_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_task_siglock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_tasklist_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_fs_lock_rcu_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_mmap_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_membarrier_deferred<T>(boundaries: T) -> bool;
predicate payload_bprm_mm_init_task_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_mmap_task_lock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_sched_mm_cid_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_files_unshare_cloexec_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_io_uring_cancel_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_posix_timer_siglock_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_namespace_switch_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_success_accounting_hooks_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_full_binfmt_deferred<T>(boundaries: T) -> bool;
predicate payload_binfmt_module_retry_trimmed_noop<T>(boundaries: T) -> bool;
predicate payload_binfmt_module_retry_trimmed_because_modules_disabled<T>(boundaries: T) -> bool;
predicate payload_ramdisk_init_branch_trimmed_noop<T>(boundaries: T) -> bool;
predicate payload_ramdisk_init_trimmed_because_config_initrd_disabled<T>(boundaries: T) -> bool;
predicate payload_default_init_branch_trimmed_noop<T>(boundaries: T) -> bool;
predicate payload_default_init_trimmed_because_config_default_init_empty<T>(boundaries: T) -> bool;
predicate payload_binfmt_script_deferred<T>(boundaries: T) -> bool;
predicate payload_exec_panic_terminal_bound<T>(boundaries: T) -> bool;

predicate user_clone_deferred_boundaries_ready<T>(boundaries: T) -> bool;
predicate user_clone_linux_6_12_legacy_clone_bound<T>(boundaries: T) -> bool;
predicate user_clone_riscv_abi_argument_order_bound<T>(boundaries: T) -> bool;
predicate user_clone_observed_plain_fork_args_bound<T>(boundaries: T) -> bool;
predicate user_clone_observed_vfork_vm_args_bound<T>(boundaries: T) -> bool;
predicate user_clone_observed_vfork_pidfd_args_bound<T>(boundaries: T) -> bool;
predicate user_clone_plain_fork_first_slice_bound<T>(boundaries: T) -> bool;
predicate user_clone_vfork_vm_first_slice_bound<T>(boundaries: T) -> bool;
predicate user_clone_vfork_pidfd_first_slice_bound<T>(boundaries: T) -> bool;
predicate user_clone_bounded_sequential_vfork_records_bound<T>(boundaries: T) -> bool;
predicate user_clone_single_active_child_slot_bound<T>(boundaries: T) -> bool;
predicate user_clone_completed_child_record_capacity_bound<T>(boundaries: T) -> bool;
predicate user_clone_csignal_split_bound<T>(boundaries: T) -> bool;
predicate user_clone_sigchld_exit_signal_bound<T>(boundaries: T) -> bool;
predicate user_clone_newsp_zero_inherits_parent_sp<T>(boundaries: T) -> bool;
predicate user_clone_newsp_sets_child_sp<T>(boundaries: T) -> bool;
predicate user_clone_legacy_pidfd_uses_parent_tidptr<T>(boundaries: T) -> bool;
predicate user_clone_tls_ignored_without_clone_settls<T>(boundaries: T) -> bool;
predicate user_clone_thread_group_deferred<T>(boundaries: T) -> bool;
predicate user_clone_full_clone_vm_vfork_deferred<T>(boundaries: T) -> bool;
predicate user_clone_cow_mm_deferred<T>(boundaries: T) -> bool;
predicate user_clone_full_pidfd_file_ops_deferred<T>(boundaries: T) -> bool;
predicate user_clone_ptrace_seccomp_cgroup_audit_deferred<T>(boundaries: T) -> bool;
predicate user_clone_namespace_deferred<T>(boundaries: T) -> bool;
predicate user_clone_robust_futex_deferred<T>(boundaries: T) -> bool;
predicate user_clone_clear_child_futex_deferred<T>(boundaries: T) -> bool;
predicate user_clone_wait_exit_reap_deferred<T>(boundaries: T) -> bool;
predicate user_clone_unsupported_flags_first_slice<T>(boundaries: T) -> bool;

predicate elf_object_input_bound<T>(elf: T) -> bool;
predicate elf_object_input_from_vfs<T, V>(elf: T, vfs: V) -> bool;
predicate elf_object_magic_valid<T>(elf: T) -> bool;
predicate elf_object_class_elf64<T>(elf: T) -> bool;
predicate elf_object_little_endian<T>(elf: T) -> bool;
predicate elf_object_machine_riscv<T>(elf: T) -> bool;
predicate elf_object_type_supported<T>(elf: T) -> bool;
predicate elf_object_static_executable<T>(elf: T) -> bool;
predicate elf_object_role_bound<T, R>(elf: T, role: R) -> bool;
predicate elf_object_dynamic_executable<T>(elf: T) -> bool;
predicate elf_object_et_dyn_pie_main_supported<T>(elf: T) -> bool;
predicate elf_object_main_pie_load_bias_bound<T>(elf: T) -> bool;
predicate elf_object_interpreter_required<T>(elf: T) -> bool;
predicate elf_object_interpreter_path_bound<T>(elf: T) -> bool;
predicate elf_object_interpreter_elf_bound<T, I>(elf: T, interpreter: I) -> bool;
predicate elf_object_et_dyn_interpreter_supported<T>(elf: T) -> bool;
predicate elf_object_et_dyn_loader_without_interp_deferred<T>(elf: T) -> bool;
predicate elf_object_runtime_entry_bound<T>(elf: T) -> bool;
predicate elf_object_auxv_exec_fields_bound<T>(elf: T) -> bool;
predicate elf_object_program_headers_parsed<T>(elf: T) -> bool;
predicate elf_object_pt_load_segments_bound<T>(elf: T) -> bool;
predicate elf_object_segment_permissions_bound<T>(elf: T) -> bool;
predicate elf_object_load_plan_bound<T>(elf: T) -> bool;
predicate elf_object_entry_in_executable_segment<T>(elf: T) -> bool;
predicate elf_object_init_content_observed<T>(elf: T) -> bool;
predicate elf_object_bss_zero_plan_bound<T>(elf: T) -> bool;
predicate elf_object_bss_zeroed<T>(elf: T) -> bool;
predicate elf_object_entry_bound<T>(elf: T) -> bool;
predicate elf_object_mapped_to_user_address_space<T, A>(elf: T, space: A) -> bool;
predicate elf_object_no_separate_loader<T>(elf: T) -> bool;
predicate elf_object_load_merged_into_setup<T>(elf: T) -> bool;
predicate elf_object_user_entry_ready<T>(elf: T) -> bool;

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
predicate swapper_vm_remains_kernel_shared_instance<T>(swapper: T) -> bool;

predicate user_stack_allocated<T>(stack: T) -> bool;
predicate user_stack_fixed_size_bound<T>(stack: T) -> bool;
predicate user_stack_mapped_into_address_space<T, A>(stack: T, space: A) -> bool;
predicate user_stack_backing_pages_allocated<T>(stack: T) -> bool;
predicate user_stack_zeroed<T>(stack: T) -> bool;
predicate user_stack_initial_sp_bound<T>(stack: T) -> bool;
predicate user_stack_minimal_arg_env_bound<T>(stack: T) -> bool;
predicate user_stack_initial_argc_argv_envp_auxv_bound<T>(stack: T) -> bool;
predicate user_stack_static_libc_entry_supported<T>(stack: T) -> bool;
predicate user_stack_dynamic_auxv_fields_bound<T, E, I>(stack: T, elf: E, interpreter: I) -> bool;

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
predicate syscall_table_fcntl_supported<T>(table: T) -> bool;
predicate syscall_table_ioctl_supported<T>(table: T) -> bool;
predicate syscall_table_getrandom_supported<T>(table: T) -> bool;
predicate syscall_table_getuid_supported<T>(table: T) -> bool;
predicate syscall_table_getgid_supported<T>(table: T) -> bool;
predicate syscall_table_getpgid_supported<T>(table: T) -> bool;
predicate syscall_table_setpgid_supported<T>(table: T) -> bool;
predicate syscall_table_setsid_supported<T>(table: T) -> bool;
predicate syscall_table_setuid_supported<T>(table: T) -> bool;
predicate syscall_table_setgid_supported<T>(table: T) -> bool;
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
predicate syscall_getrandom_usercopy_ready<T>(table: T) -> bool;
predicate syscall_signal_mask_usercopy_ready<T>(table: T) -> bool;
predicate syscall_signal_action_usercopy_ready<T>(table: T) -> bool;
predicate syscall_time_usercopy_ready<T>(table: T) -> bool;
predicate syscall_path_usercopy_ready<T>(table: T) -> bool;
predicate syscall_stat_usercopy_ready<T>(table: T) -> bool;
predicate syscall_write_routes_to_console<T>(table: T) -> bool;
predicate syscall_writev_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_openat_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_chdir_routes_to_fs_struct<T, F>(table: T, fs: F) -> bool;
predicate syscall_chdir_linux_6_12_path_walk_bound<T>(table: T) -> bool;
predicate syscall_chdir_root_first_slice<T>(table: T) -> bool;
predicate syscall_chdir_permissions_lsm_deferred<T>(table: T) -> bool;
predicate syscall_read_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_read_stdin_ready_data_first_slice<T>(table: T) -> bool;
predicate syscall_read_no_ready_blocking_out_of_slice<T>(table: T) -> bool;
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
predicate syscall_fcntl_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_ioctl_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_ioctl_usercopy_ready<T>(table: T) -> bool;
predicate syscall_ioctl_tcgets_termios_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tcsets_termios_mutation_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tiocgpgrp_foreground_pgrp_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tiocspgrp_foreground_pgrp_update_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tty_full_linux_model_deferred<T>(table: T) -> bool;
predicate syscall_getrandom_routes_to_hwrng_core<T, H>(table: T, hwrng: H) -> bool;
predicate syscall_getrandom_not_vfs_or_devfs_path<T>(table: T) -> bool;
predicate syscall_getrandom_flags_first_slice_bound<T>(table: T) -> bool;
predicate syscall_getrandom_full_random_core_deferred<T>(table: T) -> bool;
predicate syscall_getuid_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
predicate syscall_getgid_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
predicate syscall_getpgid_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
predicate syscall_setpgid_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
predicate syscall_setpgid_child_plain_fork_first_slice<T, P>(table: T, process: P) -> bool;
predicate syscall_setsid_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
predicate syscall_setsid_process_group_leader_eperm_first_slice<T>(table: T) -> bool;
predicate syscall_setuid_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
predicate syscall_setgid_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
predicate syscall_credentials_full_linux_model_deferred<T>(table: T) -> bool;
predicate syscall_rt_sigprocmask_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
predicate syscall_rt_sigprocmask_sigsetsize_bound<T>(table: T) -> bool;
predicate syscall_rt_sigprocmask_unblockable_signals_cleared<T>(table: T) -> bool;
predicate syscall_rt_sigaction_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
predicate syscall_rt_sigaction_routes_to_signal_action_table<T, P>(table: T, process: P) -> bool;
predicate syscall_rt_sigaction_sigsetsize_bound<T>(table: T) -> bool;
predicate syscall_rt_sigaction_layout_bound<T>(table: T) -> bool;
predicate syscall_rt_sigaction_unblockable_signals_cleared<T>(table: T) -> bool;
predicate syscall_rt_sigaction_kernel_only_signals_rejected<T>(table: T) -> bool;
predicate syscall_rt_sigtimedwait_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
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
predicate syscall_set_tid_address_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
predicate syscall_clone_routes_to_task_creation_core<T, C>(table: T, core: C) -> bool;
predicate syscall_clone_routes_to_user_clone_deferred_boundaries<T, B>(table: T, boundaries: B) -> bool;
predicate syscall_clone_legacy_args_decoded<T>(table: T) -> bool;
predicate syscall_clone_plain_fork_first_slice<T>(table: T) -> bool;
predicate syscall_clone_vfork_vm_first_slice<T>(table: T) -> bool;
predicate syscall_clone_vfork_pidfd_first_slice<T>(table: T) -> bool;
predicate syscall_clone_parent_returns_child_pid<T, P>(table: T, process: P) -> bool;
predicate syscall_clone_child_return_zero_bound<T, P>(table: T, process: P) -> bool;
predicate syscall_clone_pidfd_copyout_bound<T, F>(table: T, files: F) -> bool;
predicate syscall_clone_vfork_parent_frame_saved<T, C>(table: T, child: C) -> bool;
predicate syscall_clone_vfork_child_handoff<T, C>(table: T, child: C) -> bool;
predicate syscall_clone_vfork_next_child_accepted<T, C>(table: T, child: C) -> bool;
predicate syscall_clone_wake_up_new_task_shape<T, S>(table: T, scheduler: S) -> bool;
predicate syscall_execve_linux_6_12_do_execveat_common_bound<T>(table: T) -> bool;
predicate syscall_execve_observed_shell_ls_args_bound<T>(table: T) -> bool;
predicate syscall_execve_observed_openrc_getty_args_bound<T>(table: T) -> bool;
predicate syscall_execve_child_continuation_first_slice<T, C>(table: T, child: C) -> bool;
predicate syscall_execve_reuses_user_boot_payload_elf_loader<T, P>(table: T, payload: P) -> bool;
predicate syscall_execve_replaces_user_address_space_first_slice<T, A>(table: T, space: A) -> bool;
predicate syscall_execve_context_staging_address_space_bound<T, A>(table: T, space: A) -> bool;
predicate syscall_execve_sets_start_thread_frame_first_slice<T, R>(table: T, frame: R) -> bool;
predicate syscall_execve_argv0_first_slice<T>(table: T) -> bool;
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
predicate syscall_wait4_options_wuntraced_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_wnohang_no_waitable_child_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_completed_child_record_reap_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_parent_wait_chldexit_boundary<T>(table: T) -> bool;
predicate syscall_wait4_yields_to_user_child_continuation<T, P>(table: T, process: P) -> bool;
predicate syscall_wait4_child_exit_status_copyout_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_observed_child_reap_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_no_child_echild_first_slice<T>(table: T) -> bool;
predicate syscall_wait4_blocking_sleep_deferred<T>(table: T) -> bool;
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
predicate user_kernel_trap_stack_entry_scratch_deferred<T>(frame: T) -> bool;
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
predicate syscall_table_fcntl_observed<T>(table: T) -> bool;
predicate syscall_table_ioctl_observed<T>(table: T) -> bool;
predicate syscall_table_getrandom_observed<T>(table: T) -> bool;
predicate syscall_table_getuid_observed<T>(table: T) -> bool;
predicate syscall_table_getgid_observed<T>(table: T) -> bool;
predicate syscall_table_getpgid_observed<T>(table: T) -> bool;
predicate syscall_table_setpgid_observed<T>(table: T) -> bool;
predicate syscall_table_setsid_observed<T>(table: T) -> bool;
predicate syscall_table_setuid_observed<T>(table: T) -> bool;
predicate syscall_table_setgid_observed<T>(table: T) -> bool;
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
predicate user_init_process_enter_user_mode_observed<T, R>(process: T, frame: R) -> bool;

predicate user_init_process_online<T>(process: T) -> bool;
predicate user_init_process_reuses_kernel_init_task<T, K>(process: T, task: K) -> bool;
predicate user_init_process_pid1_preserved<T, K>(process: T, task: K) -> bool;
predicate user_init_process_exec_identity_handoff<T, K>(process: T, task: K) -> bool;
predicate user_init_process_no_new_task_struct<T>(process: T) -> bool;
predicate user_init_process_kernel_init_not_destroyed<T, K>(process: T, task: K) -> bool;
predicate user_init_process_path_bound<T, P>(process: T, path: P) -> bool;
predicate user_init_process_address_space_bound<T, A>(process: T, space: A) -> bool;
predicate user_init_process_fs_struct_inherited<T, F>(process: T, fs: F) -> bool;
predicate user_init_process_files_struct_inherited<T, F>(process: T, files: F) -> bool;
predicate user_init_process_trap_frame_bound<T, R>(process: T, frame: R) -> bool;
predicate user_init_process_syscall_context_bound<T, E, S>(process: T, exception: E, table: S) -> bool;
predicate user_init_process_credentials_inherited<T, K>(process: T, task: K) -> bool;
predicate user_init_process_root_credentials_bound<T>(process: T) -> bool;
predicate user_init_process_credentials_capability_model_deferred<T>(process: T) -> bool;
predicate user_init_process_signal_state_inherited<T, K>(process: T, task: K) -> bool;
predicate user_init_process_signal_runtime_bound<T>(process: T) -> bool;
predicate user_init_process_thread_signal_state_bound<T>(process: T) -> bool;
predicate user_init_process_process_signal_state_deferred<T>(process: T) -> bool;
predicate user_init_process_signal_action_table_bound<T>(process: T) -> bool;
predicate user_init_process_signal_action_table_layout_bound<T>(process: T) -> bool;
predicate user_init_process_blocked_signal_mask_bound<T>(process: T) -> bool;
predicate user_init_process_pending_signal_set_empty_first_slice<T>(process: T) -> bool;
predicate user_init_process_signal_delivery_deferred<T>(process: T) -> bool;
predicate user_init_process_clear_child_tid_bound<T>(process: T) -> bool;
predicate user_init_process_session_leader_first_slice<T>(process: T) -> bool;
predicate user_init_process_process_group_leader_first_slice<T>(process: T) -> bool;
predicate user_init_process_process_group_read_observed<T>(process: T) -> bool;
predicate user_init_process_process_group_set_observed<T>(process: T) -> bool;
predicate user_init_process_setsid_eperm_observed<T>(process: T) -> bool;
predicate user_init_process_child_process_group_visible<T, C>(process: T, child: C) -> bool;
predicate user_init_process_child_process_group_set_observed<T, C>(process: T, child: C) -> bool;
predicate user_init_process_controlling_tty_bound<T>(process: T) -> bool;
predicate user_init_process_foreground_pgrp_bound<T>(process: T) -> bool;
predicate user_init_process_foreground_pgrp_read_observed<T>(process: T) -> bool;
predicate user_init_process_foreground_pgrp_set_observed<T>(process: T) -> bool;
predicate user_init_process_foreground_pgrp_accepts_child_pgrp_first_slice<T, C>(process: T, child: C) -> bool;
predicate user_child_process_prepared<T>(process: T) -> bool;
predicate user_child_process_parent_pid1<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_pid_allocated<T, N>(process: T, pid_ns: N) -> bool;
predicate user_child_process_tgid_equals_pid<T>(process: T) -> bool;
predicate user_child_process_process_group_visible_to_parent<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_exit_signal_sigchld<T>(process: T) -> bool;
predicate user_child_process_files_struct_copied<T, F>(process: T, files: F) -> bool;
predicate user_child_process_fs_struct_copied<T, F>(process: T, fs: F) -> bool;
predicate user_child_process_parent_fd_snapshot_saved<T, F>(process: T, files: F) -> bool;
predicate user_child_process_parent_fd_snapshot_restored<T, F>(process: T, files: F) -> bool;
predicate user_child_process_credentials_copied<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_signal_state_copied<T, P>(process: T, parent: P) -> bool;
predicate user_child_process_user_address_space_snapshot<T, A>(process: T, space: A) -> bool;
predicate user_child_process_user_stack_snapshot_copied<T, A>(process: T, space: A) -> bool;
predicate user_child_process_user_stack_snapshot_restored<T, A>(process: T, space: A) -> bool;
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
predicate user_child_process_pidfd_copyout_observed<T>(process: T) -> bool;
predicate user_child_process_single_active_slot<T>(process: T) -> bool;
predicate user_child_process_completed_records_capacity_bound<T>(process: T) -> bool;
predicate user_child_process_next_child_pid_bound<T>(process: T) -> bool;
predicate user_child_process_completed_record_archived<T>(process: T) -> bool;
predicate user_child_process_completed_record_unreaped<T>(process: T) -> bool;
predicate user_child_process_completed_record_reaped<T>(process: T) -> bool;
predicate user_child_process_completed_record_released<T>(process: T) -> bool;
predicate user_child_process_active_slot_reusable<T>(process: T) -> bool;
predicate user_child_process_vfork_next_child_accepted<T>(process: T) -> bool;
predicate user_child_process_wait4_handoff_frame_diagnostic_bound<T>(process: T) -> bool;
predicate user_child_process_parent_wait_frame_saved<T>(process: T) -> bool;
predicate user_child_process_parent_address_space_snapshot_saved<T, A>(process: T, space: A) -> bool;
predicate user_child_process_parent_wait_register_checkpoint_bound<T>(process: T) -> bool;
predicate user_child_process_parent_wait_stack_window_checkpoint_bound<T>(process: T) -> bool;
predicate user_child_process_parent_wait_stack_window_compared<T>(process: T) -> bool;
predicate user_child_process_parent_wait_stack_snapshot_copied<T, A>(process: T, space: A) -> bool;
predicate user_child_process_parent_wait_stack_snapshot_restored<T, A>(process: T, space: A) -> bool;
predicate user_child_process_parent_wait_writable_page_snapshot_copied<T, A>(process: T, space: A) -> bool;
predicate user_child_process_parent_wait_writable_page_snapshot_compared<T, A>(process: T, space: A) -> bool;
predicate user_child_process_parent_wait_writable_page_snapshot_restored<T, A>(process: T, space: A) -> bool;
predicate user_child_process_exit_status_observed<T>(process: T) -> bool;
predicate user_child_process_wait4_status_copied<T>(process: T) -> bool;
predicate user_child_process_parent_wait_resumed<T>(process: T) -> bool;
predicate user_child_process_parent_wait_resume_checkpoint_bound<T>(process: T) -> bool;
predicate user_address_space_fault_mapping_diagnostic_bound<T>(space: T) -> bool;
predicate user_init_process_uid_read_observed<T>(process: T) -> bool;
predicate user_init_process_gid_read_observed<T>(process: T) -> bool;
predicate user_init_process_uid_set_observed<T>(process: T) -> bool;
predicate user_init_process_gid_set_observed<T>(process: T) -> bool;
predicate user_init_process_rt_sigprocmask_observed<T>(process: T) -> bool;
predicate user_init_process_rt_sigaction_observed<T>(process: T) -> bool;
predicate user_init_process_rt_sigtimedwait_observed<T>(process: T) -> bool;
predicate user_init_process_rt_sigtimedwait_mask_observed<T>(process: T) -> bool;
predicate user_init_process_rt_sigtimedwait_uinfo_null<T>(process: T) -> bool;
predicate user_init_process_rt_sigtimedwait_uts_null<T>(process: T) -> bool;
predicate user_init_process_rt_sigtimedwait_pending_match_empty<T>(process: T) -> bool;
predicate user_init_process_rt_sigtimedwait_infinite_wait<T>(process: T) -> bool;
predicate user_init_process_pending_sigchld_first_slice<T>(process: T) -> bool;
predicate user_init_process_rt_sigtimedwait_waiter_enqueued<T>(process: T) -> bool;
predicate user_init_process_rt_sigtimedwait_sleep_reason_bound<T>(process: T) -> bool;
predicate user_init_process_rt_sigtimedwait_woken_by_sigchld<T>(process: T) -> bool;
predicate user_init_process_rt_sigtimedwait_dequeued_sigchld<T>(process: T) -> bool;
predicate user_init_process_user_entry_ready<T>(process: T) -> bool;
predicate user_init_process_trap_return_bound<T, R>(process: T, frame: R) -> bool;
predicate kernel_init_task_execve_to_user_init<K, T>(task: K, process: T) -> bool;
predicate kernel_init_task_pid1_identity_preserved<K>(task: K) -> bool;
predicate kernel_init_task_user_mm_attached<K, A>(task: K, space: A) -> bool;
predicate kernel_init_task_user_trap_frame_attached<K, R>(task: K, frame: R) -> bool;

predicate files_struct_stdin_ready_data_bound<T>(files: T) -> bool;
predicate files_struct_stdin_char_device_read_observed<T>(files: T) -> bool;
predicate files_struct_tty_termios_state_bound<T>(files: T) -> bool;
predicate files_struct_tty_termios_mutation_observed<T>(files: T) -> bool;
predicate files_struct_open_path_routes_to_vfs<T, V>(files: T, vfs: V) -> bool;
predicate files_struct_regular_fd_installed<T>(files: T) -> bool;
predicate files_struct_directory_fd_installed<T>(files: T) -> bool;
predicate file_backend_char_device_read_returns_ready_data<T>(backend: T) -> bool;
predicate tty_flip_buffer_ready_data_bound<T>(buffer: T) -> bool;
predicate tty_flip_buffer_ready_data_consumed<T>(buffer: T) -> bool;
predicate tty_n_tty_blocking_read_deferred<T>(buffer: T) -> bool;

context UserModeTrapReturnContext: Context {
    /*
     * UserInitProcess.EnterUserMode is the final RISC-V trap-return handoff.
     * The body writes sscratch/sepc/sstatus/satp, executes sfence.vma after
     * the satp write, switches to the user stack and executes sret. It has no
     * normal runtime exit because sret transfers to U-mode.
     */
    guard {
        entered_by {
            UserInitProcess.Action::EnterUserMode;
        }

        exited_by {
            Never;
        }
    }

    obj_refs {
        UserInitProcess;
        UserAddressSpace;
        UserTrapFrame;
    }
}

object PayloadExecSyncBoundaries: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PayloadParam.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    SystemState.state == State::Online;
                    system_state_running(SystemState);
                }

                ensures {
                    payload_exec_sync_boundaries_ready(self);
                    payload_kernel_execve_linux_window_bound(self);
                    payload_binfmt_lock_deferred(self);
                    payload_cred_guard_mutex_deferred(self);
                    payload_exec_update_lock_deferred(self);
                    payload_exec_mmap_local_irq_deferred(self);
                    payload_exec_task_siglock_deferred(self);
                    payload_exec_tasklist_lock_deferred(self);
                    payload_exec_fs_lock_rcu_deferred(self);
                    payload_exec_mmap_lock_deferred(self);
                    payload_exec_membarrier_deferred(self);
                    payload_bprm_mm_init_task_lock_deferred(self);
                    payload_exec_mmap_task_lock_deferred(self);
                    payload_exec_sched_mm_cid_deferred(self);
                    payload_exec_files_unshare_cloexec_deferred(self);
                    payload_exec_io_uring_cancel_deferred(self);
                    payload_exec_posix_timer_siglock_deferred(self);
                    payload_exec_namespace_switch_deferred(self);
                    payload_exec_success_accounting_hooks_deferred(self);
                    payload_exec_full_binfmt_deferred(self);
                    payload_binfmt_module_retry_trimmed_noop(self);
                    payload_binfmt_module_retry_trimmed_because_modules_disabled(self);
                    payload_ramdisk_init_branch_trimmed_noop(self);
                    payload_ramdisk_init_trimmed_because_config_initrd_disabled(self);
                    payload_default_init_branch_trimmed_noop(self);
                    payload_default_init_trimmed_because_config_default_init_empty(self);
                    payload_binfmt_script_deferred(self);
                    payload_exec_panic_terminal_bound(self);
                }

                deferred {
                    "Linux 6.12 kernel_execve()/bprm_execve()/exec_binprm()/begin_new_exec() 的完整同步协议保留为 PayloadExecSyncBoundaries：binfmt_lock、cred_guard_mutex、exec_update_lock、exec_mmap() 本地 IRQ 关闭与 mmap_lock、exec_mmap() task_lock(mm handoff)、siglock/tasklist_lock、fs->lock+RCU、membarrier、sched_mm_cid rq_lock_irqsave+smp_mb、bprm_mm_init/finalize_exec task_lock rlimit 边界、files unshare/CLOEXEC file_lock、io_uring cancel、POSIX timer siglock、namespace switch、exec 成功后的 rseq/perf/audit/accounting hooks、完整 binfmt/script retry 和失败后 panic terminal 后续展开；当前 UserBootPayload 只实现最小 VFS/ELF/UserAddressSpace/trap-return handoff。当前 ../linux-6.12/.config 中 CONFIG_MODULES=n，因此 request_module(\"binfmt-...\") retry 记录为 trimmed/no-op。";
                }
            }
        }
    }

    state State::Ready {
        invariant {
            payload_exec_sync_boundaries_ready(self);
            payload_kernel_execve_linux_window_bound(self);
            payload_binfmt_lock_deferred(self);
            payload_cred_guard_mutex_deferred(self);
            payload_exec_update_lock_deferred(self);
            payload_exec_mmap_local_irq_deferred(self);
            payload_exec_task_siglock_deferred(self);
            payload_exec_tasklist_lock_deferred(self);
            payload_exec_fs_lock_rcu_deferred(self);
            payload_exec_mmap_lock_deferred(self);
            payload_exec_membarrier_deferred(self);
            payload_bprm_mm_init_task_lock_deferred(self);
            payload_exec_mmap_task_lock_deferred(self);
            payload_exec_sched_mm_cid_deferred(self);
            payload_exec_files_unshare_cloexec_deferred(self);
            payload_exec_io_uring_cancel_deferred(self);
            payload_exec_posix_timer_siglock_deferred(self);
            payload_exec_namespace_switch_deferred(self);
            payload_exec_success_accounting_hooks_deferred(self);
            payload_exec_full_binfmt_deferred(self);
            payload_binfmt_module_retry_trimmed_noop(self);
            payload_binfmt_module_retry_trimmed_because_modules_disabled(self);
            payload_ramdisk_init_branch_trimmed_noop(self);
            payload_ramdisk_init_trimmed_because_config_initrd_disabled(self);
            payload_default_init_branch_trimmed_noop(self);
            payload_default_init_trimmed_because_config_default_init_empty(self);
            payload_binfmt_script_deferred(self);
            payload_exec_panic_terminal_bound(self);
        }
    }
}

object UserCloneDeferredBoundaries: KernelObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PayloadParam.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    SystemState.state == State::Online;
                    system_state_running(SystemState);
                }

                ensures {
                    user_clone_deferred_boundaries_ready(self);
                    user_clone_linux_6_12_legacy_clone_bound(self);
                    user_clone_riscv_abi_argument_order_bound(self);
                    user_clone_observed_plain_fork_args_bound(self);
                    user_clone_observed_vfork_vm_args_bound(self);
                    user_clone_observed_vfork_pidfd_args_bound(self);
                    user_clone_plain_fork_first_slice_bound(self);
                    user_clone_vfork_vm_first_slice_bound(self);
                    user_clone_vfork_pidfd_first_slice_bound(self);
                    user_clone_bounded_sequential_vfork_records_bound(self);
                    user_clone_single_active_child_slot_bound(self);
                    user_clone_completed_child_record_capacity_bound(self);
                    user_clone_csignal_split_bound(self);
                    user_clone_sigchld_exit_signal_bound(self);
                    user_clone_newsp_zero_inherits_parent_sp(self);
                    user_clone_newsp_sets_child_sp(self);
                    user_clone_legacy_pidfd_uses_parent_tidptr(self);
                    user_clone_tls_ignored_without_clone_settls(self);
                    user_clone_thread_group_deferred(self);
                    user_clone_full_clone_vm_vfork_deferred(self);
                    user_clone_cow_mm_deferred(self);
                    user_clone_full_pidfd_file_ops_deferred(self);
                    user_clone_ptrace_seccomp_cgroup_audit_deferred(self);
                    user_clone_namespace_deferred(self);
                    user_clone_robust_futex_deferred(self);
                    user_clone_clear_child_futex_deferred(self);
                    user_clone_wait_exit_reap_deferred(self);
                    user_clone_unsupported_flags_first_slice(self);
                }

                deferred {
                    "BusyBox /bin/sh 输入 ls 的原始观察为 clone(220) a0=0x11, a1=0, a2=0, a3=8, a4=0x20096580, a5=1；按 Linux 6.12 RISC-V legacy clone ABI，a0 的低 8 位 CSIGNAL 为 SIGCHLD，去掉 CSIGNAL 后没有额外 CLONE_* flags，因此首片是 plain fork。newsp=0 表示 child 继承 parent 用户 sp；没有 CLONE_SETTLS 时 a4/tls 不写入 child tp，child 继承 parent TLS。本首片实现后，同一 guest 输入 ls 已越过 clone，下一条观察为 parent wait4(260) a0=-1, a1=0x3ffff77c, a2=2, a3=0, a4=0, a5=0；当前 wait4 首片记录 parent wait_chldexit 边界、保存 parent wait frame 和 parent address-space snapshot，然后交给 child continuation。wait4 handoff 诊断确认此前 child continuation instruction page fault 时 satp 匹配，sepc/stval 落在用户 ELF writable non-executable 页，根因是 parent 返回 child pid并执行 wait4 期间复用同一用户栈页污染 child continuation；当前首片只复制 bounded 用户栈 snapshot 并在 wait4 handoff 前恢复，已越过该 page fault。后续证据显示 child /bin/ls 成功 exit_group(0) 后不应触发全系统 shutdown，而应形成 wait4-completable child exit status、恢复 parent address space 并让 parent wait4 返回 child pid；新的 objdump 证据显示 parent wait4 返回后 BusyBox 在 stack canary 检查 `ld a5,0(s4)` 处以 `stval=1` fault，register checkpoint 又显示 parent wait saved/resumed 的 s4 均为 0x1，parent wait 用户栈窗口 checkpoint 显示 handoff 前保存的 512 字节窗口在 child exit 后已有 171 字节差异，因此定位为缺少 Linux COW/mm 隔离导致父栈页被 child continuation 污染。wait4 首片必须保存 parent wait stack snapshot，并在 child exit 恢复 parent address-space 后、wait status copyout 前恢复该 snapshot；这是完整 `dup_mm()`/COW mm deferred 前的首片替代。prompt-return 后 BusyBox 还会调用 child 已收割后的 follow-up wait4；Linux 6.12 `kernel_wait4()` 会追加 `WEXITED`，且无 eligible child 时 `__do_wait()` 返回 `-ECHILD`。当前已观察到 `wait4(-1, status, options=0, NULL)` 和 `wait4(-1, status, options=3, NULL)` 形状，因此首片在 observed child 已收割后对 valid options 返回 `ECHILD`，不得继续返回 `ENOSYS` 污染 shell 上一条命令状态。若 shell last status 仍变为 255，wait4 checkpoint 必须先保存并比较 parent writable user page checksum summary；当前证据为 checked=525、dirty=4、stack_dirty=1、non_stack_dirty=3，首个非栈 dirty 页为 ElfSegment mapping index 1 / page 4 / vaddr 0x100c9000，证明污染已经越过栈页。当前首片修正必须在 wait4 handoff 前复制 parent writable user pages 的完整页面内容，child exit 恢复 parent address-space 后先比较 checksum并保留 dirty facts，再在 wait status copyout 前恢复全部 saved writable pages；这是单 observed-child 的 parent page rollback，不等价于完整 Linux COW mm。OpenRC 原生 /sbin/init 的当前新观察为 clone_flags=0x4111，实测 flags_without_csignal=0x4100，即 SIGCHLD=17 | CLONE_VM=0x100 | CLONE_VFORK=0x4000，newsp 为 child sp；它不是 CLONE_PIDFD。当前片采用单 active child execution slot + bounded completed-child records：child bounded exit/exit_group 恢复 parent clone frame、让 parent clone 返回 child pid 后，归档 pid/status/wait status/reaped=false，释放 internal UserChild runqueue fact，并把 slot 复位为 Prepared-like reusable；下一次顺序 vfork clone 分配递增 user-visible pid 并复用同一 internal UserChild task ref。新的可复现边界是 completed records 达到固定容量后停在 `clone_vfork stage=child_records_full`，诊断为 completed_records=8、record_capacity=8、active_slot_reusable=1、next_child_pid=11 且 first_unreaped_pid=0，说明历史已 reaped records 不能继续占用 bounded slot。wait4 成功 reaping 且 status copyout 成功后必须释放对应 completed record slot，保留 last/total archived、reaped、released 诊断；status copyout EFAULT 和 rt_sigtimedwait(SIGCHLD) 消费 signal 均不得 release record。真正含 CLONE_PIDFD=0x1000 的 vfork shape 才安装 pidfd-like fd、写回 parent_tidptr，并在对应 completed record exit 后记录 pidfd readable。active child 未完成、当前 occupied records 容量耗尽、child 长驻、parent/child 并发或多个 runnable user task refs 仍必须停在明确 unsupported/diagnostic 边界。完整 clone3、线程组、完整 CLONE_VM/vfork completion scheduler、pidfs/pidfd file ops、pidfd_send_signal/pidfd_getfd/waitid(P_PIDFD)、ptrace/seccomp/cgroup/audit、namespace、robust futex、clear_child_tid futex wake、完整 wait sleep/wakeup、完整 task graph/zombie lifecycle/release_task/pid hash/资源累计、完整地址空间复制/COW、setsid、orphan pgrp、pty、job-control signal 和未观察到的 flags/options 组合保持 deferred 或 unsupported-first-slice。";
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
            user_clone_bounded_sequential_vfork_records_bound(self);
            user_clone_single_active_child_slot_bound(self);
            user_clone_completed_child_record_capacity_bound(self);
            user_clone_csignal_split_bound(self);
            user_clone_sigchld_exit_signal_bound(self);
            user_clone_newsp_zero_inherits_parent_sp(self);
            user_clone_newsp_sets_child_sp(self);
            user_clone_legacy_pidfd_uses_parent_tidptr(self);
            user_clone_tls_ignored_without_clone_settls(self);
            user_clone_thread_group_deferred(self);
            user_clone_full_clone_vm_vfork_deferred(self);
            user_clone_cow_mm_deferred(self);
            user_clone_full_pidfd_file_ops_deferred(self);
            user_clone_ptrace_seccomp_cgroup_audit_deferred(self);
            user_clone_namespace_deferred(self);
            user_clone_robust_futex_deferred(self);
            user_clone_clear_child_futex_deferred(self);
            user_clone_wait_exit_reap_deferred(self);
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
                    SwapperVm.state == State::Online;
                    PageAllocator.state == State::Ready;
                    KernelGlobalAllocator.state == State::Ready;
                    KernelInitTask.state == State::Online;
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
                    SwapperVm.state == State::Online;
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
                }
            }
        }
    }
}

object UserStack: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    UserAddressSpace.state == State::Prepared;
                    PageAllocator.state == State::Ready;
                }

                ensures {
                    user_stack_allocated(self);
                    user_stack_fixed_size_bound(self);
                    user_stack_mapped_into_address_space(self, UserAddressSpace);
                    user_stack_backing_pages_allocated(self);
                    user_stack_zeroed(self);
                    user_stack_initial_sp_bound(self);
                    user_stack_minimal_arg_env_bound(self);
                    user_stack_initial_argc_argv_envp_auxv_bound(self);
                    user_stack_static_libc_entry_supported(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            user_stack_allocated(self);
            user_stack_fixed_size_bound(self);
            user_stack_mapped_into_address_space(self, UserAddressSpace);
            user_stack_backing_pages_allocated(self);
            user_stack_zeroed(self);
            user_stack_initial_sp_bound(self);
            user_stack_minimal_arg_env_bound(self);
            user_stack_initial_argc_argv_envp_auxv_bound(self);
            user_stack_static_libc_entry_supported(self);
        }
    }
}

object ElfObject: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    vfs_absolute_path_walk_supported(VfsCore);
                    fs_struct_root_dentry_set(FsStruct, Dentry);
                }

                ensures {
                    elf_object_input_bound(self);
                    elf_object_input_from_vfs(self, VfsCore);
                    elf_object_magic_valid(self);
                    elf_object_class_elf64(self);
                    elf_object_little_endian(self);
                    elf_object_machine_riscv(self);
                    elf_object_type_supported(self);
                    elf_object_role_bound(self, ElfObjectRole::MainExecutable);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            elf_object_input_bound(self);
            elf_object_input_from_vfs(self, VfsCore);
            elf_object_magic_valid(self);
            elf_object_class_elf64(self);
            elf_object_little_endian(self);
            elf_object_machine_riscv(self);
            elf_object_type_supported(self);
            elf_object_role_bound(self, ElfObjectRole::MainExecutable);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    elf_object_program_headers_parsed(self);
                    elf_object_pt_load_segments_bound(self);
                    elf_object_segment_permissions_bound(self);
                    elf_object_load_plan_bound(self);
                    elf_object_entry_in_executable_segment(self);
                    elf_object_runtime_entry_bound(self);
                    elf_object_auxv_exec_fields_bound(self);
                    elf_object_init_content_observed(self);
                    elf_object_bss_zero_plan_bound(self);
                    elf_object_entry_bound(self);
                    elf_object_load_merged_into_setup(self);
                    /*
                     * Static executables keep elf_object_no_separate_loader.
                     * Dynamically linked executables instead bind PT_INTERP to
                     * a second ElfObject role, Interpreter. The interpreter is
                     * not an ElfLoader resource object and load remains merged
                     * into ElfObject.Setup / UserAddressSpace.Setup.
                     *
                     * Linux 6.12 fs/binfmt_elf.c::load_elf_binary() accepts
                     * both ET_EXEC and ET_DYN. It distinguishes ET_DYN PIE
                     * programs by the presence of PT_INTERP and loads them away
                     * from the interpreter/loader using a load_bias derived
                     * from ELF_ET_DYN_BASE plus ASLR. This model keeps the same
                     * classification but trims ASLR/VMA search to a fixed,
                     * non-overlapping main PIE load bias for the first slice.
                     * ET_DYN without PT_INTERP is the direct-loader form and is
                     * explicitly deferred here.
                     */
                    elf_object_static_executable(self) || elf_object_dynamic_executable(self);
                    elf_object_et_dyn_pie_main_supported(self);
                    elf_object_et_dyn_loader_without_interp_deferred(self);
                    elf_object_no_separate_loader(self) || elf_object_interpreter_required(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            elf_object_program_headers_parsed(self);
            elf_object_pt_load_segments_bound(self);
            elf_object_segment_permissions_bound(self);
            elf_object_load_plan_bound(self);
            elf_object_entry_in_executable_segment(self);
            elf_object_init_content_observed(self);
            elf_object_bss_zero_plan_bound(self);
            elf_object_entry_bound(self);
            elf_object_load_merged_into_setup(self);
            elf_object_runtime_entry_bound(self);
            elf_object_static_executable(self) || elf_object_dynamic_executable(self);
            elf_object_et_dyn_pie_main_supported(self);
            elf_object_et_dyn_loader_without_interp_deferred(self);
            elf_object_no_separate_loader(self) || elf_object_interpreter_required(self);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    UserAddressSpace.state == State::Ready;
                    UserStack.state == State::Ready;
                    UserTrapFrame.state == State::Ready;
                }

                ensures {
                    elf_object_user_entry_ready(self);
                }
            }
        }
    }

    state State::Online {
        invariant {
            elf_object_user_entry_ready(self);
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
                    KernelInitTask.state == State::Online;
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
                    SyscallException.state == State::Ready;
                }

                ensures {
                    syscall_table_ready(self);
                    syscall_table_bound_to_exception(self, SyscallException);
                    syscall_table_write_supported(self);
                    syscall_table_writev_supported(self);
                    syscall_table_openat_supported(self);
                    syscall_table_chdir_supported(self);
                    syscall_table_read_supported(self);
                    syscall_table_ppoll_supported(self);
                    syscall_table_close_supported(self);
                    syscall_table_newfstatat_supported(self);
                    syscall_table_readlinkat_supported(self);
                    syscall_table_fcntl_supported(self);
                    syscall_table_ioctl_supported(self);
                    syscall_table_getrandom_supported(self);
                    syscall_table_getuid_supported(self);
                    syscall_table_getgid_supported(self);
                    syscall_table_getpgid_supported(self);
                    syscall_table_setpgid_supported(self);
                    syscall_table_setsid_supported(self);
                    syscall_table_setuid_supported(self);
                    syscall_table_setgid_supported(self);
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
                    syscall_exit_records_status(self);
                    syscall_exit_group_pid1_shutdown_child_wait4_split(self);
                    syscall_setsid_process_group_leader_eperm_first_slice(self);
                    syscall_trace_probe_observes_returns_without_side_effect(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            syscall_table_ready(self);
            syscall_table_bound_to_exception(self, SyscallException);
            syscall_table_write_supported(self);
            syscall_table_writev_supported(self);
            syscall_table_openat_supported(self);
            syscall_table_chdir_supported(self);
            syscall_table_read_supported(self);
            syscall_table_ppoll_supported(self);
            syscall_table_close_supported(self);
            syscall_table_newfstatat_supported(self);
            syscall_table_readlinkat_supported(self);
            syscall_table_fcntl_supported(self);
            syscall_table_ioctl_supported(self);
            syscall_table_getrandom_supported(self);
            syscall_table_getuid_supported(self);
            syscall_table_getgid_supported(self);
            syscall_table_getpgid_supported(self);
            syscall_table_setpgid_supported(self);
            syscall_table_setsid_supported(self);
            syscall_table_setuid_supported(self);
            syscall_table_setgid_supported(self);
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
            syscall_exit_records_status(self);
            syscall_exit_group_pid1_shutdown_child_wait4_split(self);
            syscall_setsid_process_group_leader_eperm_first_slice(self);
        }

        actions {
            on Action::Write {
                depends_on {
                    SyscallException.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                }

                drives {
                    FilesStruct.Action::LookupFd(FdRef::Stdout);
                    FileDescriptorTable.Action::Lookup(FdRef::Stdout);
                    OpenFileDescription.Action::Write;
                    FileBackend.Action::WriteCharDevice;
                }

                ensures {
                    syscall_write_usercopy_ready(self);
                    files_struct_fd_lookup_routes_to_table(FilesStruct, FileDescriptorTable);
                    fd_table_lookup_returns(FileDescriptorTable, FdRef::Stdout, OpenFileDescription);
                    open_file_description_write_dispatches_backend(OpenFileDescription, FileBackend);
                    file_backend_write_to_console(FileBackend);
                    syscall_table_write_observed(self);
                }
            }

            on Action::Writev {
                depends_on {
                    SyscallException.state == State::Online;
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
                 * console-like character-device opened instance. If
                 * O_DIRECTORY is present and the resolved non-TTY target is
                 * not a directory, the syscall must fail with ENOTDIR. O_NONBLOCK
                 * is consumed only by accepted TTY opens in this slice; regular
                 * and directory opens do not gain nonblocking read/write
                 * semantics. Write/create modes, O_PATH, O_TMPFILE, nofollow,
                 * permissions, LSM hooks, mount namespaces, real VT/devtmpfs
                 * and full errno detail remain trimmed.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FsStruct.state == State::Ready;
                    VfsCore.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    syscall_path_usercopy_ready(self);
                }

                drives {
                    FilesStruct.Action::OpenPath;
                    FileDescriptorTable.Action::Install(FdRef::Regular0);
                }

                ensures {
                    syscall_openat_routes_to_files_struct(self, FilesStruct);
                    files_struct_open_path_routes_to_vfs(FilesStruct, VfsCore);
                    files_struct_regular_fd_installed(FilesStruct) ||
                        files_struct_directory_fd_installed(FilesStruct) ||
                        files_struct_tty_alias_fd_installed(FilesStruct);
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
                 * observed BusyBox/OpenRC chdir("/") path after native init has
                 * opened and closed "/", reuses the existing AT_FDCWD/rooted
                 * VfsCore path walk and requires the resolved target to be a
                 * directory through FsStruct.Action::Chdir. Permission checks,
                 * LSM hooks, refcount/seqcount locking and ESTALE retry remain
                 * deferred. Unsupported path-walk shapes stay outside the slice
                 * and may return ENOSYS instead of pretending to be complete
                 * Linux chdir(2).
                 */
                depends_on {
                    SyscallException.state == State::Online;
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
                 * This slice supports two already-open fd classes: the existing
                 * Regular0 read-only file, and fd0 char-device stdin through
                 * the minimal N_TTY line discipline. In canonical mode,
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
                 * scheduler sleep behind wait_woken(). Job control, signal interruption/restart,
                 * echo/erase, special input characters and full poll/ppoll
                 * integration remain deferred. A user-read-trace probe may
                 * observe fd, requested length and the returned read result for
                 * distro debugging, but must not alter this action's return
                 * value, errno path, checkpoint ordering or smoke policy.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    syscall_read_usercopy_ready(self);
                }

                drives {
                    FilesStruct.Action::ReadFd(FdRef::Regular0);
                    FilesStruct.Action::ReadFd(FdRef::Stdin);
                    FileBackend.Action::ReadCharDevice;
                    NTtyLineDiscipline.Action::ReadLineOrBytes;
                    TtyInputWait.Action::WaitReadable;
                }

                ensures {
                    syscall_read_routes_to_files_struct(self, FilesStruct);
                    files_struct_regular_file_read_observed(FilesStruct);
                    syscall_read_stdin_ready_data_first_slice(self);
                    files_struct_stdin_char_device_read_observed(FilesStruct);
                    open_file_description_read_observed(OpenFileDescription);
                    file_backend_regular_file_read_returns_data(FileBackend);
                    file_backend_char_device_read_returns_ready_data(FileBackend);
                    tty_flip_buffer_ready_data_consumed(TtyFlipBuffer);
                    syscall_read_no_ready_blocking_out_of_slice(self);
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
                 * from the existing fd table and char-device N_TTY readiness.
                 * Canonical mode requires a newline-terminated bounded input
                 * slice before fd0 reports POLLIN; noncanonical mode preserves
                 * byte readiness. It parses and validates the optional timeout.
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
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

            on Action::Ioctl {
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
                 * TCSETS immediate mutation, and TIOCGPGRP/TIOCSPGRP against
                 * the UserInitProcess controlling-tty foreground-pgrp state.
                 * TCSETSW/TCSETSF, drain/flush, driver and line-discipline
                 * set_termios hooks, canonical N_TTY behavior, pty and real
                 * TTY locking remain deferred.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    syscall_ioctl_usercopy_ready(self);
                }

                drives {
                    FilesStruct.Action::LookupFd(FdRef::Stdout);
                    FilesStruct.Action::ReadTermios(FdRef::Stdout);
                    FilesStruct.Action::SetTermios(FdRef::Stdout);
                    UserInitProcess.Action::ReadForegroundProcessGroup;
                    UserInitProcess.Action::SetForegroundProcessGroup;
                }

                ensures {
                    syscall_ioctl_routes_to_files_struct(self, FilesStruct);
                    files_struct_fd_lookup_routes_to_table(FilesStruct, FileDescriptorTable);
                    syscall_ioctl_tcgets_termios_first_slice(self);
                    syscall_ioctl_tcsets_termios_mutation_first_slice(self);
                    syscall_ioctl_tiocgpgrp_foreground_pgrp_first_slice(self);
                    syscall_ioctl_tiocspgrp_foreground_pgrp_update_first_slice(self);
                    files_struct_tty_termios_state_bound(FilesStruct);
                    files_struct_tty_termios_mutation_observed(FilesStruct);
                    user_init_process_controlling_tty_bound(UserInitProcess);
                    user_init_process_foreground_pgrp_read_observed(UserInitProcess);
                    user_init_process_foreground_pgrp_set_observed(UserInitProcess);
                    user_init_process_foreground_pgrp_accepts_child_pgrp_first_slice(UserInitProcess, UserChildProcess);
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::ReadUid;
                }

                ensures {
                    syscall_getuid_routes_to_user_init_process(self, UserInitProcess);
                    user_init_process_uid_read_observed(UserInitProcess);
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
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::ReadGid;
                }

                ensures {
                    syscall_getgid_routes_to_user_init_process(self, UserInitProcess);
                    user_init_process_gid_read_observed(UserInitProcess);
                    syscall_table_getgid_observed(self);
                }
            }

            on Action::GetPid {
                /*
                 * Linux 6.12 kernel/sys.c::sys_getpid() returns
                 * task_tgid_vnr(current). The current user init is the
                 * exec-transformed KernelInitTask, so this first slice returns
                 * the preserved PID1 task identity.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::ReadProcessId;
                }

                ensures {
                    syscall_getpid_routes_to_user_init_process(self, UserInitProcess);
                    user_init_process_pid_read_observed(UserInitProcess);
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
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::ReadParentProcessId;
                }

                ensures {
                    syscall_getppid_routes_to_user_init_process(self, UserInitProcess);
                    user_init_process_ppid_zero_first_slice(UserInitProcess);
                    user_init_process_ppid_read_observed(UserInitProcess);
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
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::ReadProcessGroup;
                }

                ensures {
                    syscall_getpgid_routes_to_user_init_process(self, UserInitProcess);
                    user_init_process_process_group_read_observed(UserInitProcess);
                    syscall_table_getpgid_observed(self);
                }
            }

            on Action::SetPgid {
                /*
                 * Linux 6.12 sys_setpgid() normalizes pid==0 to current and
                 * pgid==0 to the normalized pid. After plain fork, the parent
                 * may set the not-yet-exec child into a process group whose
                 * id equals the child pid. The current slice only admits PID1
                 * pgrp 1 and the observed child pid/pgrp 3 in the same
                 * session; full tasklist/RCU, PF_FORKNOEXEC lifetime,
                 * security hooks and multi-process process groups remain
                 * deferred.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::SetProcessGroup;
                }

                ensures {
                    syscall_setpgid_routes_to_user_init_process(self, UserInitProcess);
                    syscall_setpgid_child_plain_fork_first_slice(self, UserChildProcess);
                    user_init_process_process_group_set_observed(UserInitProcess);
                    user_init_process_child_process_group_set_observed(UserInitProcess, UserChildProcess);
                    syscall_table_setpgid_observed(self);
                }
            }

            on Action::SetSid {
                /*
                 * Linux 6.12 kernel/sys.c::ksys_setsid() fails with EPERM
                 * when the current group leader is already a session leader
                 * or when a process-group id equal to the proposed session id
                 * exists. The current first slice preserves PID1's existing
                 * session-leader/process-group-leader identity and only
                 * exposes that conservative EPERM result; creating a new
                 * session, changing SID/PGID and detaching the controlling tty
                 * remain deferred.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::SetSessionId;
                }

                ensures {
                    syscall_setsid_routes_to_user_init_process(self, UserInitProcess);
                    syscall_setsid_process_group_leader_eperm_first_slice(self);
                    user_init_process_setsid_eperm_observed(UserInitProcess);
                    syscall_table_setsid_observed(self);
                }
            }

            on Action::GetEuid {
                /*
                 * Linux 6.12 kernel/sys.c::sys_geteuid() reads current_euid().
                 * The first slice routes it to the current UserInitProcess
                 * root credentials substate.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::ReadEffectiveUid;
                }

                ensures {
                    syscall_geteuid_routes_to_user_init_process(self, UserInitProcess);
                    user_init_process_euid_read_observed(UserInitProcess);
                    syscall_table_geteuid_observed(self);
                }
            }

            on Action::GetEgid {
                /*
                 * Linux 6.12 kernel/sys.c::sys_getegid() reads current_egid().
                 * The first slice routes it to the current UserInitProcess
                 * root credentials substate.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::ReadEffectiveGid;
                }

                ensures {
                    syscall_getegid_routes_to_user_init_process(self, UserInitProcess);
                    user_init_process_egid_read_observed(UserInitProcess);
                    syscall_table_getegid_observed(self);
                }
            }

            on Action::GetResUid {
                /*
                 * Linux 6.12 kernel/sys.c::sys_getresuid() snapshots real,
                 * effective and saved uid from current_cred(), then writes the
                 * three uid_t values to user memory in order. User pointer
                 * failure returns EFAULT. The first slice keeps all three root
                 * ids on UserInitProcess and writes riscv64 uid_t-sized
                 * values.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                    syscall_credentials_usercopy_ready(self);
                }

                drives {
                    UserInitProcess.Action::ReadResUid;
                }

                ensures {
                    syscall_getresuid_routes_to_user_init_process(self, UserInitProcess);
                    user_init_process_resuid_read_observed(UserInitProcess);
                    syscall_table_getresuid_observed(self);
                }
            }

            on Action::GetResGid {
                /*
                 * Linux 6.12 kernel/sys.c::sys_getresgid() mirrors getresuid
                 * for real/effective/saved gid. The current slice writes the
                 * three root gid_t values from UserInitProcess.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                    syscall_credentials_usercopy_ready(self);
                }

                drives {
                    UserInitProcess.Action::ReadResGid;
                }

                ensures {
                    syscall_getresgid_routes_to_user_init_process(self, UserInitProcess);
                    user_init_process_resgid_read_observed(UserInitProcess);
                    syscall_table_getresgid_observed(self);
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
                    SyscallException.state == State::Online;
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
                 * the user buffer is too small. The current slice only covers
                 * the boot UserInitProcess whose inherited FsStruct root and
                 * pwd both point to the ext2 root, so getcwd returns "/\0".
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                    FsStruct.state == State::Ready;
                    syscall_getcwd_usercopy_ready(self);
                }

                drives {
                    UserInitProcess.Action::ReadCurrentWorkingDirectory;
                }

                ensures {
                    syscall_getcwd_routes_to_user_init_process(self, UserInitProcess);
                    user_init_process_root_cwd_first_slice(UserInitProcess, FsStruct);
                    syscall_getcwd_returns_root_with_nul(self);
                    syscall_table_getcwd_observed(self);
                }
            }

            on Action::SetUid {
                /*
                 * Linux 6.12 kernel/sys.c::__sys_setuid() prepares and commits
                 * new credentials for current. The current first slice keeps a
                 * root PID1 credential object on UserInitProcess and accepts
                 * the no-op/root setuid path required by BusyBox startup;
                 * namespaces, capability checks, LSM hooks, user accounting and
                 * credential COW/RCU are explicit deferred facts.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::SetUid;
                }

                ensures {
                    syscall_setuid_routes_to_user_init_process(self, UserInitProcess);
                    syscall_credentials_full_linux_model_deferred(self);
                    user_init_process_uid_set_observed(UserInitProcess);
                    syscall_table_setuid_observed(self);
                }
            }

            on Action::SetGid {
                /*
                 * Linux 6.12 kernel/sys.c::__sys_setgid() mirrors setuid for
                 * group credentials. The current first slice only preserves
                 * the root/no-op path and records the remaining credential
                 * machinery as deferred.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::SetGid;
                }

                ensures {
                    syscall_setgid_routes_to_user_init_process(self, UserInitProcess);
                    syscall_credentials_full_linux_model_deferred(self);
                    user_init_process_gid_set_observed(UserInitProcess);
                    syscall_table_setgid_observed(self);
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
                 * The current slice stores the blocked mask on UserInitProcess
                 * because the PID1 task is the exec-transformed
                 * KernelInitTask. Full signal delivery, shared sighand,
                 * pending queues, restart, thread-group semantics and
                 * siglock/IRQ locking are deferred.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                    syscall_signal_mask_usercopy_ready(self);
                }

                drives {
                    UserInitProcess.Action::RtSigprocmask;
                }

                ensures {
                    syscall_rt_sigprocmask_routes_to_user_init_process(self, UserInitProcess);
                    syscall_rt_sigprocmask_sigsetsize_bound(self);
                    syscall_rt_sigprocmask_unblockable_signals_cleared(self);
                    syscall_signal_delivery_deferred(self);
                    user_init_process_rt_sigprocmask_observed(UserInitProcess);
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
                 * SignalActionTable, folded into the current UserInitProcess
                 * implementation for single PID1/single-thread execution.
                 * ProcessSignalState shared pending queues, ThreadSignalState
                 * pending delivery, siglock/RCU, restart handling, signal
                 * frame construction and rt_sigreturn remain deferred.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                    syscall_signal_action_usercopy_ready(self);
                }

                drives {
                    UserInitProcess.Action::RtSigaction;
                }

                ensures {
                    syscall_rt_sigaction_routes_to_user_init_process(self, UserInitProcess);
                    syscall_rt_sigaction_routes_to_signal_action_table(self, UserInitProcess);
                    syscall_rt_sigaction_sigsetsize_bound(self);
                    syscall_rt_sigaction_layout_bound(self);
                    syscall_rt_sigaction_unblockable_signals_cleared(self);
                    syscall_rt_sigaction_kernel_only_signals_rejected(self);
                    syscall_signal_delivery_deferred(self);
                    user_init_process_rt_sigaction_observed(UserInitProcess);
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
                 * The current OpenRC evidence reaches
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
                 * A real observed UserChildProcess exit/exit_group may set
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
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                    syscall_signal_mask_usercopy_ready(self);
                }

                drives {
                    UserInitProcess.Action::RtSigtimedwait;
                }

                ensures {
                    syscall_rt_sigtimedwait_routes_to_user_init_process(self, UserInitProcess);
                    syscall_rt_sigtimedwait_sigsetsize_bound(self);
                    syscall_rt_sigtimedwait_copies_wait_mask(self);
                    syscall_rt_sigtimedwait_uinfo_null_no_copyout_first_slice(self);
                    syscall_rt_sigtimedwait_uts_null_infinite_wait_first_slice(self);
                    syscall_rt_sigtimedwait_empty_pending_wait_boundary(self);
                    syscall_rt_sigtimedwait_waitqueue_sleep_first_slice(self);
                    syscall_rt_sigtimedwait_sigchld_pending_first_slice(self);
                    syscall_rt_sigtimedwait_return_signal_first_slice(self);
                    user_init_process_rt_sigtimedwait_observed(UserInitProcess);
                    user_init_process_rt_sigtimedwait_pending_match_empty(UserInitProcess);
                    user_init_process_rt_sigtimedwait_infinite_wait(UserInitProcess);
                    user_init_process_pending_sigchld_first_slice(UserInitProcess);
                    user_init_process_rt_sigtimedwait_waiter_enqueued(UserInitProcess);
                    user_init_process_rt_sigtimedwait_sleep_reason_bound(UserInitProcess);
                    user_init_process_rt_sigtimedwait_woken_by_sigchld(UserInitProcess);
                    user_init_process_rt_sigtimedwait_dequeued_sigchld(UserInitProcess);
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
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
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                }

                drives {
                    UserInitProcess.Action::SetClearChildTid;
                }

                ensures {
                    syscall_set_tid_address_routes_to_user_init_process(self, UserInitProcess);
                    user_init_process_clear_child_tid_bound(UserInitProcess);
                    syscall_table_set_tid_address_observed(self);
                }
            }

            on Action::Clone {
                /*
                 * Linux 6.12 legacy clone(220) on riscv64 receives
                 * clone_flags, newsp, parent_tidptr, child_tidptr and tls in
                 * a0..a4. The current observed BusyBox /bin/sh "ls" path is a
                 * plain fork: clone_flags=0x11, so exit_signal=SIGCHLD and
                 * flags with CSIGNAL removed are zero.  newsp=0 keeps the
                 * copied child user sp, and tls is ignored because
                 * CLONE_SETTLS is not set.  The syscall action decodes this
                 * ABI shape, then drives TaskCreationCore.CopyUserProcess;
                 * it must not synthesize a PID return without creating a
                 * child task boundary.
                 *
                 * The OpenRC native /sbin/init boundary observes
                 * clone_flags=0x4111 and diagnostics decode
                 * flags_without_csignal=0x4100: SIGCHLD plus CLONE_VM and
                 * CLONE_VFORK, not CLONE_PIDFD.  Linux 6.12 legacy clone
                 * maps CLONE_PIDFD to parent_tidptr only when the 0x1000 bit
                 * is actually present, and kernel_clone() waits for vfork
                 * completion before returning to parent.  This first slice
                 * implements the bounded sequential child lifecycle: save
                 * the parent clone frame, set child a0=0 and child sp=newsp,
                 * then hand off directly to the child.  A previous completed
                 * vfork child may remain as an unreaped completed-child record
                 * until wait4 successfully reaps it; after successful wait4,
                 * the slot is released and only total/last diagnostics remain.
                 * The active execution slot must be reusable before
                 * CopyUserProcess runs again.  Records-full means all bounded
                 * slots are currently occupied by unreaped/diagnostic records,
                 * not that the lifetime archive counter reached capacity.  A
                 * true vfork+pidfd shape additionally installs a pidfd-like fd
                 * in FilesStruct and copies it to parent_tidptr.
                 *
                 * Because the current runtime still has one shared
                 * FilesStruct object, CopyUserProcess also saves a bounded
                 * parent fd table and regular-slot metadata snapshot for the
                 * single active child. Child execve close-on-exec may mutate
                 * the shared runtime table while the child runs, but child
                 * exit must restore the parent snapshot before resuming the
                 * parent.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    TaskCreationCore.state == State::Ready;
                    UserInitProcess.state == State::Online;
                    UserChildProcess.state == State::Prepared;
                    user_child_process_active_slot_reusable(UserChildProcess);
                    user_child_process_completed_records_capacity_bound(UserChildProcess);
                    UserCloneDeferredBoundaries.state == State::Ready;
                    Scheduler.state == State::Online;
                    RootPidNamespace.state == State::Ready;
                    FsStruct.state == State::Ready;
                    FilesStruct.state == State::Ready;
                    UserAddressSpace.state == State::Online;
                    UserTrapFrame.state == State::Ready;
                }

                drives {
                    TaskCreationCore.Action::CopyUserProcess(
                        src_process: UserInitProcess,
                        dst_process: UserChildProcess,
                        pid_ns: RootPidNamespace,
                        scheduler: Scheduler,
                        fs: FsStruct,
                        files: FilesStruct,
                        address_space: UserAddressSpace,
                        trap_frame: UserTrapFrame,
                        boundaries: UserCloneDeferredBoundaries
                    );
                    FilesStruct.Action::SaveParentFdSnapshot;
                }

                ensures {
                    syscall_clone_routes_to_task_creation_core(self, TaskCreationCore);
                    syscall_clone_routes_to_user_clone_deferred_boundaries(self, UserCloneDeferredBoundaries);
                    syscall_clone_legacy_args_decoded(self);
                    syscall_clone_plain_fork_first_slice(self);
                    syscall_clone_vfork_vm_first_slice(self);
                    syscall_clone_vfork_pidfd_first_slice(self);
                    syscall_clone_parent_returns_child_pid(self, UserInitProcess);
                    syscall_clone_child_return_zero_bound(self, UserChildProcess);
                    syscall_clone_pidfd_copyout_bound(self, FilesStruct);
                    syscall_clone_vfork_parent_frame_saved(self, UserChildProcess);
                    syscall_clone_vfork_child_handoff(self, UserChildProcess);
                    syscall_clone_vfork_next_child_accepted(self, UserChildProcess);
                    syscall_clone_wake_up_new_task_shape(self, Scheduler);
                    user_child_process_process_group_visible_to_parent(UserChildProcess, UserInitProcess);
                    user_init_process_child_process_group_visible(UserInitProcess, UserChildProcess);
                    user_child_process_user_stack_snapshot_copied(UserChildProcess, UserAddressSpace);
                    user_child_process_parent_fd_snapshot_saved(UserChildProcess, FilesStruct);
                    files_struct_parent_fd_snapshot_saved(FilesStruct, UserChildProcess);
                    user_child_process_single_active_slot(UserChildProcess);
                    user_child_process_next_child_pid_bound(UserChildProcess);
                    syscall_table_clone_observed(self);
                }
            }

            on Action::Execve {
                /*
                 * Linux 6.12 execve(221) routes through
                 * fs/exec.c::do_execveat_common()/bprm_execve() and the ELF
                 * binfmt handler.  The observed BusyBox /bin/sh "ls" child
                 * continuation issues execve("/bin/ls", argv={"ls", NULL},
                 * envp={"SHLVL=1", "PWD=/", NULL}) and then tries
                 * "/usr/bin/ls" if the first attempt returns ENOSYS.  This
                 * first slice accepts the child-continuation path, copies the
                 * filename and argv[0], reuses the current UserBootPayload
                 * VFS/ELF/interpreter/UserStack/UserAddressSpace loading
                 * shape to build a replacement user mm.  This follows the
                 * local Linux 6.12 shape where fs/exec.c::alloc_bprm()
                 * heap-allocates linux_binprm, bprm_mm_init() installs a
                 * nascent bprm->mm from mm_alloc(), begin_new_exec() crosses
                 * the point-of-no-return/context handoff, exec_mmap(bprm->mm)
                 * installs the new mm, and RISC-V start_thread() installs
                 * the return pt_regs.  The replacement UserAddressSpace is
                 * staged in Context-owned storage rather than on the
                 * syscall/trap stack.  Child exit paths release backing pages
                 * for a child address space that exec replaced before
                 * restoring the saved parent snapshot, while preserving a
                 * vfork parent that still references the old mm; full
                 * old-mm/page-table/VMA reclamation remains deferred.  The runtime
                 * checkpoint order is ContextReplaced, SatpReady, then
                 * TrapFrameReady; live satp switch and final return-frame
                 * diagnostics remain later return-path boundaries.  The
                 * current first slice runs the fixed-table close-on-exec scan;
                 * when this is a child continuation, the saved parent fd
                 * snapshot is the rollback boundary that prevents child
                 * close-on-exec from closing the parent's fd entries. It does
                 * not model the
                 * full point-of-no-return rollback, credentials, signal table,
                 * files unshare/refcounting, task comm, perf/audit/accounting or
                 * complete old-mm reclamation paths.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserBootPayload.state == State::Online;
                    UserChildProcess.state == State::Ready;
                    UserAddressSpace.state == State::Online;
                    UserTrapFrame.state == State::Ready;
                    FilesStruct.state == State::Ready;
                    FsStruct.state == State::Ready;
                    VfsCore.state == State::Ready;
                }

                drives {
                    UserBootPayload.Action::TryDefaultInitSequence;
                    ElfObject.Transition::Preset;
                    ElfObject.Transition::Setup;
                    UserStack.Transition::Setup;
                    UserAddressSpace.Transition::Setup;
                    UserAddressSpace.Transition::Enable;
                    UserTrapFrame.Transition::Setup;
                }

                ensures {
                    syscall_execve_linux_6_12_do_execveat_common_bound(self);
                    syscall_execve_observed_shell_ls_args_bound(self);
                    syscall_execve_observed_openrc_getty_args_bound(self);
                    syscall_execve_child_continuation_first_slice(self, UserChildProcess);
                    syscall_execve_reuses_user_boot_payload_elf_loader(self, UserBootPayload);
                    syscall_execve_replaces_user_address_space_first_slice(self, UserAddressSpace);
                    syscall_execve_context_staging_address_space_bound(self, UserAddressSpace);
                    syscall_execve_sets_start_thread_frame_first_slice(self, UserTrapFrame);
                    syscall_execve_argv0_first_slice(self);
                    syscall_execve_stage_checkpoints_bound(self);
                    syscall_execve_return_path_diagnostic_bound(self);
                    syscall_execve_envp_full_copy_deferred(self);
                    syscall_execve_close_on_exec_deferred(self);
                    syscall_execve_old_user_backing_reclaimed_first_slice(self);
                    syscall_execve_old_mm_reclaim_deferred(self);
                    syscall_execve_full_linux_model_deferred(self);
                    syscall_table_execve_observed(self);
                }
            }

            on Action::Wait4 {
                /*
                 * Linux 6.12 wait4(260) routes through
                 * kernel/exit.c::kernel_wait4()/do_wait(). For the observed
                 * BusyBox /bin/sh "ls" parent path, pid is -1, status is a
                 * user pointer, options is WUNTRACED, and rusage is NULL.
                 * kernel_wait4() adds WEXITED internally. Because the cloned
                 * child exists but has not produced a waitable exit/stop/
                 * continue event, do_wait() reaches the interruptible
                 * wait_chldexit boundary and would schedule another runnable
                 * task. This first slice records that parent wait boundary,
                 * saves the parent wait frame/address-space snapshot, and
                 * yields to the child trap-frame continuation already produced
                 * by clone. A later observed child exit_group can then restore
                 * the parent address space, copy the Linux wait status to the
                 * parent status pointer, and return the child pid from wait4.
                 * After that observed child has been reaped, a follow-up
                 * wait4(-1, status, valid_options, NULL) has no eligible child
                 * and returns ECHILD, matching __do_wait()'s notask_error path
                 * after kernel_wait4() adds WEXITED internally.
                 * Completed vfork records are checked before the old active
                 * slot state.  An unreaped completed record is a waitable
                 * child event: wait4 copies status when requested, returns
                 * that user-visible pid, marks only that record reaped, then
                 * releases that occupied slot for bounded sequential reuse.
                 * Status copyout failure returns EFAULT and must not reap or
                 * release the record.  Consuming SIGCHLD through
                 * rt_sigtimedwait is not reaping and cannot release a record.
                 * The native OpenRC /sbin/init path reaches
                 * wait4(-1, NULL, WNOHANG, NULL) after setsid and
                 * rt_sigtimedwait. For that observed nonblocking shape, the
                 * first slice follows __do_wait(): if an eligible child exists
                 * but has no waitable event, return 0 without sleeping; if no
                 * eligible child exists, return ECHILD. This does not create a
                 * synthetic child, does not block, and does not consume signal
                 * or scheduler wait state.
                 * It still does not model full wait queues, zombie lists,
                 * pid hashes, resource aggregation or release_task().
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
                    UserChildProcess.state == State::Ready;
                    Scheduler.state == State::Online;
                }

                ensures {
                    syscall_wait4_linux_6_12_kernel_wait4_bound(self);
                    syscall_wait4_observed_shell_args_bound(self);
                    syscall_wait4_pid_minus_one_all_children_first_slice(self);
                    syscall_wait4_options_wuntraced_first_slice(self);
                    syscall_wait4_wnohang_no_waitable_child_first_slice(self);
                    syscall_wait4_completed_child_record_reap_first_slice(self);
                    syscall_wait4_parent_wait_chldexit_boundary(self);
                    syscall_wait4_yields_to_user_child_continuation(self, UserChildProcess);
                    user_child_process_wait4_parent_wait_observed(UserChildProcess);
                    user_child_process_child_continuation_taken(UserChildProcess);
                    user_child_process_wait4_handoff_frame_diagnostic_bound(UserChildProcess);
                    user_child_process_parent_wait_frame_saved(UserChildProcess);
                    user_child_process_parent_address_space_snapshot_saved(UserChildProcess, UserAddressSpace);
                    user_child_process_parent_wait_register_checkpoint_bound(UserChildProcess);
                    user_child_process_parent_wait_stack_window_checkpoint_bound(UserChildProcess);
                    user_child_process_parent_wait_stack_snapshot_copied(UserChildProcess, UserAddressSpace);
                    user_child_process_parent_wait_writable_page_snapshot_copied(UserChildProcess, UserAddressSpace);
                    user_child_process_user_stack_snapshot_restored(UserChildProcess, UserAddressSpace);
                    user_address_space_fault_mapping_diagnostic_bound(UserAddressSpace);
                    syscall_wait4_child_exit_status_copyout_first_slice(self);
                    syscall_wait4_observed_child_reap_first_slice(self);
                    syscall_wait4_no_child_echild_first_slice(self);
                    syscall_wait4_blocking_sleep_deferred(self);
                    user_child_process_completed_record_reaped(UserChildProcess);
                    syscall_table_wait4_observed(self);
                }
            }

            on Action::Exit {
                depends_on {
                    SyscallException.state == State::Online;
                }

                ensures {
                    syscall_exit_records_status(self);
                    syscall_table_exit_observed(self);
                }
            }

            on Action::ExitGroup {
                depends_on {
                    SyscallException.state == State::Online;
                }

                ensures {
                    syscall_exit_records_status(self);
                    syscall_exit_group_pid1_shutdown_child_wait4_split(self);
                    user_child_process_exit_status_observed(UserChildProcess);
                    user_child_process_wait4_status_copied(UserChildProcess);
                    user_child_process_parent_wait_resumed(UserChildProcess);
                    user_child_process_parent_wait_resume_checkpoint_bound(UserChildProcess);
                    user_child_process_parent_wait_stack_window_compared(UserChildProcess);
                    user_child_process_parent_wait_stack_snapshot_restored(UserChildProcess, UserAddressSpace);
                    user_child_process_parent_wait_writable_page_snapshot_compared(UserChildProcess, UserAddressSpace);
                    user_child_process_parent_wait_writable_page_snapshot_restored(UserChildProcess, UserAddressSpace);
                    user_child_process_parent_fd_snapshot_restored(UserChildProcess, FilesStruct);
                    files_struct_parent_fd_snapshot_restored(FilesStruct, UserChildProcess);
                    user_child_process_completed_record_archived(UserChildProcess);
                    user_child_process_active_slot_reusable(UserChildProcess);
                    syscall_table_exit_observed(self);
                }
            }
        }
    }
}

object UserChildProcess: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    UserInitProcess.state == State::Online;
                    UserCloneDeferredBoundaries.state == State::Ready;
                }

                ensures {
                    user_child_process_prepared(self);
                    task_clone_args_ready(self);
                    task_entry_bound(self, TaskEntry::UserChild);
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            user_child_process_prepared(self);
            task_clone_args_ready(self);
            task_entry_bound(self, TaskEntry::UserChild);
            user_child_process_single_active_slot(self);
            user_child_process_active_slot_reusable(self);
            user_child_process_completed_records_capacity_bound(self);
            user_child_process_next_child_pid_bound(self);
        }
    }

    state State::Ready {
        invariant {
            user_child_process_parent_pid1(self, UserInitProcess);
            user_child_process_pid_allocated(self, RootPidNamespace);
            user_child_process_tgid_equals_pid(self);
            user_child_process_process_group_visible_to_parent(self, UserInitProcess);
            user_child_process_exit_signal_sigchld(self);
            user_child_process_files_struct_copied(self, FilesStruct);
            user_child_process_fs_struct_copied(self, FsStruct);
            user_child_process_parent_fd_snapshot_saved(self, FilesStruct);
            user_child_process_credentials_copied(self, UserInitProcess);
            user_child_process_signal_state_copied(self, UserInitProcess);
            user_child_process_user_address_space_snapshot(self, UserAddressSpace);
            user_child_process_user_stack_snapshot_copied(self, UserAddressSpace);
            user_child_process_trap_frame_copied(self, UserTrapFrame);
            user_child_process_trap_frame_child_return_zero(self);
            user_child_process_tls_inherited(self);
            user_child_process_enqueued(self, Scheduler);
            user_child_process_single_active_slot(self);
            user_child_process_completed_records_capacity_bound(self);
            user_child_process_next_child_pid_bound(self);
        }
    }

    actions {
        on Action::VforkChildExit {
            /*
             * Bounded OpenRC vfork completion: child exit restores the saved
             * parent clone frame and makes parent clone(220) return the
             * user-visible child pid, then archives a completed-child record
             * with raw status, wait status and reaped=false. SIGCHLD pending/
             * wake is derived from that archived record, but rt_sigtimedwait
             * consumption does not reap it. The active execution slot and
             * internal UserChild runqueue fact are then released so a later
             * sequential vfork clone can reuse the slot. True vfork+pidfd
             * shapes additionally mark the pidfd ready for read-interest
             * ppoll and associate readiness with the same completed record.
             */
            depends_on {
                UserChildProcess.state == State::Ready;
                UserInitProcess.state == State::Online;
                FilesStruct.state == State::Ready;
            }

            ensures {
                user_child_process_exit_status_observed(self);
                user_child_process_vfork_parent_resumed(self);
                user_child_process_completed_record_archived(self);
                user_child_process_completed_record_unreaped(self);
                user_child_process_active_slot_reusable(self);
                user_init_process_rt_sigtimedwait_woken_by_sigchld(UserInitProcess);
                user_pidfd_ready(FilesStruct, self);
            }
        }

        on Action::ReapCompletedChildRecord {
            depends_on {
                UserInitProcess.state == State::Online;
                UserChildProcess.state == State::Prepared;
            }

            ensures {
                user_child_process_completed_record_reaped(self);
                user_child_process_completed_record_released(self);
                user_child_process_active_slot_reusable(self);
            }
        }
    }
}

object UserInitProcess: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    UserAddressSpace.state == State::Online;
                    ElfObject.state == State::Online;
                    UserTrapFrame.state == State::Ready;
                    FsStruct.state == State::Ready;
                    FilesStruct.state == State::Ready;
                }

                ensures {
                    user_init_process_reuses_kernel_init_task(self, KernelInitTask);
                    user_init_process_pid1_preserved(self, KernelInitTask);
                    user_init_process_exec_identity_handoff(self, KernelInitTask);
                    user_init_process_no_new_task_struct(self);
                    user_init_process_kernel_init_not_destroyed(self, KernelInitTask);
                    user_init_process_path_bound(self, UserInitPathRef::DefaultInit);
                    user_init_process_address_space_bound(self, UserAddressSpace);
                    user_init_process_fs_struct_inherited(self, FsStruct);
                    user_init_process_files_struct_inherited(self, FilesStruct);
                    user_init_process_trap_frame_bound(self, UserTrapFrame);
                    user_init_process_credentials_inherited(self, KernelInitTask);
                    user_init_process_root_credentials_bound(self);
                    user_init_process_credentials_capability_model_deferred(self);
                    user_init_process_signal_state_inherited(self, KernelInitTask);
                    user_init_process_signal_runtime_bound(self);
                    user_init_process_thread_signal_state_bound(self);
                    user_init_process_process_signal_state_deferred(self);
                    user_init_process_signal_action_table_bound(self);
                    user_init_process_signal_action_table_layout_bound(self);
                    user_init_process_blocked_signal_mask_bound(self);
                    user_init_process_pending_signal_set_empty_first_slice(self);
                    user_init_process_signal_delivery_deferred(self);
                    user_init_process_session_leader_first_slice(self);
                    user_init_process_process_group_leader_first_slice(self);
                    user_init_process_controlling_tty_bound(self);
                    user_init_process_foreground_pgrp_bound(self);
                    kernel_init_task_execve_to_user_init(KernelInitTask, self);
                    kernel_init_task_pid1_identity_preserved(KernelInitTask);
                    kernel_init_task_user_mm_attached(KernelInitTask, UserAddressSpace);
                    kernel_init_task_user_trap_frame_attached(KernelInitTask, UserTrapFrame);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            user_init_process_reuses_kernel_init_task(self, KernelInitTask);
            user_init_process_pid1_preserved(self, KernelInitTask);
            user_init_process_exec_identity_handoff(self, KernelInitTask);
            user_init_process_no_new_task_struct(self);
            user_init_process_kernel_init_not_destroyed(self, KernelInitTask);
            user_init_process_path_bound(self, UserInitPathRef::DefaultInit);
            user_init_process_address_space_bound(self, UserAddressSpace);
            user_init_process_fs_struct_inherited(self, FsStruct);
            user_init_process_files_struct_inherited(self, FilesStruct);
            user_init_process_trap_frame_bound(self, UserTrapFrame);
            user_init_process_credentials_inherited(self, KernelInitTask);
            user_init_process_root_credentials_bound(self);
            user_init_process_credentials_capability_model_deferred(self);
            user_init_process_signal_state_inherited(self, KernelInitTask);
            user_init_process_signal_runtime_bound(self);
            user_init_process_thread_signal_state_bound(self);
            user_init_process_process_signal_state_deferred(self);
            user_init_process_signal_action_table_bound(self);
            user_init_process_signal_action_table_layout_bound(self);
            user_init_process_blocked_signal_mask_bound(self);
            user_init_process_pending_signal_set_empty_first_slice(self);
            user_init_process_signal_delivery_deferred(self);
            user_init_process_session_leader_first_slice(self);
            user_init_process_process_group_leader_first_slice(self);
            user_init_process_controlling_tty_bound(self);
            user_init_process_foreground_pgrp_bound(self);
            kernel_init_task_execve_to_user_init(KernelInitTask, self);
            kernel_init_task_pid1_identity_preserved(KernelInitTask);
            kernel_init_task_user_mm_attached(KernelInitTask, UserAddressSpace);
            kernel_init_task_user_trap_frame_attached(KernelInitTask, UserTrapFrame);
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    UserTrapFrame.state == State::Ready;
                    SyscallTable.state == State::Ready;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_online(self);
                    user_init_process_pid1_preserved(self, KernelInitTask);
                    user_init_process_exec_identity_handoff(self, KernelInitTask);
                    user_init_process_address_space_bound(self, UserAddressSpace);
                    user_init_process_fs_struct_inherited(self, FsStruct);
                    user_init_process_trap_frame_bound(self, UserTrapFrame);
                    user_init_process_credentials_inherited(self, KernelInitTask);
                    user_init_process_root_credentials_bound(self);
                    user_init_process_signal_state_inherited(self, KernelInitTask);
                    user_init_process_signal_runtime_bound(self);
                    user_init_process_thread_signal_state_bound(self);
                    user_init_process_process_signal_state_deferred(self);
                    user_init_process_signal_action_table_bound(self);
                    user_init_process_signal_action_table_layout_bound(self);
                    user_init_process_blocked_signal_mask_bound(self);
                    user_init_process_pending_signal_set_empty_first_slice(self);
                    user_init_process_syscall_context_bound(self, SyscallException, SyscallTable);
                    user_init_process_session_leader_first_slice(self);
                    user_init_process_process_group_leader_first_slice(self);
                    user_init_process_controlling_tty_bound(self);
                    user_init_process_foreground_pgrp_bound(self);
                    kernel_init_task_execve_to_user_init(KernelInitTask, self);
                    kernel_init_task_pid1_identity_preserved(KernelInitTask);
                    kernel_init_task_user_mm_attached(KernelInitTask, UserAddressSpace);
                    kernel_init_task_user_trap_frame_attached(KernelInitTask, UserTrapFrame);
                    user_init_process_trap_return_bound(self, UserTrapFrame);
                    syscall_exception_dispatches_via_table(SyscallException, SyscallTable);
                    syscall_exception_extracts_arguments(SyscallException);
                }
            }
        }
    }

    state State::Online {
        invariant {
            user_init_process_online(self);
            user_init_process_reuses_kernel_init_task(self, KernelInitTask);
            user_init_process_pid1_preserved(self, KernelInitTask);
            user_init_process_exec_identity_handoff(self, KernelInitTask);
            user_init_process_no_new_task_struct(self);
            user_init_process_kernel_init_not_destroyed(self, KernelInitTask);
            user_init_process_address_space_bound(self, UserAddressSpace);
            user_init_process_fs_struct_inherited(self, FsStruct);
            user_init_process_files_struct_inherited(self, FilesStruct);
            user_init_process_trap_frame_bound(self, UserTrapFrame);
            user_init_process_syscall_context_bound(self, SyscallException, SyscallTable);
            user_init_process_credentials_inherited(self, KernelInitTask);
            user_init_process_root_credentials_bound(self);
            user_init_process_credentials_capability_model_deferred(self);
            user_init_process_signal_state_inherited(self, KernelInitTask);
            user_init_process_signal_runtime_bound(self);
            user_init_process_thread_signal_state_bound(self);
            user_init_process_process_signal_state_deferred(self);
            user_init_process_signal_action_table_bound(self);
            user_init_process_signal_action_table_layout_bound(self);
            user_init_process_blocked_signal_mask_bound(self);
            user_init_process_pending_signal_set_empty_first_slice(self);
            user_init_process_signal_delivery_deferred(self);
            user_init_process_session_leader_first_slice(self);
            user_init_process_process_group_leader_first_slice(self);
            user_init_process_controlling_tty_bound(self);
            user_init_process_foreground_pgrp_bound(self);
            kernel_init_task_execve_to_user_init(KernelInitTask, self);
            kernel_init_task_pid1_identity_preserved(KernelInitTask);
            kernel_init_task_user_mm_attached(KernelInitTask, UserAddressSpace);
            kernel_init_task_user_trap_frame_attached(KernelInitTask, UserTrapFrame);
            user_init_process_trap_return_bound(self, UserTrapFrame);
            syscall_exception_dispatches_via_table(SyscallException, SyscallTable);
        }

        actions {
            on Action::EnterUserMode {
                depends_on {
                    UserInitProcess.state == State::Online;
                    UserAddressSpace.state == State::Online;
                    UserTrapFrame.state == State::Ready;
                }

                within UserModeTrapReturnContext {
                    ensures {
                        user_init_process_user_entry_ready(self);
                        user_init_process_enter_user_mode_observed(self, UserTrapFrame);
                        user_trap_return_context_used(self, UserTrapFrame);
                        user_trap_return_switches_satp();
                        user_trap_return_sfence_vma_after_satp();
                        user_trap_return_sret_handoff();
                        user_trap_entry_uses_kernel_stack(UserTrapFrame);
                        user_kernel_trap_stack_linux_riscv_config_bound(UserTrapFrame);
                        user_kernel_trap_stack_thread_size_bound(UserTrapFrame);
                        user_kernel_trap_stack_vmap_alignment_bound(UserTrapFrame);
                        user_kernel_trap_stack_vmapped_bound(UserTrapFrame);
                        user_kernel_trap_stack_vmap_guard_page_bound(UserTrapFrame);
                        user_kernel_trap_stack_overflow_stack_bound(UserTrapFrame);
                        user_kernel_trap_stack_entry_scratch_deferred(UserTrapFrame);
                        user_kernel_trap_stack_irq_stack_switch_deferred(UserTrapFrame);
                    }
                }
            }

            on Action::SetClearChildTid {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_clear_child_tid_bound(self);
                }
            }

            on Action::ReadUid {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_root_credentials_bound(self);
                    user_init_process_uid_read_observed(self);
                }
            }

            on Action::ReadGid {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_root_credentials_bound(self);
                    user_init_process_gid_read_observed(self);
                }
            }

            on Action::ReadProcessId {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_pid1_preserved(self, KernelInitTask);
                    user_init_process_pid_read_observed(self);
                }
            }

            on Action::ReadParentProcessId {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_pid1_preserved(self, KernelInitTask);
                    user_init_process_ppid_zero_first_slice(self);
                    user_init_process_ppid_read_observed(self);
                }
            }

            on Action::ReadProcessGroup {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_session_leader_first_slice(self);
                    user_init_process_process_group_leader_first_slice(self);
                    user_init_process_process_group_read_observed(self);
                }
            }

            on Action::SetProcessGroup {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_session_leader_first_slice(self);
                    user_init_process_process_group_leader_first_slice(self);
                    user_init_process_process_group_set_observed(self);
                    user_init_process_child_process_group_visible(self, UserChildProcess);
                    user_init_process_child_process_group_set_observed(self, UserChildProcess);
                }
            }

            on Action::SetSessionId {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_session_leader_first_slice(self);
                    user_init_process_process_group_leader_first_slice(self);
                    user_init_process_setsid_eperm_observed(self);
                }
            }

            on Action::ReadForegroundProcessGroup {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_controlling_tty_bound(self);
                    user_init_process_foreground_pgrp_bound(self);
                    user_init_process_foreground_pgrp_read_observed(self);
                }
            }

            on Action::SetForegroundProcessGroup {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_controlling_tty_bound(self);
                    user_init_process_foreground_pgrp_bound(self);
                    user_init_process_foreground_pgrp_set_observed(self);
                    user_init_process_foreground_pgrp_accepts_child_pgrp_first_slice(self, UserChildProcess);
                }
            }

            on Action::ReadEffectiveUid {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_root_credentials_bound(self);
                    user_init_process_euid_read_observed(self);
                }
            }

            on Action::ReadEffectiveGid {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_root_credentials_bound(self);
                    user_init_process_egid_read_observed(self);
                }
            }

            on Action::ReadResUid {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_root_credentials_bound(self);
                    user_init_process_resuid_read_observed(self);
                }
            }

            on Action::ReadResGid {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_root_credentials_bound(self);
                    user_init_process_resgid_read_observed(self);
                }
            }

            on Action::ReadCurrentWorkingDirectory {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                    FsStruct.state == State::Ready;
                }

                ensures {
                    user_init_process_fs_struct_inherited(self, FsStruct);
                    user_init_process_root_cwd_first_slice(self, FsStruct);
                    user_init_process_getcwd_observed(self);
                }
            }

            on Action::SetUid {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_root_credentials_bound(self);
                    user_init_process_credentials_capability_model_deferred(self);
                    user_init_process_uid_set_observed(self);
                }
            }

            on Action::SetGid {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_root_credentials_bound(self);
                    user_init_process_credentials_capability_model_deferred(self);
                    user_init_process_gid_set_observed(self);
                }
            }

            on Action::RtSigprocmask {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_blocked_signal_mask_bound(self);
                    user_init_process_signal_delivery_deferred(self);
                    user_init_process_rt_sigprocmask_observed(self);
                }
            }

            on Action::RtSigaction {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_signal_runtime_bound(self);
                    user_init_process_signal_action_table_bound(self);
                    user_init_process_signal_action_table_layout_bound(self);
                    user_init_process_signal_delivery_deferred(self);
                    user_init_process_rt_sigaction_observed(self);
                }
            }

            on Action::RtSigtimedwait {
                depends_on {
                    UserInitProcess.state == State::Online;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_signal_runtime_bound(self);
                    user_init_process_thread_signal_state_bound(self);
                    user_init_process_pending_signal_set_empty_first_slice(self);
                    user_init_process_signal_delivery_deferred(self);
                    user_init_process_rt_sigtimedwait_observed(self);
                    user_init_process_rt_sigtimedwait_mask_observed(self);
                    user_init_process_rt_sigtimedwait_uinfo_null(self);
                    user_init_process_rt_sigtimedwait_uts_null(self);
                    user_init_process_rt_sigtimedwait_pending_match_empty(self);
                    user_init_process_rt_sigtimedwait_infinite_wait(self);
                    user_init_process_pending_sigchld_first_slice(self);
                    user_init_process_rt_sigtimedwait_waiter_enqueued(self);
                    user_init_process_rt_sigtimedwait_sleep_reason_bound(self);
                    user_init_process_rt_sigtimedwait_woken_by_sigchld(self);
                    user_init_process_rt_sigtimedwait_dequeued_sigchld(self);
                }
            }

        }
    }
}

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
                    KernelInitTask.state == State::Online;
                    PayloadExecSyncBoundaries.state == State::Ready;
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
            user_boot_payload_init_attempt_failure_trace_defined(self);
            payload_image_read_failed_checkpoint_defined(self);
            payload_image_read_error_classification_contract_ready(self);
            PayloadExecSyncBoundaries.state == State::Ready;
        }

        actions {
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
                    KernelInitTask.state == State::Online;
                    ExceptionStream.state == State::Ready;
                    SyscallException.state == State::Prepared;
                    PayloadExecSyncBoundaries.state == State::Ready;
                }

                drives {
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
                    SyscallException.Transition::Setup;
                    SyscallTable.Transition::Setup;
                    SyscallException.Transition::Enable;
                    FilesStruct.Transition::Setup;
                    FilesStruct.Action::ClearStdinReadyData;
                    FilesStruct.Action::PrepareDefaultStdinReadyData;
                    FilesStruct.Action::EnableStdinBlockingWait;
                    UserInitProcess.Transition::Setup;
                    UserInitProcess.Transition::Enable;
                    UserInitProcess.Action::EnterUserMode;
                }

                ensures {
                    ElfObject.state == State::Online;
                    UserAddressSpace.state == State::Online;
                    UserTrapFrame.state == State::Ready;
                    SyscallTable.state == State::Ready;
                    SyscallException.state == State::Online;
                    UserInitProcess.state == State::Online;
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
            user_boot_payload_reads_init_from_vfs(self, VfsCore);
            user_boot_payload_selected_path_bound(self);
            user_boot_payload_selected_argv0_path_bound(self);
            user_boot_payload_driven_by_kernel_init_task(self, KernelInitTask);
            user_boot_payload_enters_user_mode(self);
            user_boot_payload_no_return_handoff(self);
        }
    }
}
