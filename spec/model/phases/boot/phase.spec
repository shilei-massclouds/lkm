/*
 * Boot Phase Specification
 *
 * This phase drives the entry-successor, core-prepare, mm-core-init, and
 * sched-init subphases after the Kernel-owned entry prelude is online.
 */

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
 * BootPhase 引导期阶段
 * 在 Kernel 直接拥有的入口前导期完成后，顺序推进入口后继期、核心准备期、
 * 内存核心初始化期和调度准备期四个直接子阶段。
 */
object BootPhase: PhaseObject {
    initial_state: State::Base;
    parent: BootInitFlow;

    state State::Base {
        transitions {
            /*
             * Kernel.Preset 已同步驱动 EntryPreludePhase 到 Online。Kernel.Setup 随后
             * 驱动本 transition；BootPhase 只确认这个前置阶段，不重新拥有或驱动它。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    EntryPreludePhase.state == State::Online;
                }

                ensures {
                    EntryPreludePhase.state == State::Online;
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
                within SingleTaskContext {
                    drives {
                        EntrySuccessorPhase.Transition::Preset;
                    }
                }

                ensures {
                    EntrySuccessorPhase.state == State::Online;
                }

                /* EntrySuccessor Online returns to the Boot.Setup continuation. */

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                within SingleTaskContext {
                    drives {
                        CorePreparePhase.Transition::Preset;
                    }
                }
                ensures {
                    CorePreparePhase.state == State::Online;
                }

                /* Each child returns to the next Boot.Enable continuation. */

                within SingleTaskContext {
                    drives {
                        MmCoreInitPhase.Transition::Preset;
                    }
                }
                ensures {
                    MmCoreInitPhase.state == State::Online;
                }
                
                within SingleTaskContext {
                    drives {
                        SchedInitPhase.Transition::Preset;
                    }
                }
                ensures {
                    SchedInitPhase.state == State::Online;
                }
            }
        }
    }

    state State::Online {
    }
}
