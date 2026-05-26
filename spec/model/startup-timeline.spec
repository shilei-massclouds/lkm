/*
 * Startup Timeline Specification
 *
 * This file defines the current top-level timeline object and composes the
 * model phase tree. The formal directory entry is spec/model/main.spec.
 */

include "common.spec";
include "prepare/main.spec";
include "boot/main.spec";

/*
 * StartupTimeline 表示当前模型的内核启动时间轴对象。
 * 它临时编排准备期和当前已经展开的引导期阶段，并在内核启动完成后退出。
 */
object StartupTimeline: TimelineObject {
    initial_state: State::Base;

    /*
     * Base 表示顶层启动阶段对象已经进入模型空间，但尚未推进其子阶段。
     */
    state State::Base {
        events {
            /*
             * Setup 先推进准备期边界，再推进当前已经展开的引导期阶段。
             */
            on Event::Setup -> State::Ready {
                drives {
                    PreparePhase.Event::Setup;
                    PreparePhase.Event::Enable;
                    BootPhase.Event::Setup;
                }
            }
        }
    }

    /*
     * Ready 表示准备期边界已经生效，且当前已经展开的引导期阶段已经完成。
     */
    state State::Ready {
        invariant {
            PreparePhase.state == State::Online;
            BootPhase.state == State::Ready;
        }
    }
}
