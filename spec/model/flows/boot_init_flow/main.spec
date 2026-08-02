/* BootInitFlow model entry and complete root object. */

include "preset.spec";
include "setup.spec";
include "enable.spec";

object BootInitFlow: TaskFlow {
    lifecycle_override: true;
    initial_state: State::Base;
    parent: BootTask;

    processes {
        /*
         * 把内核启动时的第一个参数作为BootCPU的hartid记录下来，以备后续使用。
         */
        Action::RecordBootCpuHartid {
            state_effect: StateEffect::None;
            depends_on {
                BootCpuRegisters.a0 == BootArgs.boot_hartid;
            }
            ensures {
                boot_cpu_hartid_recorded_for_later_use(BootCpuRegisters.a0);
            }
        }

        Action::PrepareIdleEntry {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                BootTask.state == State::OnCpu;
                task_flow_effective_execution_guard(self);
            }
            ensures {
                boot_idle_runtime_ready(self, BootTask);
                boot_idle_cpu_startup_entry_ready(self, CpuGroup.cpus[0]);
                boot_idle_entry_prepared(self, BootTask);
            }
        }

        Action::RunIdleLoop {
            state_effect: StateEffect::None;
            depends_on {
                self.state == State::Online;
                BootTask.state == State::OnCpu;
                boot_idle_entry_prepared(self, BootTask);
            }
            ensures {
                boot_idle_runtime_loop_entered(self, BootTask);
                boot_idle_loop_cycle_committed(self);
                boot_idle_loop_continues(self);
            }
        }

    }

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Kernel.state == State::Ready;
                    kernel_enable_accepted(Kernel);
                    Riscv64.state == State::Online;
                    SbiSpec.state == State::Online;
                    OpenSBI.state == State::Online;
                    BootCpuRegisters.state == State::Online;
                    Lds.state == State::Online;
                    Config.state == State::Online;
                    task_breakpoint_state_is(
                        BootTask,
                        TaskBreakpointState::Invalid
                    );
                    task_fixed_flow_is(BootTask, self);
                    self.parent.state == State::OnCpu;
                    task_execution_authority_is(
                        self.parent,
                        TaskExecutionAuthority::Live
                    );
                    task_concurrency_closed();
                    BootCpuRegisters.satp == 0;
                    cpu_active_translation_controller_for_ref_is(
                        BootCPURef,
                        TranslationControllerKind::PhysicalDirect
                    );
                }

                within SingleTaskContext {
                    drives {
                        CurrentCPU.trap.interrupt.Transition::Preset;
                        KernelImage.Transition::Preset;
                        CurrentCPU.Action::DisableFpuVectorExecution;
                        KernelImage.Transition::Setup;
                        BootInitFlow.Action::RecordBootCpuHartid;
                        CurrentTask.Action::BindTaskStack(BootTask, BootTask.stack);
                        CurrentCPU.Transition::Setup(true);
                        CurrentCPU.trap.interrupt.Transition::Setup;
                        CurrentCPU.trap.Transition::Preset;
                        Vm.Transition::Preset;
                        Vm.Transition::Setup;
                        CurrentCPU.trap.Transition::Setup;
                        CurrentTask.Action::RefreshTaskStack(BootTask, BootTask.stack);
                        Soc.Transition::Preset;
                    }
                }

                ensures {
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    context_is(SystemExclusive);
                    cpu_fpu_execution_disabled(CpuGroup.cpus[0]);
                    cpu_vector_execution_disabled(CpuGroup.cpus[0]);
                    cpu_kernel_fpu_vector_default_disabled(CpuGroup.cpus[0]);
                    cpu_kernel_fpu_vector_temporary_enable_requires_controlled_scope(CpuGroup.cpus[0]);
                    cpu_kernel_fpu_vector_disabled_after_controlled_scope(CpuGroup.cpus[0]);
                    cpu_user_fpu_vector_enable_follows_task_need_and_system_policy(CpuGroup.cpus[0]);
                    boot_cpu_hartid_recorded_for_later_use(BootCpuRegisters.a0);
                    CurrentCPU.trap.interrupt.state == State::Ready;
                    CurrentCPU.trap.state == State::Ready;
                    CurrentCPU.trap.exception.state == State::Prepared;
                    CurrentCPU.trap.exception.page_fault.state == State::Prepared;
                    CurrentCPU.trap.exception.syscall.state == State::Prepared;
                    CurrentCPU.trap.exception.breakpoint.state == State::Prepared;
                    CurrentCPU.trap.exception.unexpected.state == State::Prepared;
                    KernelImage.state == State::Online;
                    RawDtb.state == State::Ready;
                    KernelAddrSpace.state == State::Ready;
                    BootTask.state == State::OnCpu;
                    Vm.state == State::Ready;
                    TrampolineVm.state == State::Ready;
                    EarlyVm.state == State::Ready;
                    cpu_active_translation_controller_for_ref_is(
                        BootCPURef,
                        TranslationControllerKind::EarlyVm
                    );
                    CpuGroup.cpus[0].state == State::Ready;
                    CpuGroup.state == State::Prepared;
                    Soc.state == State::Prepared;
                    task_ref_targets(BootTaskRef, BootTask);
                    task_ref_ready(BootTaskRef);
                    boot_task_current_binding_established(CpuGroup.cpus[0], BootTask);
                    boot_task_current_binding_refreshed_for_active_controller(CpuGroup.cpus[0], BootTask);
                    boot_task_stack_current_binding_established(CpuGroup.cpus[0], BootTask, BootTask.stack);
                    boot_task_stack_current_binding_refreshed_for_active_controller(CpuGroup.cpus[0], BootTask, BootTask.stack);
                    current_task_stack_binding_pair_consistent(CpuGroup.cpus[0], BootTask, BootTask.stack);
                    current_stack_binding_matches_task(CpuGroup.cpus[0], BootTask, BootTask.stack);
                    current_stack_pointer_matches_active_controller(CpuGroup.cpus[0], BootTask, BootTask.stack);
                    boot_task_preemption_is_static_initial_property(BootTask);
                    task_stack_is_static_initial_property(BootTask, BootTask.stack);
                    task_stack_range_is(
                        BootTask,
                        BootTask.stack,
                        Lds.init_stack_start,
                        Lds.init_stack_end
                    );
                    task_has_unique_stack_attribute(BootTask);
                    boot_task_task_stack_binding_diagnostic_clear();
                    task_flow_owner_is(self, BootTask);
                    task_flow_parent_is(self, BootTask);
                    task_flow_cpu_ref_is(self, BootCPURef);
                    task_flow_cpu_ref_targets(self, CpuGroup.cpus[0]);
                    current_cpu_resolved_target_is(self, BootCPURef, CpuGroup.cpus[0]);
                    task_flow_started(self);
                    task_flow_start_binding_consistent(self);
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            BootTask.state == State::OnCpu;
            task_flow_started(self);
            boot_cpu_hartid_recorded_for_later_use(BootCpuRegisters.a0);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Kernel.state == State::Ready;
                    kernel_enable_accepted(Kernel);
                    self.parent.state == State::OnCpu;
                    task_execution_authority_is(
                        self.parent,
                        TaskExecutionAuthority::Live
                    );
                    Vm.state == State::Ready;
                    EarlyVm.state == State::Ready;
                    cpu_active_translation_controller_for_ref_is(
                        BootCPURef,
                        TranslationControllerKind::EarlyVm
                    );
                    current_stack_binding_matches_task(
                        CpuGroup.cpus[0],
                        BootTask,
                        BootTask.stack
                    );
                    current_stack_pointer_matches_active_controller(
                        CpuGroup.cpus[0],
                        BootTask,
                        BootTask.stack
                    );
                    CurrentCPU.trap.interrupt.state == State::Ready;
                    RawDtb.state == State::Ready;
                    FixMap.state == State::Ready;
                    KernelImage.state == State::Online;
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                }

                within SingleTaskContext {
                    drives {
                        BootTask.Action::EnableStackGuard;
                        EarlyDtb.Transition::Preset;
                        CurrentCPU.Transition::Enable;
                        PrintkBuffer.Transition::Preset;
                        EarlyDtb.Transition::Setup;
                        InitMM.Transition::Setup;
                        EarlyIoremap.Transition::Setup;
                        SBI.Transition::Setup;
                        Params.Transition::Preset;
                        MemBlock.Transition::Setup;
                        Vm.Transition::Enable;
                        MemBlock.Transition::Enable;
                        EarlyDtb.Transition::Cleanup;
                        DeviceTree.Transition::Setup;
                        Zones.Transition::Setup;
                        PageMetadataMap.Transition::Setup;
                        ResourceLock.Transition::Preset;
                        ResourceLock.Transition::Setup;
                        ResourceTree.Transition::Setup;
                        CpuGroup.Transition::Setup;
                        CacheBlockInfo.Transition::Setup;
                        CpuCapabilities.Transition::Setup;
                        DmaCachePolicy.Transition::Setup;
                        CorePreparePhase.Transition::Preset;
                        MmCoreInitPhase.Transition::Preset;
                        SchedInitPhase.Transition::Preset;
                        IrqTimeInitPhase.Transition::Preset;
                    }
                }

                drives {
                    LocalIrqEnablePhase.Transition::Preset;
                }

                within SingleTaskInterruptContext {
                    drives {
                        IrqOpenPreparePhase.Transition::Preset;
                        ProcessPreparePhase.Transition::Preset;
                    }
                }

                drives {
                    BootInitRestInitPhase.Transition::Preset;
                }

                ensures {
                    efi_boot_init_deferred(BootInitFlow);
                    boot_init_setup_start_kernel_position_preserved(BootInitFlow);
                    boot_init_setup_debug_objects_trimmed(BootInitFlow);
                    boot_init_setup_vmlinux_build_id_trimmed(BootInitFlow);
                    boot_init_setup_page_address_init_trimmed(BootInitFlow);
                    boot_init_setup_cgroup_early_trimmed(BootInitFlow);
                    boot_init_setup_acpi_boot_tables_trimmed(BootInitFlow);
                    boot_init_setup_early_memtest_input_trimmed(BootInitFlow);
                    boot_init_setup_sparse_init_trimmed(BootInitFlow);
                    boot_init_setup_vmemmap_tlb_flush_trimmed(BootInitFlow);
                    boot_init_setup_crashkernel_trimmed(BootInitFlow);
                    boot_init_setup_kasan_trimmed(BootInitFlow);
                    boot_init_setup_acpi_rintc_trimmed(BootInitFlow);
                    boot_init_setup_acpi_cpu_numa_trimmed(BootInitFlow);
                    boot_init_setup_cbop_block_size_deferred(BootInitFlow);
                    boot_init_setup_boot_alternatives_deferred(BootInitFlow);
                    boot_init_setup_rt_signal_env_deferred(BootInitFlow);
                    boot_init_setup_user_isa_deferred(BootInitFlow);
                    CorePreparePhase.state == State::Online;
                    MmCoreInitPhase.state == State::Online;
                    SchedInitPhase.state == State::Online;
                    IrqTimeInitPhase.state == State::Online;
                    LocalIrqEnablePhase.state == State::Online;
                    IrqOpenPreparePhase.state == State::Online;
                    ProcessPreparePhase.state == State::Online;
                    BootInitRestInitPhase.state == State::Online;
                    KernelInitFlow.state == State::Online;
                    KthreaddFlow.state == State::Online;
                    BootTask.state == State::OnCpu;
                }

                deferred boot_init_setup.001 {
                    category: DeferredCategory::AlternatePath;
                    summary: "Model the enabled EFI boot initialization path as a firmware interface object.";
                    evidence { efi_boot_init_deferred(BootInitFlow); }
                    close_when: "EFI initialization, handoff facts and enabled-reference-path tests pass.";
                }
                trimmed boot_init_setup.002 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "init_vmlinux_build_id is an inline no-op because STACKTRACE_BUILD_ID and VMCORE_INFO are disabled.";
                    evidence { boot_init_setup_vmlinux_build_id_trimmed(BootInitFlow); }
                    revisit_when: "The reference configuration enables CONFIG_STACKTRACE_BUILD_ID or CONFIG_VMCORE_INFO.";
                }
                trimmed boot_init_setup.003 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "page_address_init expands to no work without HASHED_PAGE_VIRTUAL or WANT_PAGE_VIRTUAL.";
                    evidence { boot_init_setup_page_address_init_trimmed(BootInitFlow); }
                    revisit_when: "The reference architecture enables hashed or explicit page virtual-address metadata.";
                }
                trimmed boot_init_setup.004 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "debug_objects_early_init is absent because CONFIG_DEBUG_OBJECTS=n.";
                    evidence { boot_init_setup_debug_objects_trimmed(BootInitFlow); }
                    revisit_when: "The reference configuration enables CONFIG_DEBUG_OBJECTS.";
                }
                trimmed boot_init_setup.005 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "cgroup_init_early is absent because CONFIG_CGROUPS=n.";
                    evidence { boot_init_setup_cgroup_early_trimmed(BootInitFlow); }
                    revisit_when: "The reference configuration enables CONFIG_CGROUPS.";
                }
                trimmed boot_init_setup.006 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "acpi_boot_table_init is absent because CONFIG_ACPI=n.";
                    evidence { boot_init_setup_acpi_boot_tables_trimmed(BootInitFlow); }
                    revisit_when: "The reference configuration enables CONFIG_ACPI.";
                }
                trimmed boot_init_setup.007 {
                    category: TrimmedCategory::ReferenceInput;
                    summary: "early_memtest has no work because the fixed reference command line has no memtest request.";
                    evidence { boot_init_setup_early_memtest_input_trimmed(BootInitFlow); }
                    revisit_when: "The reference boot arguments request an early memory test.";
                }
                trimmed boot_init_setup.008 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "sparse_init is a no-op under CONFIG_FLATMEM=y and CONFIG_SPARSEMEM=n.";
                    evidence { boot_init_setup_sparse_init_trimmed(BootInitFlow); }
                    revisit_when: "The reference configuration selects sparse memory.";
                }
                trimmed boot_init_setup.009 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "The VMEMMAP kernel-range TLB flush path is unreachable without SPARSEMEM_VMEMMAP.";
                    evidence { boot_init_setup_vmemmap_tlb_flush_trimmed(BootInitFlow); }
                    revisit_when: "The reference configuration selects SPARSEMEM_VMEMMAP.";
                }
                trimmed boot_init_setup.010 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "Crashkernel reservation is absent with KEXEC/crash support disabled.";
                    evidence { boot_init_setup_crashkernel_trimmed(BootInitFlow); }
                    revisit_when: "The reference configuration enables crashkernel support.";
                }
                trimmed boot_init_setup.011 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "kasan_init is absent because CONFIG_KASAN=n.";
                    evidence { boot_init_setup_kasan_trimmed(BootInitFlow); }
                    revisit_when: "The reference configuration enables CONFIG_KASAN.";
                }
                trimmed boot_init_setup.012 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "ACPI RINTC mapping is absent because CONFIG_ACPI=n.";
                    evidence { boot_init_setup_acpi_rintc_trimmed(BootInitFlow); }
                    revisit_when: "The reference configuration enables CONFIG_ACPI.";
                }
                trimmed boot_init_setup.013 {
                    category: TrimmedCategory::BuildConfig;
                    summary: "ACPI CPU-to-NUMA mapping is absent because ACPI and NUMA are disabled.";
                    evidence { boot_init_setup_acpi_cpu_numa_trimmed(BootInitFlow); }
                    revisit_when: "The reference configuration enables ACPI or NUMA CPU topology.";
                }
                deferred boot_init_setup.014 {
                    category: DeferredCategory::ModelDetail;
                    summary: "Publish and validate the RISC-V CBOP block-size binding.";
                    evidence { boot_init_setup_cbop_block_size_deferred(BootInitFlow); }
                    close_when: "CBOP discovery, validation and implementation tests cover supported platform data.";
                }
                deferred boot_init_setup.015 {
                    category: DeferredCategory::Protocol;
                    summary: "Model and implement the generic boot alternatives text-patch protocol.";
                    evidence { boot_init_setup_boot_alternatives_deferred(BootInitFlow); }
                    close_when: "Alternative patch selection, ordering and synchronization match the reference path.";
                }
                deferred boot_init_setup.016 {
                    category: DeferredCategory::Feature;
                    summary: "Initialize the user real-time signal environment.";
                    evidence { boot_init_setup_rt_signal_env_deferred(BootInitFlow); }
                    close_when: "RT signal environment state and user-visible tests pass.";
                }
                deferred boot_init_setup.017 {
                    category: DeferredCategory::Feature;
                    summary: "Expose the supported RISC-V user ISA capabilities.";
                    evidence { boot_init_setup_user_isa_deferred(BootInitFlow); }
                    close_when: "User ISA exposure is modeled and validated against the reference capability set.";
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            ProcessPreparePhase.state == State::Online;
            BootInitRestInitPhase.state == State::Online;
            KernelInitFlow.state == State::Online;
            KthreaddFlow.state == State::Online;
            BootTask.state == State::OnCpu;
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    Kernel.state == State::Ready;
                    kernel_enable_accepted(Kernel);
                    self.parent.state == State::OnCpu;
                    task_execution_authority_is(
                        self.parent,
                        TaskExecutionAuthority::Live
                    );
                }

                drives {
                    BootInitScheduleHandoffPhase.Transition::Preset;
                }

                ensures {
                    BootInitScheduleHandoffPhase.state == State::Online;
                    task_fixed_flow_is(BootTask, self);
                    task_fixed_flow_is(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
                    boot_init_flow_switch_precommit_ready(
                        BootInitFlow,
                        Cpu0Scheduler,
                        BootTask,
                        KernelInitTask
                    );
                    scheduler_switch_to_prepared(
                        Cpu0Scheduler,
                        Cpu0Scheduler,
                        BootTaskRef,
                        KernelInitTaskRef
                    );
                    BootTask.state == State::OnCpu;
                }

                emits {
                    self.Action::RequestSchedule;
                }
            }
        }
    }

    state State::Online {
        invariant {
            ProcessPreparePhase.state == State::Online;
            BootInitRestInitPhase.state == State::Online;
            BootInitScheduleHandoffPhase.state == State::Online;
            task_fixed_flow_is(BootTask, self);
            boot_init_flow_switch_precommit_ready(
                BootInitFlow,
                Cpu0Scheduler,
                BootTask,
                KernelInitTask
            );
            scheduler_switch_to_prepared(
                Cpu0Scheduler,
                Cpu0Scheduler,
                BootTaskRef,
                KernelInitTaskRef
            );
            task_flow_started(self);
        }

        actions {
            on Action::RequestSchedule {
                state_effect: StateEffect::None;
                depends_on {
                    BootTask.state == State::OnCpu;
                    Cpu0Scheduler.state == State::Online;
                    task_flow_effective_execution_guard(self);
                }
                yields {
                    Cpu0Scheduler.Action::Schedule;
                }
                drives {
                    BootIdleEntryPhase.Transition::Preset;
                }
                ensures {
                    boot_init_flow_schedule_returned(self, Cpu0Scheduler);
                    BootIdleEntryPhase.state == State::Online;
                }
            }
        }
    }
}
