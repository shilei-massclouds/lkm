/*
 * First user-mode program bootstrap model.
 *
 * This slice models the shortest Linux-like path from PayloadPhase to the
 * first user-mode program. The selected payload variant is UserBootPayload.
 * It reads an init candidate such as /sbin/init from the current VFS root,
 * treats the result as an ElfObject, builds a PT_LOAD mapping plan, maps that
 * plan into a UserAddressSpace, prepares a UserStack and UserTrapFrame, then
 * enters U-mode. Syscalls remain under the existing SyscallException branch of
 * ExceptionStream. SyscallException owns syscall entry validation, argument
 * extraction and dispatch selection; this slice only adds SyscallTable as the
 * minimal action table consumed by that branch.
 *
 * The current root disk image is a whole-disk ext2 filesystem. PartitionTable
 * and BlockPartition objects are intentionally deferred until the disk image
 * format actually includes a partition table.
 */

enum UserInitPathRef {
    DefaultInit,
}

enum ElfObjectRole {
    MainExecutable,
    Interpreter,
}

predicate user_boot_payload_selected<T>(payload: T) -> bool;
predicate user_boot_payload_candidates_bound<T>(payload: T) -> bool;
predicate user_boot_payload_default_init_path_bound<T>(payload: T) -> bool;
predicate user_boot_payload_uses_current_fs_struct<T, F>(payload: T, fs: F) -> bool;
predicate user_boot_payload_reads_init_from_vfs<T, V>(payload: T, vfs: V) -> bool;
predicate user_boot_payload_no_partition_dependency<T>(payload: T) -> bool;
predicate user_boot_payload_partition_objects_deferred<T>(payload: T) -> bool;
predicate user_boot_payload_driven_by_kernel_init_task<T, K>(payload: T, task: K) -> bool;
predicate user_boot_payload_try_candidate_bound<T>(payload: T) -> bool;
predicate user_boot_payload_selected_path_bound<T>(payload: T) -> bool;
predicate user_boot_payload_try_candidate_read_init<T, V>(payload: T, vfs: V) -> bool;
predicate user_boot_payload_try_candidate_elf_ready<T, E>(payload: T, elf: E) -> bool;
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
predicate elf_object_interpreter_required<T>(elf: T) -> bool;
predicate elf_object_interpreter_path_bound<T>(elf: T) -> bool;
predicate elf_object_interpreter_elf_bound<T, I>(elf: T, interpreter: I) -> bool;
predicate elf_object_et_dyn_interpreter_supported<T>(elf: T) -> bool;
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
predicate user_trap_frame_sret_ready<T>(frame: T) -> bool;
predicate user_trap_frame_address_space_bound<T, A>(frame: T, space: A) -> bool;
predicate user_trap_frame_prepared_but_not_entered<T>(frame: T) -> bool;

predicate syscall_table_ready<T>(table: T) -> bool;
predicate syscall_table_bound_to_exception<T, E>(table: T, exception: E) -> bool;
predicate syscall_table_write_supported<T>(table: T) -> bool;
predicate syscall_table_writev_supported<T>(table: T) -> bool;
predicate syscall_table_openat_supported<T>(table: T) -> bool;
predicate syscall_table_read_supported<T>(table: T) -> bool;
predicate syscall_table_close_supported<T>(table: T) -> bool;
predicate syscall_table_newfstatat_supported<T>(table: T) -> bool;
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
predicate syscall_path_usercopy_ready<T>(table: T) -> bool;
predicate syscall_stat_usercopy_ready<T>(table: T) -> bool;
predicate syscall_write_routes_to_console<T>(table: T) -> bool;
predicate syscall_writev_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_openat_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_read_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_close_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_newfstatat_routes_to_files_struct<T, F>(table: T, files: F) -> bool;
predicate syscall_brk_routes_to_user_address_space<T, A>(table: T, space: A) -> bool;
predicate syscall_mmap_routes_to_user_address_space<T, A>(table: T, space: A) -> bool;
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
predicate syscall_table_close_observed<T>(table: T) -> bool;
predicate syscall_table_newfstatat_observed<T>(table: T) -> bool;
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
predicate user_init_process_clear_child_tid_bound<T>(process: T) -> bool;
predicate user_init_process_user_entry_ready<T>(process: T) -> bool;
predicate user_init_process_trap_return_bound<T, R>(process: T, frame: R) -> bool;
predicate kernel_init_task_execve_to_user_init<K, T>(task: K, process: T) -> bool;
predicate kernel_init_task_pid1_identity_preserved<K>(task: K) -> bool;
predicate kernel_init_task_user_mm_attached<K, A>(task: K, space: A) -> bool;
predicate kernel_init_task_user_trap_frame_attached<K, R>(task: K, frame: R) -> bool;

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
                     */
                    elf_object_static_executable(self) || elf_object_dynamic_executable(self);
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
                    syscall_table_close_supported(self);
                    syscall_table_newfstatat_supported(self);
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
                    syscall_path_usercopy_ready(self);
                    syscall_stat_usercopy_ready(self);
                    syscall_write_routes_to_console(self);
                    syscall_exit_records_status(self);
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
            syscall_table_close_supported(self);
            syscall_table_newfstatat_supported(self);
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
            syscall_path_usercopy_ready(self);
            syscall_stat_usercopy_ready(self);
            syscall_write_routes_to_console(self);
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
                depends_on {
                    SyscallException.state == State::Online;
                    FilesStruct.state == State::Ready;
                    FileDescriptorTable.state == State::Ready;
                    fd_table_fd_bound(FileDescriptorTable, FdRef::Regular0, OpenFileDescription);
                    syscall_read_usercopy_ready(self);
                }

                drives {
                    FilesStruct.Action::ReadFd(FdRef::Regular0);
                }

                ensures {
                    syscall_read_routes_to_files_struct(self, FilesStruct);
                    files_struct_regular_file_read_observed(FilesStruct);
                    open_file_description_read_observed(OpenFileDescription);
                    file_backend_regular_file_read_returns_data(FileBackend);
                    syscall_table_read_observed(self);
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
                    user_boot_payload_uses_current_fs_struct(self, FsStruct);
                    user_boot_payload_no_partition_dependency(self);
                    user_boot_payload_partition_objects_deferred(self);
                    user_boot_payload_driven_by_kernel_init_task(self, KernelInitTask);
                    user_boot_payload_try_candidate_bound(self);
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
            user_boot_payload_uses_current_fs_struct(self, FsStruct);
            user_boot_payload_no_partition_dependency(self);
            user_boot_payload_partition_objects_deferred(self);
            user_boot_payload_driven_by_kernel_init_task(self, KernelInitTask);
            user_boot_payload_try_candidate_bound(self);
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
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    PayloadParam.state == State::Ready;
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
                    UserBootPayload.Action::TryCandidate(UserInitPathRef::DefaultInit);
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
            user_boot_payload_driven_by_kernel_init_task(self, KernelInitTask);
            user_boot_payload_enters_user_mode(self);
            user_boot_payload_no_return_handoff(self);
        }
    }
}
