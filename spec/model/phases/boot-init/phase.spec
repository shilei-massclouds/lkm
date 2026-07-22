/*
 * Boot Init Flow Phase Specification
 *
 * BootInitFlow is the TaskFlow instance initially associated with BootTask.
 * BootTask becomes Online at `_start`, then its post-commit lossy signal is
 * accepted because BootDispatchWindow still names BootTaskRef. Because
 * TaskFlow is a PhaseObject subtype, this flow uses the standard phase lifecycle while spanning the kernel entry
 * execution fragment through the pre-commit boundary of the first real
 * BootTask -> KernelInitTask switch.
 */

include "rest-init/main.spec";

object BootInitFlow: TaskFlow {
    initial_state: State::Base;
    parent: BootTask;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    BootTask.state == State::Online;
                    task_initial_flow_is(BootTask, self);
                    task_flow_initial_binding_consistent(self);
                    task_flow_dispatch_guard_satisfied(self, BootDispatchWindow);
                }

                within SingleTaskContext {
                    drives {
                        EntryPreludePhase.Transition::Preset;
                    }
                }

                ensures {
                    EntryPreludePhase.state == State::Online;
                    BootTask.state == State::Online;
                    task_flow_started(self);
                    task_owns_flow(BootTask, self);
                    task_flow_owner_is(self, BootTask);
                    task_flow_parent_is(self, BootTask);
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            EntryPreludePhase.state == State::Online;
            BootTask.state == State::Online;
            task_flow_started(self);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    task_flow_dispatch_guard_satisfied(self, BootDispatchWindow);
                }

                drives {
                    BootPhase.Transition::Preset;
                    InterruptPhase.Transition::Preset;
                }

                ensures {
                    BootPhase.state == State::Online;
                    InterruptPhase.state == State::Online;
                    BootTask.state == State::Online;
                }

                emits {
                    Transition::Enable;
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
                depends_on {
                    task_flow_dispatch_guard_satisfied(self, BootDispatchWindow);
                }

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
                    task_flow_parent_is(BootIdleFlow, BootTask);
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
                    task_flow_online_on_dispatch(self, BootDispatchWindow);
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
            task_flow_started(self);
            task_flow_online_on_dispatch(self, BootDispatchWindow);
        }
    }
}

predicate boot_init_flow_switch_precommit_ready<B, S, T, K>(
    boot_init: B,
    scheduler: S,
    boot_task: T,
    kernel_init_task: K
) -> bool;
