/*
 * UP Multitask Phase Specification
 *
 * This top-level phase starts after InterruptPhase has completed the
 * ProcessPreparePhase boundary and before the selected PayloadPhase handoff.
 * The currently expanded subphases are RestInitPhase and PreSmpInitPhase.
 * RestInitPhase expands schedule_preempt_disabled() into a preemption guard
 * exit, Scheduler.Action::Schedule, and a BootIdleStartupContext that enters
 * the boot idle runtime loop and its conditional schedule_idle boundary;
 * PreSmpInitPhase starts from the resulting KernelInitTask dispatch facts
 * rather than from RestInitPhase.Ready.
 */

include "rest-init/main.spec";
include "pre-smp-init/main.spec";

/*
 * UpMultitaskPhase 表示单核多任务期阶段对象。RestInitPhase 和
 * PreSmpInitPhase 在 Scheduler 首次调度交接事实处分叉，并在本阶段 Ready 汇合。
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
                    PreSmpInitPhase.Event::Setup;
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
            PreSmpInitPhase.state == State::Ready;
        }
    }
}
