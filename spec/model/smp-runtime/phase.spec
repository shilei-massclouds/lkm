/*
 * SMP Runtime Phase Specification
 *
 * This top-level phase starts at smp_init(). The currently expanded
 * subphases are SmpBringupPhase, RuntimeCorePhase, InitcallPhase and
 * RootfsPhase. Later finalization subphase details are intentionally summarized
 * as deferred boundaries so the object-level prototype can continue to
 * PayloadPhase while AP-side details remain future work.
 */

include "smp-bringup/main.spec";
include "runtime-core/main.spec";
include "initcall/main.spec";
include "rootfs/main.spec";

/*
 * SmpRuntimePhase 表示多核运行期阶段对象。本轮正式展开
 * SmpBringupPhase、RuntimeCorePhase、InitcallPhase 与 RootfsPhase；
 * 后续 finalize 子阶段保留 deferred 边界。
 */
object SmpRuntimePhase: PhaseObject {
    initial_state: State::Base;
    parent: StartupTimeline;

    state State::Base {
        events {
            on Event::Setup -> State::Ready {
                depends_on {
                    UpMultitaskPhase.state == State::Ready;
                    PreSmpInitPhase.state == State::Ready;
                }

                drives {
                    SmpBringupPhase.Event::Setup;
                    RuntimeCorePhase.Event::Setup;
                    InitcallPhase.Event::Setup;
                    RootfsPhase.Event::Setup;
                }

                ensures {
                    smp_runtime_phase_ready(SmpRuntimePhase);
                    smp_bringup_phase_ready(SmpBringupPhase);
                    runtime_core_phase_ready(RuntimeCorePhase);
                    initcall_phase_ready(InitcallPhase);
                    rootfs_phase_ready(RootfsPhase);
                    finalize_phase_deferred();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            UpMultitaskPhase.state == State::Ready;
            PreSmpInitPhase.state == State::Ready;
            SmpBringupPhase.state == State::Ready;
            RuntimeCorePhase.state == State::Ready;
            InitcallPhase.state == State::Ready;
            RootfsPhase.state == State::Ready;
            smp_runtime_phase_ready(SmpRuntimePhase);
            runtime_core_phase_ready(RuntimeCorePhase);
            initcall_phase_ready(InitcallPhase);
            rootfs_phase_ready(RootfsPhase);
            finalize_phase_deferred();
        }
    }
}
