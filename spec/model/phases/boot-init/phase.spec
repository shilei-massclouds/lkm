/*
 * BootInitFlow directly owns the boot execution leaves.  The former
 * BootPhase and InterruptPhase wrapper lifecycles are intentionally absent.
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
                    task_flow_start_signal_discarded(KernelInitFlow, BootDispatchWindow);
                    task_flow_start_signal_discarded(KthreaddFlow, BootDispatchWindow);
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
            ProcessPreparePhase.state == State::Online;
            BootInitRestInitPhase.state == State::Online;
            KernelInitFlow.state == State::Base;
            KthreaddFlow.state == State::Base;
            BootTask.state == State::Online;
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    task_flow_dispatch_guard_satisfied(self, BootDispatchWindow);
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
                        CurrentTaskRef,
                        KernelInitTaskRef
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
