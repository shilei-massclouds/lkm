/*
 * Boot Init Flow Phase Specification
 *
 * BootInitFlow is a PhaseObject child of the statically Online BootTask. It
 * spans the kernel entry execution fragment through the pre-commit boundary
 * of the first real BootTask -> KernelInitTask switch. It is not a TaskFlow.
 */

include "rest-init/main.spec";

object BootInitFlow: PhaseObject {
    initial_state: State::Base;
    parent: BootTask;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    BootTask.state == State::Online;
                }

                within SingleTaskContext {
                    drives {
                        EntryPreludePhase.Transition::Preset;
                    }
                }

                ensures {
                    EntryPreludePhase.state == State::Online;
                    BootTask.state == State::Online;
                }

            }
        }
    }

    state State::Prepared {
        invariant {
            EntryPreludePhase.state == State::Online;
            BootTask.state == State::Online;
        }

        transitions {
            on Transition::Setup -> State::Ready {
                drives {
                    BootPhase.Transition::Preset;
                    InterruptPhase.Transition::Preset;
                }

                ensures {
                    BootPhase.state == State::Online;
                    InterruptPhase.state == State::Online;
                    BootTask.state == State::Online;
                }

            }
        }
    }

    state State::Ready {
        invariant {
            EntryPreludePhase.state == State::Online;
            BootPhase.state == State::Online;
            InterruptPhase.state == State::Online;
            BootTask.state == State::Online;
        }

        transitions {
            on Transition::Enable -> State::Online {
                drives {
                    BootInitRestInitPhase.Transition::Preset;
                    BootInitScheduleHandoffPhase.Transition::Preset;
                }

                ensures {
                    BootInitRestInitPhase.state == State::Online;
                    BootInitScheduleHandoffPhase.state == State::Online;
                    BootIdleFlow.state == State::Ready;
                    task_owns_flow(BootTask, BootIdleFlow);
                    task_flow_owner_is(BootIdleFlow, BootTask);
                    task_flow_owner_exclusive(BootIdleFlow);
                    task_active_flow_is(BootTask, BootIdleFlow);
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    task_flow_first_phase(KernelInitTask, SmpRuntimePhase);
                    kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
                    boot_init_flow_switch_precommit_ready(
                        BootInitFlow,
                        Scheduler,
                        BootTask,
                        KernelInitTask
                    );
                    BootTask.state == State::Online;
                }
            }
        }
    }

    state State::Online {
        invariant {
            EntryPreludePhase.state == State::Online;
            BootPhase.state == State::Online;
            InterruptPhase.state == State::Online;
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
            BootTask.state == State::Online;
        }
    }
}

predicate boot_init_flow_switch_precommit_ready<B, S, T, K>(
    boot_init: B,
    scheduler: S,
    boot_task: T,
    kernel_init_task: K
) -> bool;
