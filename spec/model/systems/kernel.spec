/*
 * Kernel System Specification
 *
 * This file defines the formal kernel system object and composes phase trees.
 *
 */

include "../phases/boot/entry-prelude/main.spec";
include "../phases/boot/main.spec";
include "../phases/interrupt/main.spec";
include "../phases/boot-init/main.spec";
include "../phases/smp-runtime/main.spec";
include "../phases/payload/main.spec";

/*
 * Kernel 代表内核运行实例, 由 OpenSBI 交接控制权后启动。
 * `_start` 先使静态 BootTask Online；BootTask 的 initial-flow completion
 * signal 启动并自推进 BootInitFlow，随后在真实 PID 1 栈入口继续
 * 多核运行期以及应用交接期。
 */
object Kernel: KernelObject {
    initial_state: State::Base;
    parent: Computer;

    /*
     * Base 表示内核映像已经驻留在内存中，但是 Kernel 尚未收到启动事件、
     * 尚未开始内核生命周期。
     */
    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                /*
                * 检查硬件/固件规范，确保固件服务可用，确认内核Lds和Config存在。
                */
                depends_on {
                    Riscv64.state == State::Online;
                    SbiSpec.state == State::Online;
                    BootArgs.state == State::Online;
                    Riscv64Platform.state == State::Online;
                    OpenSBI.state == State::Online;
                    BootCpuRegisters.a0 == BootArgs.boot_hartid;
                    BootCpuRegisters.a1 == BootArgs.dtb_pa;
                    Lds.state == State::Online;
                    Config.state == State::Online;
                }

                drives {
                    BootTask.Transition::Enable;
                }

                ensures {
                    BootTask.state == State::Online;
                    BootInitFlow.state == State::Online;
                    EntryPreludePhase.state == State::Online;
                    task_flow_started(BootInitFlow);
                    task_flow_online_on_dispatch(BootInitFlow, BootDispatchWindow);
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    /*
     * Prepared 状态：入口前导期完成，具备进入引导期的条件。
     */
    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    BootInitFlow.state == State::Online;
                    EntrySuccessorPhase.state == State::Online;
                    CorePreparePhase.state == State::Online;
                    MmCoreInitPhase.state == State::Online;
                    SchedInitPhase.state == State::Online;
                    IrqTimeInitPhase.state == State::Online;
                    LocalIrqEnablePhase.state == State::Online;
                    IrqOpenPreparePhase.state == State::Online;
                    ProcessPreparePhase.state == State::Online;
                    BootInitRestInitPhase.state == State::Online;
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    /*
     * Ready 表示入口前导期、引导期和中断期阶段已经完成，可以继续推进多任务、
     * SMP/runtime 与 payload 交接。
     */
    state State::Ready {
        transitions {
            /*
             * BootInitFlow 已由自己的 completion-event 链提交 Online；
             * Enable 执行真实首次调度切换，KernelInitTask 的实际入口随后
             * 推进 SMP/runtime 和 payload。
             */
            on Transition::Enable -> State::Online {
                drives {
                    Scheduler.Action::Schedule;
                    KernelInitFlow.Action::CommitPayloadHandoff;
                }

                ensures {
                    BootInitFlow.state == State::Online;
                    kernel_init_task_stack_switch_committed(
                        Scheduler,
                        BootTask,
                        KernelInitTask
                    );
                    kernel_init_flow_payload_handoff_committed(KernelInitFlow);
                    selected_payload_user_boot_replacement_ordered(
                        SelectedPayloadHandoff,
                        KernelInitFlow,
                        KernelInitTask
                    );
                    selected_payload_kernel_mode_keeps_kernel_init_flow(
                        SelectedPayloadHandoff,
                        KernelInitFlow
                    );
                    PreSmpInitPhase.state == State::Online;
                    SmpBringupPhase.state == State::Online;
                    RuntimeCorePhase.state == State::Online;
                    InitcallPhase.state == State::Online;
                    RootfsPhase.state == State::Online;
                    FinalizePhase.state == State::Online;
                    PayloadPreparePhase.state == State::Online;
                    PayloadHandoffPreparePhase.state == State::Online;
                    kernel_init_flow_payload_handoff_committed(KernelInitFlow);
                }
            }
        }
    }

    /*
     * Online 表示内核启动编排链已经移交给 selected payload，内核进入正式服务状态。
     */
    state State::Online {
    }
}
