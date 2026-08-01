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
                    task_owns_flow(BootTask, self);
                    task_flow_owner_is(self, BootTask);
                    task_flow_parent_is(self, BootTask);
                    task_flow_cpu_ref_is(self, BootCPURef);
                    task_flow_cpu_ref_targets(self, CpuGroup.cpus[0]);
                    current_cpu_resolved_target_is(self, BootCPURef, CpuGroup.cpus[0]);
                    task_flow_started(self);
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
                    BootIdleFlow.Transition::Enable;
                }

                ensures {
                    BootInitScheduleHandoffPhase.state == State::Online;
                    BootIdleFlow.state == State::Online;
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
                    task_flow_online_on_cpu(self);
                    task_has_unique_active_flow(self.parent);
                }

                emits {
                    BootIdleFlow.Action::RequestSchedule;
                }
            }
        }
    }

    state State::Online {
        invariant {
            ProcessPreparePhase.state == State::Online;
            BootInitRestInitPhase.state == State::Online;
            BootInitScheduleHandoffPhase.state == State::Online;
            BootIdleFlow.state == State::Online;
            task_active_flow_is(BootTask, BootIdleFlow);
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
            task_flow_started(self);
            task_flow_online_on_cpu(self);
        }
    }
}

predicate boot_cpu_hartid_recorded_for_later_use(hartid: HartId) -> bool;

predicate boot_init_flow_switch_precommit_ready<B, S, T, K>(
    boot_init: B,
    scheduler: S,
    boot_task: T,
    kernel_init_task: K
) -> bool;
