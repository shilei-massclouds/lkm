/*
 * Kernel Specification
 *
 * This file defines the current top-level timeline object and composes the
 * model phase tree. The formal directory entry is spec/model/main.spec.
 */

include "objects/main.spec";
include "prepare/main.spec";
include "phases/boot/main.spec";
include "phases/interrupt/main.spec";
include "phases/up-multitask/main.spec";
include "phases/smp-runtime/main.spec";
include "phases/payload/main.spec";

/*
 * StartupTimeline 表示当前模型的内核启动时间轴对象。
 * 它临时编排准备期、当前已经展开的引导期/中断期阶段，以及启动末尾的 payload 交接阶段。
 */
object StartupTimeline: TimelineObject {
    initial_state: State::Base;

    /*
     * Base 表示顶层启动阶段对象已经进入模型空间，但尚未推进其子阶段。
     */
    state State::Base {
        transitions {
            /*
             * Setup 先推进准备期边界，再推进当前已经展开的引导期、中断期、单核多任务期和多核运行期阶段，最后
             * 进入 selected payload 的不返回交接边界。
             */
            on Transition::Setup -> State::Ready {
                drives {
                    PreparePhase.Transition::Setup;
                    PreparePhase.Transition::Enable;
                    BootPhase.Transition::Setup;
                    InterruptPhase.Transition::Setup;
                    UpMultitaskPhase.Transition::Setup;
                    SmpRuntimePhase.Transition::Setup;
                    PayloadPhase.Transition::Setup;
                    PayloadPhase.Transition::Enable;
                }
            }
        }
    }

    /*
     * Ready 表示准备期边界已经生效，当前已经展开的引导期/中断期/单核多任务期/多核运行期阶段已经完成，
     * 且启动链已经移交给 selected payload。
     */
    state State::Ready {
        invariant {
            PreparePhase.state == State::Online;
            BootPhase.state == State::Ready;
            InterruptPhase.state == State::Ready;
            UpMultitaskPhase.state == State::Ready;
            SmpRuntimePhase.state == State::Ready;
            PayloadPhase.state == State::Online;
        }
    }
}
