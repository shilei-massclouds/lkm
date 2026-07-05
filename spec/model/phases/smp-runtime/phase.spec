/*
 * SMP Runtime Phase Specification
 *
 * This top-level phase starts from the KernelInitTask execution line forked
 * by rest_init()'s first scheduler handoff. The currently expanded subphases
 * are PreSmpInitPhase, SmpBringupPhase, RuntimeCorePhase, InitcallPhase,
 * RootfsPhase and FinalizePhase. AP-side bringup is owned by subphases inside
 * SmpBringupPhase, not by the BP KernelInitTask execution line.
 */

include "pre-smp-init/main.spec";
include "smp-bringup/main.spec";
include "runtime-core/main.spec";
include "initcall/main.spec";
include "rootfs/main.spec";
include "finalize/main.spec";

/*
 * SmpRuntimePhase 表示 KernelInitTask 驱动的 SMP/runtime 建立链。
 * 它由 TaskEntry::KernelInit 启动，首个子阶段是 PreSmpInitPhase；
 * 真正打开 SMP 并行的是后续 SmpBringupPhase/smp_init()。
 */
object SmpRuntimePhase: PhaseObject {
    initial_state: State::Base;
    parent: Kernel;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    UpMultitaskPhase.state == State::Ready;
                    KernelInitTask.state == State::Online;
                    task_entry_bound(KernelInitTask, TaskEntry::KernelInit);
                    task_entry_first_phase(KernelInitTask, SmpRuntimePhase);
                    kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    scheduler_first_schedule_committed(Scheduler);
                }

                drives {
                    PreSmpInitPhase.Transition::Setup;
                    SmpBringupPhase.Transition::Setup;
                    RuntimeCorePhase.Transition::Setup;
                    InitcallPhase.Transition::Setup;
                    RootfsPhase.Transition::Setup;
                    FinalizePhase.Transition::Setup;
                }

                ensures {
                    smp_runtime_phase_ready(SmpRuntimePhase);
                    pre_smp_init_ready(PreSmpInitPhase);
                    smp_bringup_phase_ready(SmpBringupPhase);
                    runtime_core_phase_ready(RuntimeCorePhase);
                    initcall_phase_ready(InitcallPhase);
                    rootfs_phase_ready(RootfsPhase);
                    finalize_phase_ready(FinalizePhase);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            UpMultitaskPhase.state == State::Ready;
            PreSmpInitPhase.state == State::Ready;
            task_entry_bound(KernelInitTask, TaskEntry::KernelInit);
            task_entry_first_phase(KernelInitTask, SmpRuntimePhase);
            kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
            kernel_init_released_for_pre_smp_init(KernelInitTask);
            kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
            scheduler_first_schedule_committed(Scheduler);
            SmpBringupPhase.state == State::Ready;
            RuntimeCorePhase.state == State::Ready;
            InitcallPhase.state == State::Ready;
            RootfsPhase.state == State::Ready;
            FinalizePhase.state == State::Ready;
            smp_runtime_phase_ready(SmpRuntimePhase);
            pre_smp_init_ready(PreSmpInitPhase);
            runtime_core_phase_ready(RuntimeCorePhase);
            initcall_phase_ready(InitcallPhase);
            rootfs_phase_ready(RootfsPhase);
            finalize_phase_ready(FinalizePhase);
        }
    }
}
