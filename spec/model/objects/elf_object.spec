/* ELF artifact and load-plan lifecycle shared by all executable owners. */

enum ElfObjectRole {
    MainExecutable,
    Interpreter,
}

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
