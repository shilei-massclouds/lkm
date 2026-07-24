/*
 * BootInitFlow directly owns the boot execution leaves.  The former
 * BootPhase and InterruptPhase wrapper lifecycles are intentionally absent.
 */

include "preset.spec";
include "rest-init/main.spec";

object BootInitFlow: TaskFlow {
    initial_state: State::Base;
    parent: BootTask;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
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
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    context_is(SystemExclusive);
                }

                may_change {
                    BootCpuRegisters.sstatus;
                }

                within SingleTaskContext {
                    drives {
                        InterruptStream.Transition::Preset;
                        KernelImage.Transition::Preset;
                        KernelImage.Transition::Setup;
                        BootCurrentCPU.Transition::Preset;
                        BootCurrentCPU.Transition::Setup;
                        CpuGroup.Transition::Preset;
                        BootCurrentCPU.Transition::Enable;
                        BootTaskEntryBinding.Transition::Preset;
                        BootInitStack.Transition::Preset;
                        EventStream.Transition::Preset;
                        ExceptionStream.Transition::Preset;
                        Vm.Transition::Preset;
                        Vm.Transition::Setup;
                        EventStream.Transition::Setup;
                        BootTaskEntryBinding.Transition::Setup;
                        BootInitStack.Transition::Setup;
                        Soc.Transition::Preset;
                    }
                }

                ensures {
                    interrupt_concurrency_closed();
                    task_concurrency_closed();
                    context_is(SystemExclusive);
                    kernel_fpu_disabled(BootCpuRegisters.sstatus);
                    kernel_vector_disabled(BootCpuRegisters.sstatus);
                    InterruptStream.state == State::Prepared;
                    EventStream.state == State::Ready;
                    ExceptionStream.state == State::Prepared;
                    PageFaultException.state == State::Prepared;
                    SyscallException.state == State::Prepared;
                    BreakpointException.state == State::Prepared;
                    UnexpectedException.state == State::Prepared;
                    KernelImage.state == State::Online;
                    RawDtb.state == State::Ready;
                    BootTaskEntryBinding.state == State::Ready;
                    BootTask.state == State::OnCpu;
                    BootInitStack.state == State::Ready;
                    Vm.state == State::Ready;
                    TrampolineVm.state == State::Destroyed;
                    EarlyVm.state == State::Online;
                    BootCurrentCPU.state == State::Online;
                    BootCPU.state == State::Prepared;
                    CpuGroup.state == State::Prepared;
                    Soc.state == State::Prepared;
                    task_ref_targets(BootTaskRef, BootTask);
                    task_ref_ready(BootTaskRef);
                    task_owns_flow(BootTask, self);
                    task_flow_owner_is(self, BootTask);
                    task_flow_parent_is(self, BootTask);
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

                within SingleTaskInterruptStreamContext {
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
                        CurrentTaskRef,
                        KernelInitTaskRef
                    );
                    BootTask.state == State::OnCpu;
                }

                emits {
                    Kernel.Transition::Setup;
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
                CurrentTaskRef,
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
