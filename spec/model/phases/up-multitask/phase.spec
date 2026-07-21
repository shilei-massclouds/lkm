/*
 * UP Multitask Phase Specification
 *
 * This top-level phase starts after InterruptPhase is Online and ends after
 * the rest_init() boot idle branch is Online.
 * The expanded path is split by execution owner:
 * BootInitRestInitPhase creates PID 1/kthreadd and completes kthreadd_done,
 * BootInitScheduleHandoffPhase commits the first scheduler handoff from the
 * BootTask perspective, and BootIdleEntryPhase enters the boot idle
 * continuation owned by BootTask. No RestInitPhase wrapper object is part
 * of the formal phase tree.
 */

include "rest-init/main.spec";

/*
 * UpMultitaskPhase 表示 rest_init() 所在的单核多任务启动分支。
 * 这三个子阶段分别属于 BootTask 的 rest_init 前半段、BootTask
 * 的首次 schedule handoff 点，以及 BootTask 的 idle 入口。规格不再建立
 * RestInitPhase 兼容 wrapper，避免后续依赖一个不对应真实 Linux 控制流边界
 * 或单一执行主体的阶段对象。
 */
object UpMultitaskPhase: PhaseObject {
    initial_state: State::Base;
    parent: Kernel;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    InterruptPhase.state == State::Online;
                }

                drives {
                    BootInitRestInitPhase.Transition::Preset;
                }

                ensures {
                    BootInitRestInitPhase.state == State::Online;
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    task_flow_first_phase(KernelInitTask, SmpRuntimePhase);
                    kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
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
                    BootInitRestInitPhase.state == State::Online;
                }

                drives {
                    BootInitScheduleHandoffPhase.Transition::Preset;
                }

                ensures {
                    BootInitScheduleHandoffPhase.state == State::Online;
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    task_flow_first_phase(KernelInitTask, SmpRuntimePhase);
                    kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            InterruptPhase.state == State::Online;
            BootInitRestInitPhase.state == State::Online;
            BootInitScheduleHandoffPhase.state == State::Online;
            task_owns_flow(KernelInitTask, KernelInitFlow);
            task_flow_first_phase(KernelInitTask, SmpRuntimePhase);
            kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
            kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
            scheduler_first_schedule_committed(Scheduler);
        }

        transitions {
            on Transition::Enable -> State::Online {
                drives {
                    BootIdleEntryPhase.Transition::Preset;
                }

                ensures {
                    BootIdleEntryPhase.state == State::Online;
                    task_owns_flow(KernelInitTask, KernelInitFlow);
                    task_flow_first_phase(KernelInitTask, SmpRuntimePhase);
                    kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                    kernel_init_task_stack_switch_committed(
                        Scheduler,
                        BootTask,
                        KernelInitTask
                    );
                }
            }
        }
    }

    state State::Online {
        invariant {
            InterruptPhase.state == State::Online;
            BootInitRestInitPhase.state == State::Online;
            BootInitScheduleHandoffPhase.state == State::Online;
            BootIdleEntryPhase.state == State::Online;
            task_owns_flow(KernelInitTask, KernelInitFlow);
            task_flow_first_phase(KernelInitTask, SmpRuntimePhase);
            kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
            kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
            scheduler_first_schedule_committed(Scheduler);
            kernel_init_task_stack_switch_committed(
                Scheduler,
                BootTask,
                KernelInitTask
            );
        }
    }
}
