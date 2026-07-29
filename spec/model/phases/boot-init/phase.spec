/*
 * BootInitFlow directly owns the boot execution leaves.  The former
 * BootPhase and InterruptPhase wrapper lifecycles are intentionally absent.
 */

include "preset.spec";
include "rest-init/main.spec";

object BootInitFlow: TaskFlow {
    lifecycle_override: true;
    initial_state: State::Base;
    parent: BootTask;

    processes {
        Action::BindBootTaskEntry(current_task_ref: TaskRef) {
            state_effect: StateEffect::None;
            depends_on {
                task_ref_ready(current_task_ref);
                task_ref_targets(current_task_ref, BootTask);
                cpu_active_translation_controller_for_ref_is(
                    BootCPURef,
                    TranslationControllerKind::PhysicalDirect
                ) || cpu_active_translation_controller_for_ref_is(
                    BootCPURef,
                    TranslationControllerKind::EarlyVm
                ) || cpu_active_translation_controller_for_ref_is(
                    BootCPURef,
                    TranslationControllerKind::SwapperVm
                );
                cpu_translation_controller_matches_live_satp_for_ref(BootCPURef);
            }
            ensures {
                boot_task_entry_bound_for_active_controller(
                    CpuGroup.cpus[0],
                    BootTask
                );
                boot_task_entry_preempt_count_initialized_once(BootTask);
                boot_task_entry_preempt_count_preserved(BootTask);
                boot_task_entry_binding_diagnostic_clear();
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
                    task_initial_flow_is(BootTask, self);
                    self.parent.state == State::OnCpu;
                    task_execution_authority_is(
                        self.parent,
                        TaskExecutionAuthority::Live
                    );
                    task_flow_start_binding_consistent(self);
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
                        KernelAddrSpace.Transition::Preset;
                        CurrentCPU.Transition::Setup(true);
                        CurrentCPU.trap.interrupt.Transition::Setup;
                        BootInitFlow.Action::BindBootTaskEntry(CurrentTaskRef);
                        BootInitStack.Transition::Preset;
                        CurrentCPU.trap.Transition::Preset;
                        CurrentCPU.trap.exception.Transition::Preset;
                        Vm.Transition::Preset;
                        Vm.Transition::Setup;
                        CurrentCPU.trap.Transition::Setup;
                        BootInitFlow.Action::BindBootTaskEntry(CurrentTaskRef);
                        BootInitStack.Transition::Setup;
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
                    BootInitStack.state == State::Ready;
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
                    task_owns_flow(BootTask, self);
                    task_flow_owner_is(self, BootTask);
                    task_flow_parent_is(self, BootTask);
                    task_flow_cpu_ref_is(self, BootCPURef);
                    task_flow_cpu_ref_targets(self, CpuGroup.cpus[0]);
                    current_cpu_resolved_target_is(self, BootCPURef, CpuGroup.cpus[0]);
                    task_flow_started(self);
                }

            }
        }
    }

    state State::Prepared {
        invariant {
            BootTask.state == State::OnCpu;
            task_flow_started(self);
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
                }

                within SingleTaskContext {
                    drives {
                        EntrySuccessorPhase.Transition::Preset;
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
                    EntrySuccessorPhase.state == State::Online;
                    CorePreparePhase.state == State::Online;
                    MmCoreInitPhase.state == State::Online;
                    SchedInitPhase.state == State::Online;
                    IrqTimeInitPhase.state == State::Online;
                    LocalIrqEnablePhase.state == State::Online;
                    IrqOpenPreparePhase.state == State::Online;
                    ProcessPreparePhase.state == State::Online;
                    BootInitRestInitPhase.state == State::Online;
                    KernelInitFlow.state == State::Base;
                    KthreaddFlow.state == State::Base;
                    BootTask.state == State::OnCpu;
                }

            }
        }
    }

    state State::Ready {
        invariant {
            ProcessPreparePhase.state == State::Online;
            BootInitRestInitPhase.state == State::Online;
            KernelInitFlow.state == State::Base;
            KthreaddFlow.state == State::Base;
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
                    BootIdleFlow.state == State::Ready;
                    task_owns_flow(BootTask, BootIdleFlow);
                    task_flow_owner_is(BootIdleFlow, BootTask);
                    task_flow_parent_is(BootIdleFlow, BootTask);
                    task_flow_owner_exclusive(BootIdleFlow);
                    task_active_flow_is(BootTask, BootIdleFlow);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    kernel_init_flow_first_leaf(KernelInitFlow, PreSmpInitPhase);
                    kernel_init_entry_reaches_kernel_init_flow(KernelInitTask, KernelInitFlow);
                    boot_init_flow_switch_precommit_ready(
                        BootInitFlow,
                        Scheduler,
                        BootTask,
                        KernelInitTask
                    );
                    scheduler_switch_to_prepared(
                        Scheduler,
                        BootRunQueue,
                        BootTaskRef,
                        KernelInitTaskRef
                    );
                    BootTask.state == State::OnCpu;
                    task_flow_online_on_cpu(self);
                    task_has_unique_active_flow(self.parent);
                }
            }
        }
    }

    state State::Online {
        invariant {
            ProcessPreparePhase.state == State::Online;
            BootInitRestInitPhase.state == State::Online;
            BootInitScheduleHandoffPhase.state == State::Online;
            BootIdleFlow.state == State::Ready;
            task_active_flow_is(BootTask, BootIdleFlow);
            boot_init_flow_switch_precommit_ready(
                BootInitFlow,
                Scheduler,
                BootTask,
                KernelInitTask
            );
            scheduler_switch_to_prepared(
                Scheduler,
                BootRunQueue,
                BootTaskRef,
                KernelInitTaskRef
            );
            BootTask.state == State::OnCpu;
            task_flow_started(self);
            task_flow_online_on_cpu(self);
        }
    }
}

predicate boot_init_flow_switch_precommit_ready<B, S, T, K>(
    boot_init: B,
    scheduler: S,
    boot_task: T,
    kernel_init_task: K
) -> bool;
