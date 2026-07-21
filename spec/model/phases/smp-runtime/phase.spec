/*
 * SMP Runtime Phase Specification
 *
 * KernelInitTask owns this BP execution line from the real task-stack entry
 * through FinalizePhase. SmpBringupPhase coordinates AP-owned subphases, but
 * those AP execution lines are not direct children of SmpRuntimePhase.
 */

include "pre-smp-init/main.spec";
include "smp-bringup/main.spec";
include "runtime-core/main.spec";
include "initcall/main.spec";
include "rootfs/main.spec";
include "finalize/main.spec";

object SmpRuntimePhase: PhaseObject {
    initial_state: State::Base;
    parent: Kernel;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    UpMultitaskPhase.state == State::Online;
                    KernelInitTask.state == State::Online;
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    task_flow_first_phase(KernelInitTask, SmpRuntimePhase);
                    kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                }

                drives {
                    PreSmpInitPhase.Transition::Preset;
                }

                ensures {
                    PreSmpInitPhase.state == State::Online;
                    kernel_init_task_owns_smp_runtime_mainline(
                        KernelInitTask,
                        SmpRuntimePhase
                    );
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    PreSmpInitPhase.state == State::Online;
                }

                drives {
                    SmpBringupPhase.Transition::Preset;
                }

                ensures {
                    SmpBringupPhase.state == State::Online;
                    kernel_init_task_owns_smp_runtime_mainline(
                        KernelInitTask,
                        SmpRuntimePhase
                    );
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            UpMultitaskPhase.state == State::Online;
            PreSmpInitPhase.state == State::Online;
            SmpBringupPhase.state == State::Online;
            KernelInitTask.state == State::Online;
            kernel_init_task_owns_smp_runtime_mainline(KernelInitTask, SmpRuntimePhase);
        }

        transitions {
            on Transition::Enable -> State::Online {
                drives {
                    RuntimeCorePhase.Transition::Preset;
                    InitcallPhase.Transition::Preset;
                    RootfsPhase.Transition::Preset;
                    FinalizePhase.Transition::Preset;
                }

                ensures {
                    RuntimeCorePhase.state == State::Online;
                    InitcallPhase.state == State::Online;
                    RootfsPhase.state == State::Online;
                    FinalizePhase.state == State::Online;
                }
            }
        }
    }

    state State::Online {
        invariant {
            UpMultitaskPhase.state == State::Online;
            PreSmpInitPhase.state == State::Online;
            SmpBringupPhase.state == State::Online;
            RuntimeCorePhase.state == State::Online;
            InitcallPhase.state == State::Online;
            RootfsPhase.state == State::Online;
            FinalizePhase.state == State::Online;
            KernelInitTask.state == State::Online;
            kernel_init_task_owns_smp_runtime_mainline(KernelInitTask, SmpRuntimePhase);
        }
    }
}
