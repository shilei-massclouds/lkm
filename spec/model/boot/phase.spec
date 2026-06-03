/*
 * Boot Phase Specification
 *
 * This phase currently drives the entry-prelude, entry-successor, core-prepare,
 * mm-core-init, and sched-init subphases.
 */

include "entry-prelude/main.spec";
include "entry-successor/main.spec";
include "core-prepare/main.spec";
include "mm-core-init/main.spec";
include "sched-init/main.spec";

/*
 * BootPhase 表示引导期阶段对象。它负责推进当前模型已经展开的引导期子阶段。
 */
object BootPhase: PhaseObject {
    initial_state: State::Base;
    parent: StartupTimeline;

    /*
     * Base 表示引导期阶段对象已经进入模型空间，但尚未推进其子阶段。
     */
    state State::Base {
        events {
            /*
             * Setup 顺序推进入口前导期、入口后继期、核心准备期、内存核心初始化期
             * 和调度准备期五个子阶段。
             * BootPhase 不直接依赖 PreparePhase；二者作为平级阶段由上级阶段对象编排衔接。
             */
            on Event::Setup -> State::Ready {
                drives {
                    EntryPreludePhase.Event::Setup;
                    EntrySuccessorPhase.Event::Setup;
                    CorePreparePhase.Event::Setup;
                    MmCoreInitPhase.Event::Setup;
                    SchedInitPhase.Event::Setup;
                }
            }
        }
    }

    /*
     * Ready 表示当前模型已经展开的引导期子阶段均已完成。
     */
    state State::Ready {
        invariant {
            EntryPreludePhase.state == State::Destroyed;
            EntrySuccessorPhase.state == State::Ready;
            CorePreparePhase.state == State::Ready;
            MmCoreInitPhase.state == State::Ready;
            SchedInitPhase.state == State::Ready;
        }
    }
}
