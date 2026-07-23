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
 * BootTask 从模型入口起已经 OnCpu；Kernel 直接异步启动 BootInitFlow，
 * 随后在真实 PID 1 栈入口继续
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
                    BootTask.state == State::OnCpu;
                }

                ensures {
                    BootTask.state == State::OnCpu;
                    BootInitFlow.state == State::Base;
                }

                emits {
                    BootInitFlow.Transition::Preset;
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
                depends_on {
                    BootInitFlow.state == State::Online;
                    BootTask.state == State::OnCpu;
                }
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
                }

                ensures {
                    BootInitFlow.state == State::Online;
                    kernel_init_task_stack_switch_committed(
                        Scheduler,
                        BootTask,
                        KernelInitTask
                    );
                    BootTask.state == State::Online;
                    KernelInitTask.state == State::Online;
                    current_task_slot_current(BootCpuCurrentTask, KernelInitTask);
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
