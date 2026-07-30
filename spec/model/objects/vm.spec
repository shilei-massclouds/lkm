/* Vm translation control-plane model specification. */

/* Vm 是控制面协调对象，不是地址空间资源。 */
object Vm: KernelObject {
    initial_state: State::Base;
    parent: Kernel;

    state State::Base {
        transitions {
            /* 准备早期布局与页表 controller，但保持当前 CPU 继续由 PhysicalDirect 承载。 */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    PhysicalDirect.state == State::Ready;
                    TrampolineVm.state == State::Base;
                    EarlyVm.state == State::Base;
                    KernelAddrSpace.state == State::Base;
                }

                drives {
                    KernelAddrSpace.Transition::Preset;
                    TrampolineVm.Transition::Setup;
                    EarlyVm.Transition::Preset;
                    KernelAddrSpace.Transition::Setup;
                    EarlyVm.Transition::Setup;
                }

                ensures {
                    riscv_early_boot_alternatives_deferred(Vm);
                    riscv_early_boot_alternatives_mmu_off_boundary_preserved(Vm);
                }

                deferred entry_vm.001 {
                    category: DeferredCategory::Protocol;
                    summary: "Model and implement early RISC-V alternatives/errata text patching in the MMU-off setup_vm window.";
                    evidence {
                        riscv_early_boot_alternatives_deferred(Vm);
                        riscv_early_boot_alternatives_mmu_off_boundary_preserved(Vm);
                    }
                    close_when: "Alternative selection, MMU-off patch ordering, synchronization and reference tests pass.";
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            PhysicalDirect.state == State::Ready;
            TrampolineVm.state == State::Ready;
            EarlyVm.state == State::Ready;
            KernelAddrSpace.state == State::Ready;
            riscv_early_boot_alternatives_deferred(Vm);
            riscv_early_boot_alternatives_mmu_off_boundary_preserved(Vm);
        }

        transitions {
            /* 依次把 BootCPU 从 PhysicalDirect 交给 TrampolineVm 和 EarlyVm，并恢复保护入口。 */
            on Transition::Setup -> State::Ready {
                drives {
                    TrampolineVm.Action::ActivateOnCpu(BootCPURef);
                    EarlyVm.Action::ActivateOnCpu(BootCPURef);
                    KernelImage.Transition::Enable;
                }

                ensures {
                    cpu_active_translation_controller_is(CpuGroup.cpus[0], TranslationControllerKind::EarlyVm);
                    cpu_active_translation_controller_for_ref_is(BootCPURef, TranslationControllerKind::EarlyVm);
                    BootCpuRegisters.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
                    early_vm_translation_sync_complete(EarlyVm, CpuGroup.cpus[0]);
                    vm_transition_stvec_released_to_trap(BootCpuRegisters.stvec, CurrentCPU.trap);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            PhysicalDirect.state == State::Ready;
            TrampolineVm.state == State::Ready;
            EarlyVm.state == State::Ready;
            SwapperVm.state == State::Base;
            KernelAddrSpace.state == State::Ready;
            KernelImage.state == State::Online;
            cpu_active_translation_controller_is(CpuGroup.cpus[0], TranslationControllerKind::EarlyVm);
            BootCpuRegisters.satp == satp_of(EarlyVm.pg_dir, Config.satp_mode);
            early_vm_translation_sync_complete(EarlyVm, CpuGroup.cpus[0]);
        }

        transitions {
            on Transition::Enable -> State::Online {
                drives {
                    SwapperVm.Transition::Setup;
                    KernelAddrSpace.Transition::Enable;
                    SwapperVm.Action::ActivateOnCpu(BootCPURef);
                }

                ensures {
                    cpu_active_translation_controller_is(CpuGroup.cpus[0], TranslationControllerKind::SwapperVm);
                    cpu_active_translation_controller_for_ref_is(BootCPURef, TranslationControllerKind::SwapperVm);
                    BootCpuRegisters.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
                    swapper_vm_translation_sync_complete(SwapperVm, CpuGroup.cpus[0]);
                }
            }
        }
    }

    state State::Online {
        invariant {
            PhysicalDirect.state == State::Ready;
            TrampolineVm.state == State::Ready;
            EarlyVm.state == State::Ready;
            SwapperVm.state == State::Ready;
            KernelAddrSpace.state == State::Online;
            cpu_active_translation_controller_is(CpuGroup.cpus[0], TranslationControllerKind::SwapperVm);
            BootCpuRegisters.satp == satp_of(SwapperVm.pg_dir, Config.satp_mode);
            swapper_vm_translation_sync_complete(SwapperVm, CpuGroup.cpus[0]);
        }
    }
}
