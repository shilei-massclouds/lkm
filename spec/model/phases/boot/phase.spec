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

context SingleTaskContext: Context {
    /*
     * SingleTaskContext captures the natural boot execution context before
     * secondary CPUs and ordinary task concurrency are opened. The phase
     * boundary itself provides the proof; it does not lower to runtime guard
     * code.
     */
    guard {
        holds {
            cpu_concurrency: single_cpu;
            task_concurrency: single_task;
            local_interrupts: disabled;
            preemption: disabled;
        }
    }
}

/*
 * BootPhase 表示引导期阶段对象。它负责推进当前模型已经展开的引导期子阶段。
 */
object BootPhase: PhaseObject {
    initial_state: State::Base;
    parent: Kernel;

    /*
     * Base 表示引导期阶段对象已经进入模型空间，但尚未推进其子阶段。
     */
    state State::Base {
        transitions {
            /*
             * Setup 顺序推进入口前导期、入口后继期、核心准备期、内存核心初始化期
             * 和调度准备期五个子阶段。
             * BootPhase 不直接依赖 PreparePhase；二者作为平级阶段由上级阶段对象编排衔接。
             */
            on Transition::Setup -> State::Ready {
                within SingleTaskContext {
                    drives {
                        EntryPreludePhase.Transition::Setup;
                        EntrySuccessorPhase.Transition::Setup;
                        CorePreparePhase.Transition::Setup;
                        MmCoreInitPhase.Transition::Setup;
                        SchedInitPhase.Transition::Setup;
                    }
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
