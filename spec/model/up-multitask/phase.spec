/*
 * UP Multitask Phase Specification
 *
 * This top-level phase starts after InterruptPhase has completed the
 * ProcessPreparePhase boundary and before the selected PayloadPhase handoff.
 * The currently expanded subphase is RestInitPhase, which covers rest_init().
 */

include "rest-init/main.spec";

/*
 * UpMultitaskPhase 表示单核多任务期阶段对象。当前只展开 rest_init 子阶段；
 * 后续 PreSmpInitPhase 将继续挂在本阶段下。
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
