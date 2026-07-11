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
 * BootPhase 引导期阶段
 * 顺序推进入口前导期、入口后继期、核心准备期、内存核心初始化期和调度准备期五个子阶段。
 */
object BootPhase: PhaseObject {
    initial_state: State::Base;
    parent: Kernel;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                within SingleTaskContext {
                    drives {
                        EntryPreludePhase.Transition::Preset;
                    }
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
