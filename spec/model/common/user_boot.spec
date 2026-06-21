/*
 * First user-mode program bootstrap model.
 *
 * This slice models the shortest Linux-like path from PayloadPhase to the
 * first user-mode program. The selected payload variant is UserBootPayload.
 * It reads an init candidate such as /init from the current VFS root, treats
 * the result as an ElfObject, builds a PT_LOAD mapping plan, maps that plan
 * into a UserAddressSpace, prepares a UserStack and UserTrapFrame, then enters
 * U-mode. Syscalls remain under the existing SyscallException branch of
 * ExceptionStream; this file only adds the minimal dispatcher/table objects
 * consumed by that branch.
 *
 * The current root disk image is a whole-disk ext2 filesystem. PartitionTable
 * and BlockPartition objects are intentionally deferred until the disk image
 * format actually includes a partition table.
 */

enum UserInitPathRef {
    DefaultInit,
}

enum UserSyscallRef {
    Write,
    Exit,
    ExitGroup,
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

predicate elf_object_input_bound<T>(elf: T) -> bool;
predicate elf_object_input_from_vfs<T, V>(elf: T, vfs: V) -> bool;
predicate elf_object_magic_valid<T>(elf: T) -> bool;
predicate elf_object_class_elf64<T>(elf: T) -> bool;
predicate elf_object_little_endian<T>(elf: T) -> bool;
predicate elf_object_machine_riscv<T>(elf: T) -> bool;
predicate elf_object_type_supported<T>(elf: T) -> bool;
predicate elf_object_static_executable<T>(elf: T) -> bool;
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
predicate user_address_space_stack_mapped<T, S>(space: T, stack: S) -> bool;
predicate user_address_space_elf_load_plan_consumed<T, E>(space: T, elf: E) -> bool;
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
predicate swapper_vm_remains_kernel_shared_instance<T>(swapper: T) -> bool;

predicate user_stack_allocated<T>(stack: T) -> bool;
predicate user_stack_fixed_size_bound<T>(stack: T) -> bool;
predicate user_stack_mapped_into_address_space<T, A>(stack: T, space: A) -> bool;
predicate user_stack_backing_pages_allocated<T>(stack: T) -> bool;
predicate user_stack_zeroed<T>(stack: T) -> bool;
predicate user_stack_initial_sp_bound<T>(stack: T) -> bool;
predicate user_stack_minimal_arg_env_bound<T>(stack: T) -> bool;

predicate user_trap_frame_allocated<T>(frame: T) -> bool;
predicate user_trap_frame_entry_bound<T, E>(frame: T, elf: E) -> bool;
predicate user_trap_frame_sp_bound<T, S>(frame: T, stack: S) -> bool;
predicate user_trap_frame_sstatus_user_mode<T>(frame: T) -> bool;
predicate user_trap_frame_sret_ready<T>(frame: T) -> bool;
predicate user_trap_frame_address_space_bound<T, A>(frame: T, space: A) -> bool;
predicate user_trap_frame_prepared_but_not_entered<T>(frame: T) -> bool;

predicate syscall_dispatcher_ready<T>(dispatcher: T) -> bool;
predicate syscall_dispatcher_bound_to_exception<T, E>(dispatcher: T, exception: E) -> bool;
predicate syscall_table_ready<T>(table: T) -> bool;
predicate syscall_table_bound_to_dispatcher<T, D>(table: T, dispatcher: D) -> bool;
predicate syscall_table_write_supported<T>(table: T) -> bool;
predicate syscall_table_exit_supported<T>(table: T) -> bool;
predicate syscall_table_exit_group_supported<T>(table: T) -> bool;
predicate syscall_write_usercopy_ready<T>(table: T) -> bool;
predicate syscall_write_routes_to_console<T>(table: T) -> bool;
predicate syscall_exit_records_status<T>(table: T) -> bool;
predicate user_trap_return_ready() -> bool;
predicate user_trap_return_switches_satp() -> bool;
predicate user_trap_entry_uses_kernel_stack<T>(frame: T) -> bool;
predicate user_syscall_write_observed<T>(dispatcher: T) -> bool;
predicate user_syscall_exit_observed<T>(dispatcher: T) -> bool;
predicate user_mode_entry_observed<T>(frame: T) -> bool;

predicate user_init_process_online<T>(process: T) -> bool;
predicate user_init_process_reuses_kernel_init_task<T, K>(process: T, task: K) -> bool;
predicate user_init_process_path_bound<T, P>(process: T, path: P) -> bool;
predicate user_init_process_address_space_bound<T, A>(process: T, space: A) -> bool;

object UserAddressSpace: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
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

        events {
            on Event::Setup -> State::Ready {
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
            elf_object_mapped_to_user_address_space(ElfObject, self);
            elf_object_bss_zeroed(ElfObject);
        }

        events {
            on Event::Enable -> State::Online {
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
        }
    }
}

object UserStack: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
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
        }
    }
}

object ElfObject: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Preset -> State::Prepared {
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
                    elf_object_static_executable(self);
                    elf_object_no_separate_loader(self);
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
            elf_object_static_executable(self);
            elf_object_no_separate_loader(self);
        }

        events {
            on Event::Setup -> State::Ready {
                ensures {
                    elf_object_program_headers_parsed(self);
                    elf_object_pt_load_segments_bound(self);
                    elf_object_segment_permissions_bound(self);
                    elf_object_load_plan_bound(self);
                    elf_object_entry_in_executable_segment(self);
                    elf_object_init_content_observed(self);
                    elf_object_bss_zero_plan_bound(self);
                    elf_object_entry_bound(self);
                    elf_object_load_merged_into_setup(self);
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
            elf_object_no_separate_loader(self);
        }

        events {
            on Event::Enable -> State::Online {
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
        events {
            on Event::Setup -> State::Ready {
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
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    ConsoleRegistry.state == State::Ready;
                }

                ensures {
                    syscall_table_ready(self);
                    syscall_table_write_supported(self);
                    syscall_table_exit_supported(self);
                    syscall_table_exit_group_supported(self);
                    syscall_write_usercopy_ready(self);
                    syscall_write_routes_to_console(self);
                    syscall_exit_records_status(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            syscall_table_ready(self);
            syscall_table_write_supported(self);
            syscall_table_exit_supported(self);
            syscall_table_exit_group_supported(self);
            syscall_write_usercopy_ready(self);
            syscall_write_routes_to_console(self);
            syscall_exit_records_status(self);
        }
    }
}

object SyscallDispatcher: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    SyscallException.state == State::Ready;
                    SyscallTable.state == State::Ready;
                }

                ensures {
                    syscall_dispatcher_ready(self);
                    syscall_dispatcher_bound_to_exception(self, SyscallException);
                    syscall_table_bound_to_dispatcher(SyscallTable, self);
                    syscall_table_ready();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            syscall_dispatcher_ready(self);
            syscall_dispatcher_bound_to_exception(self, SyscallException);
            syscall_table_bound_to_dispatcher(SyscallTable, self);
            syscall_table_ready();
        }
    }
}

object UserInitProcess: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelInitTask.state == State::Online;
                    UserAddressSpace.state == State::Online;
                    ElfObject.state == State::Online;
                }

                ensures {
                    user_init_process_reuses_kernel_init_task(self, KernelInitTask);
                    user_init_process_path_bound(self, UserInitPathRef::DefaultInit);
                    user_init_process_address_space_bound(self, UserAddressSpace);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            user_init_process_reuses_kernel_init_task(self, KernelInitTask);
            user_init_process_path_bound(self, UserInitPathRef::DefaultInit);
            user_init_process_address_space_bound(self, UserAddressSpace);
        }

        events {
            on Event::Enable -> State::Online {
                depends_on {
                    UserTrapFrame.state == State::Ready;
                    SyscallDispatcher.state == State::Ready;
                    SyscallException.state == State::Online;
                }

                ensures {
                    user_init_process_online(self);
                    user_mode_entry_observed(UserTrapFrame);
                    user_trap_return_switches_satp();
                    user_trap_entry_uses_kernel_stack(UserTrapFrame);
                    user_syscall_write_observed(SyscallDispatcher);
                    user_syscall_exit_observed(SyscallDispatcher);
                }
            }
        }
    }

    state State::Online {
        invariant {
            user_init_process_online(self);
            user_init_process_reuses_kernel_init_task(self, KernelInitTask);
            user_init_process_address_space_bound(self, UserAddressSpace);
            user_mode_entry_observed(UserTrapFrame);
            user_syscall_write_observed(SyscallDispatcher);
            user_syscall_exit_observed(SyscallDispatcher);
        }
    }
}

object UserBootPayload: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    PayloadParam.state == State::Ready;
                    RootFS.state == State::Online;
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    KernelInitTask.state == State::Online;
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
        }

        actions {
            on Action::TryCandidate(path: UserInitPathRef) {
                depends_on {
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    ElfObject.state == State::Ready;
                }

                ensures {
                    user_boot_payload_try_candidate_read_init(self, VfsCore);
                    user_boot_payload_try_candidate_elf_ready(self, ElfObject);
                    user_boot_payload_selected_path_bound(self);
                }
            }
        }

        events {
            on Event::Enable -> State::Online {
                depends_on {
                    PayloadParam.state == State::Ready;
                    RootFS.state == State::Online;
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    ExceptionStream.state == State::Ready;
                    SyscallException.state == State::Prepared;
                }

                drives {
                    VfsCore.Action::ReadPath(Path, FsStruct);
                    ElfObject.Event::Preset;
                    ElfObject.Event::Setup;
                    UserBootPayload.Action::TryCandidate(UserInitPathRef::DefaultInit);
                    UserAddressSpace.Event::Preset;
                    UserStack.Event::Setup;
                    UserAddressSpace.Event::Setup;
                    UserTrapFrame.Event::Setup;
                    ElfObject.Event::Enable;
                    UserAddressSpace.Event::Enable;
                    SyscallException.Event::Setup;
                    SyscallTable.Event::Setup;
                    SyscallDispatcher.Event::Setup;
                    SyscallException.Event::Enable;
                    UserInitProcess.Event::Setup;
                    UserInitProcess.Event::Enable;
                }

                ensures {
                    ElfObject.state == State::Online;
                    UserAddressSpace.state == State::Online;
                    UserTrapFrame.state == State::Ready;
                    SyscallDispatcher.state == State::Ready;
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
