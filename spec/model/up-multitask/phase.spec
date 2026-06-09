/*
 * UP Multitask Phase Specification
 *
 * This top-level phase starts after InterruptPhase has completed the
 * ProcessPreparePhase boundary and ends in the rest_init() boot idle branch.
 * The currently expanded subphase is RestInitPhase.
 * RestInitPhase expands schedule_preempt_disabled() into a preemption guard
 * exit, Scheduler.Action::Schedule, and a BootIdleStartupContext that enters
 * the boot idle runtime loop and its conditional schedule_idle boundary.
 * Scheduler.Action::Schedule also publishes the KernelInitTask dispatch facts
 * that start the separate SmpRuntimePhase execution line.
 */

include "rest-init/main.spec";

/*
 * UpMultitaskPhase 表示 rest_init() 所在的单核多任务启动分支。
 * RestInitPhase 是本阶段唯一子阶段；它在 Scheduler 首次调度边界 fork
 * 出 KernelInitTask/SmpRuntimePhase 执行线后，继续完成 boot idle tail。
 */
object UpMultitaskPhase: PhaseObject {
    initial_state: State::Base;
    parent: StartupTimeline;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    InterruptPhase.state == State::Ready;
                    ProcessPreparePhase.state == State::Ready;
                }

                drives {
                    RestInitPhase.Event::Preset;
                    RestInitPhase.Event::Setup;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            InterruptPhase.state == State::Ready;
            ProcessPreparePhase.state == State::Ready;
            RestInitPhase.state == State::Ready;
        }
    }
}
