/*
 * UP Multitask Phase Specification
 *
 * This top-level phase starts after InterruptPhase has completed the
 * ProcessPreparePhase boundary and ends in the rest_init() boot idle branch.
 * The expanded path is split by execution owner:
 * BootInitRestInitPhase creates PID 1/kthreadd and completes kthreadd_done,
 * BootInitScheduleHandoffPhase commits the first scheduler handoff from the
 * BootInitTask perspective, and BootIdleEntryPhase enters the boot idle
 * continuation owned by BootIdleTask.
 */

include "rest-init/main.spec";

/*
 * UpMultitaskPhase 表示 rest_init() 所在的单核多任务启动分支。
 * 这三个子阶段分别属于 BootInitTask 的 rest_init 前半段、BootInitTask
 * 的首次 schedule handoff 点，以及 BootIdleTask 的 idle 入口。旧
 * RestInitPhase 仅可作为兼容 wrapper，不再是跨 owner 的最小子阶段。
 */
object UpMultitaskPhase: PhaseObject {
    initial_state: State::Base;
    parent: StartupTimeline;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    InterruptPhase.state == State::Ready;
                    ProcessPreparePhase.state == State::Ready;
                }

                drives {
                    BootInitRestInitPhase.Transition::Setup;
                    BootInitScheduleHandoffPhase.Transition::Setup;
                    BootIdleEntryPhase.Transition::Setup;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            InterruptPhase.state == State::Ready;
            ProcessPreparePhase.state == State::Ready;
            BootInitRestInitPhase.state == State::Ready;
            BootInitScheduleHandoffPhase.state == State::Ready;
            BootIdleEntryPhase.state == State::Ready;
        }
    }
}
