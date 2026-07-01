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
predicate user_boot_payload_user_smoke_stdin_fixture_only<T>(payload: T) -> bool;
predicate user_boot_payload_distro_init_no_stdin_fixture<T>(payload: T) -> bool;
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
predicate syscall_table_setuid_supported<T>(table: T) -> bool;
predicate syscall_table_setgid_supported<T>(table: T) -> bool;
predicate syscall_table_rt_sigprocmask_supported<T>(table: T) -> bool;
predicate syscall_table_rt_sigaction_supported<T>(table: T) -> bool;
predicate syscall_table_clock_gettime_supported<T>(table: T) -> bool;
predicate syscall_table_gettimeofday_supported<T>(table: T) -> bool;
predicate syscall_table_nanosleep_supported<T>(table: T) -> bool;
predicate syscall_table_brk_supported<T>(table: T) -> bool;
predicate syscall_table_mmap_supported<T>(table: T) -> bool;
predicate syscall_table_mprotect_supported<T>(table: T) -> bool;
predicate syscall_table_munmap_supported<T>(table: T) -> bool;
predicate syscall_table_set_tid_address_supported<T>(table: T) -> bool;
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
predicate syscall_read_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_read_stdin_ready_data_first_slice<T>(table: T) -> bool;
predicate syscall_read_tty_blocking_deferred<T>(table: T) -> bool;
predicate syscall_ppoll_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_ppoll_pollfd_usercopy_ready<T>(table: T) -> bool;
predicate syscall_ppoll_ready_data_first_slice<T>(table: T) -> bool;
predicate syscall_ppoll_timeout_parse_first_slice<T>(table: T) -> bool;
predicate syscall_ppoll_sigmask_deferred<T>(table: T) -> bool;
predicate syscall_ppoll_blocking_wait_deferred<T>(table: T) -> bool;
predicate syscall_close_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_newfstatat_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_readlinkat_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_fcntl_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_ioctl_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_ioctl_usercopy_ready<T>(table: T) -> bool;
predicate syscall_ioctl_tcgets_termios_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tcsets_termios_mutation_first_slice<T>(table: T) -> bool;
predicate syscall_ioctl_tty_full_linux_model_deferred<T>(table: T) -> bool;
predicate syscall_getrandom_routes_to_hwrng_core<T, H>(table: T, hwrng: H) -> bool;
predicate syscall_getrandom_not_vfs_or_devfs_path<T>(table: T) -> bool;
predicate syscall_getrandom_flags_first_slice_bound<T>(table: T) -> bool;
predicate syscall_getrandom_full_random_core_deferred<T>(table: T) -> bool;
predicate syscall_getuid_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
predicate syscall_getgid_routes_to_user_init_process<T, P>(table: T, process: P) -> bool;
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
predicate syscall_exit_records_status<T>(table: T) -> bool;
predicate syscall_exception_dispatches_via_table<T, S>(exception: T, table: S) -> bool;
predicate syscall_exception_extracts_arguments<T>(exception: T) -> bool;
predicate user_trap_return_ready() -> bool;
predicate user_trap_return_switches_satp() -> bool;
predicate user_trap_return_sfence_vma_after_satp() -> bool;
predicate user_trap_return_sret_handoff() -> bool;
predicate user_trap_return_context_used<T, R>(process: T, frame: R) -> bool;
predicate user_trap_entry_uses_kernel_stack<T>(frame: T) -> bool;
predicate syscall_table_write_observed<T>(table: T) -> bool;
predicate syscall_table_writev_observed<T>(table: T) -> bool;
predicate syscall_table_openat_observed<T>(table: T) -> bool;
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
predicate syscall_table_setuid_observed<T>(table: T) -> bool;
predicate syscall_table_setgid_observed<T>(table: T) -> bool;
predicate syscall_table_rt_sigprocmask_observed<T>(table: T) -> bool;
predicate syscall_table_rt_sigaction_observed<T>(table: T) -> bool;
predicate syscall_table_clock_gettime_observed<T>(table: T) -> bool;
predicate syscall_table_gettimeofday_observed<T>(table: T) -> bool;
predicate syscall_table_nanosleep_observed<T>(table: T) -> bool;
predicate syscall_table_set_tid_address_observed<T>(table: T) -> bool;
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
predicate user_init_process_signal_delivery_deferred<T>(process: T) -> bool;
predicate user_init_process_clear_child_tid_bound<T>(process: T) -> bool;
predicate user_init_process_uid_read_observed<T>(process: T) -> bool;
predicate user_init_process_gid_read_observed<T>(process: T) -> bool;
predicate user_init_process_uid_set_observed<T>(process: T) -> bool;
predicate user_init_process_gid_set_observed<T>(process: T) -> bool;
predicate user_init_process_rt_sigprocmask_observed<T>(process: T) -> bool;
predicate user_init_process_rt_sigaction_observed<T>(process: T) -> bool;
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
                    syscall_table_setuid_supported(self);
                    syscall_table_setgid_supported(self);
                    syscall_table_rt_sigprocmask_supported(self);
                    syscall_table_rt_sigaction_supported(self);
                    syscall_table_clock_gettime_supported(self);
                    syscall_table_gettimeofday_supported(self);
                    syscall_table_nanosleep_supported(self);
                    syscall_table_brk_supported(self);
                    syscall_table_mmap_supported(self);
                    syscall_table_mprotect_supported(self);
                    syscall_table_munmap_supported(self);
                    syscall_table_set_tid_address_supported(self);
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
                    syscall_ppoll_timeout_parse_first_slice(self);
                    syscall_ppoll_sigmask_deferred(self);
                    syscall_ppoll_blocking_wait_deferred(self);
                    syscall_ioctl_tty_full_linux_model_deferred(self);
                    syscall_nanosleep_full_hrtimer_deferred(self);
                    syscall_exit_records_status(self);
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
            syscall_table_setuid_supported(self);
            syscall_table_setgid_supported(self);
            syscall_table_rt_sigprocmask_supported(self);
            syscall_table_rt_sigaction_supported(self);
            syscall_table_clock_gettime_supported(self);
            syscall_table_gettimeofday_supported(self);
            syscall_table_nanosleep_supported(self);
            syscall_table_brk_supported(self);
            syscall_table_mmap_supported(self);
            syscall_table_mprotect_supported(self);
            syscall_table_munmap_supported(self);
            syscall_table_set_tid_address_supported(self);
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
            syscall_ppoll_timeout_parse_first_slice(self);
            syscall_ppoll_sigmask_deferred(self);
            syscall_ppoll_blocking_wait_deferred(self);
            syscall_ioctl_tty_full_linux_model_deferred(self);
            syscall_nanosleep_full_hrtimer_deferred(self);
            syscall_exit_records_status(self);
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
                    files_struct_regular_fd_installed(FilesStruct);
                    fd_table_fd_installed(FileDescriptorTable, FdRef::Regular0, OpenFileDescription);
                    syscall_table_openat_observed(self);
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
                 * Regular0 read-only file, and fd0 char-device stdin only when
                 * the TTY side already contains bounded ready data. Blocking
                 * wait queues, canonical line discipline, job control, signal
                 * interruption/restart, poll/ppoll and real RX wakeup remain
                 * deferred. A user-read-trace probe may observe fd, requested
                 * length and the returned read result for distro debugging, but
                 * must not alter this action's return value, errno path,
                 * checkpoint ordering or smoke pass/fail policy.
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
                    syscall_read_tty_blocking_deferred(self);
                    tty_n_tty_blocking_read_deferred(TtyFlipBuffer);
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
                 * This first slice only observes immediately available
                 * readiness from the existing fd table and char-device ready
                 * data. It parses and validates the optional timeout but does
                 * not sleep, update a remaining timeout, install a temporary
                 * signal mask, restart after signal delivery, or implement
                 * N_TTY wait queues / real RX wakeup.
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
                }

                ensures {
                    syscall_ppoll_routes_to_files_struct(self, FilesStruct);
                    files_struct_fd_lookup_routes_to_table(FilesStruct, FileDescriptorTable);
                    syscall_ppoll_ready_data_first_slice(self);
                    syscall_ppoll_timeout_parse_first_slice(self);
                    syscall_ppoll_sigmask_deferred(self);
                    syscall_ppoll_blocking_wait_deferred(self);
                    tty_n_tty_blocking_read_deferred(TtyFlipBuffer);
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
                 * This first slice covers F_GETFL, F_GETFD and F_SETFD. The
                 * close-on-exec bit lives in FileDescriptorTable state, not in
                 * struct file status flags.
                 */
                depends_on {
                    SyscallException.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                }

                drives {
                    FilesStruct.Action::LookupFd(FdRef::Regular0);
                    FilesStruct.Action::GetFdFlags(FdRef::Regular0);
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
                 * The current slice only covers console-like char-device fds,
                 * the riscv64/generic 36-byte old struct termios, TCGETS
                 * readback and TCSETS immediate mutation. TCSETSW/TCSETSF,
                 * drain/flush, driver and line-discipline set_termios hooks,
                 * canonical N_TTY behavior and real TTY locking remain
                 * deferred.
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
                }

                ensures {
                    syscall_ioctl_routes_to_files_struct(self, FilesStruct);
                    files_struct_fd_lookup_routes_to_table(FilesStruct, FileDescriptorTable);
                    syscall_ioctl_tcgets_termios_first_slice(self);
                    syscall_ioctl_tcsets_termios_mutation_first_slice(self);
                    files_struct_tty_termios_state_bound(FilesStruct);
                    files_struct_tty_termios_mutation_observed(FilesStruct);
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
                    syscall_table_exit_observed(self);
                }
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
                    user_init_process_signal_delivery_deferred(self);
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
            user_init_process_signal_delivery_deferred(self);
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
                    user_init_process_syscall_context_bound(self, SyscallException, SyscallTable);
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
            user_init_process_signal_delivery_deferred(self);
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
                    FilesStruct.Action::PrepareDefaultStdinReadyData;
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
                    user_boot_payload_user_smoke_stdin_fixture_only(self);
                    user_boot_payload_distro_init_no_stdin_fixture(self);
                    files_struct_stdin_ready_data_bound(FilesStruct);
                    tty_flip_buffer_ready_data_bound(TtyFlipBuffer);
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
