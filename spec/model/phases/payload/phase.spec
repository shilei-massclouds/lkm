/*
 * Payload Phase Specification
 *
 * This phase is the final startup handoff boundary.  It covers Unikernel
 * payloads that continue in kernel mode and future Linux-like payloads that
 * load and enter the first user-mode program.  In all forms, entering the
 * selected payload does not return to the startup orchestration chain.
 */

/*
 * PayloadPhase 表示启动时间轴末尾的 selected payload 交接阶段。
 * 它是 SmpRuntimePhase 的后续阶段，不是 SmpRuntimePhase 的子阶段；
 * 直接衔接 FinalizePhase.Ready / FinalizeBoundary.Ready。
 * KernelInitTask 的执行线从 SmpRuntimePhase 入口开始，并经由其首个
 * 子阶段 PreSmpInitPhase 以及后续 phase 顺序自然到达本阶段；该入口由
 * TaskCreationCore 在创建 KernelInitTask 时绑定的 TaskEntry::KernelInit 决定。
 * 它不固定 payload 运行形态：当前可以是内核态 Unikernel app，后续也可以是
 * 加载首个用户态程序并切换到用户态的宏内核入口。
 */
object PayloadPhase: PhaseObject {
    initial_state: State::Base;
    parent: Kernel;

    /*
     * Base 表示 payload 阶段已经进入模型空间，但尚未确认 selected payload
     * 及其运行前置条件。
     */
    state State::Base {
        transitions {
            /*
             * Preset 确认 selected payload 可进入。当前规格只抽象 payload 选择和
             * 前置条件；Linux-like 用户态首进程路径由 UserBootPayload 承载。
             */
            on Transition::Preset -> State::Prepared {
                depends_on {
                    SmpRuntimePhase.state == State::Online;
                    Riscv64.state == State::Online;
                    SbiSpec.state == State::Online;
                    OpenSBI.state == State::Online;
                    Lds.state == State::Online;
                    Config.state == State::Online;
                    FinalizePhase.state == State::Ready;
                    FinalizeBoundary.state == State::Ready;
                    SystemState.state == State::Online;
                    KernelInitTask.state == State::Online;
                    system_state_running(SystemState);
                    task_entry_bound(KernelInitTask, TaskEntry::KernelInit);
                    task_entry_first_phase(KernelInitTask, SmpRuntimePhase);
                    kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
                    kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
                    payload_phase_next_boundary();
                }

                drives {
                    PayloadExecSyncBoundaries.Transition::Setup;
                    UserCloneDeferredBoundaries.Transition::Setup;
                    UserBootPayload.Transition::Setup;
                }

                ensures {
                    selected_payload_ready();
                    payload_execution_owned_by_kernel_init_task(
                        PayloadPhase,
                        KernelInitTask
                    );
                }
            }
        }
    }

    state State::Prepared {
    }

    /*
     * Ready 表示 selected payload 已经确定，且启动链可以执行最终交接。
     */
    state State::Ready {
        invariant {
            Riscv64.state == State::Online;
            SbiSpec.state == State::Online;
            OpenSBI.state == State::Online;
            Lds.state == State::Online;
            Config.state == State::Online;
            BootPhase.state == State::Online;
            InterruptPhase.state == State::Online;
            UpMultitaskPhase.state == State::Online;
            SmpRuntimePhase.state == State::Online;
            FinalizePhase.state == State::Ready;
            FinalizeBoundary.state == State::Ready;
            SystemState.state == State::Online;
            KernelInitTask.state == State::Online;
            system_state_running(SystemState);
            task_entry_bound(KernelInitTask, TaskEntry::KernelInit);
            task_entry_first_phase(KernelInitTask, SmpRuntimePhase);
            kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
            kernel_init_task_stack_switch_committed(
                Scheduler,
                BootIdleTask,
                KernelInitTask
            );
            kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
            payload_phase_next_boundary();
            selected_payload_ready();
            payload_execution_owned_by_kernel_init_task(PayloadPhase, KernelInitTask);
            PayloadExecSyncBoundaries.state == State::Ready;
            UserCloneDeferredBoundaries.state == State::Ready;
        }

        transitions {
            /*
             * Enable 表示启动编排链不可逆地移交给 selected payload。该 handoff
             * 的运行期语义是不返回：payload 要么进入服务循环，要么最终停机。
             */
            on Transition::Enable -> State::Online {
                drives {
                    UserBootPayload.Transition::Enable;
                }

                ensures {
                    selected_payload_ready();
                    selected_payload_no_return_handoff();
                }
            }
        }
    }

    /*
     * Online 表示启动编排链已经移交给 selected payload。该状态是形式化终态，
     * 不要求运行期存在从 payload 返回后的可观测代码点。
     */
    state State::Online {
        invariant {
            Riscv64.state == State::Online;
            SbiSpec.state == State::Online;
            OpenSBI.state == State::Online;
            Lds.state == State::Online;
            Config.state == State::Online;
            BootPhase.state == State::Online;
            InterruptPhase.state == State::Online;
            UpMultitaskPhase.state == State::Online;
            SmpRuntimePhase.state == State::Online;
            FinalizePhase.state == State::Ready;
            FinalizeBoundary.state == State::Ready;
            SystemState.state == State::Online;
            KernelInitTask.state == State::Online;
            system_state_running(SystemState);
            task_entry_bound(KernelInitTask, TaskEntry::KernelInit);
            task_entry_first_phase(KernelInitTask, SmpRuntimePhase);
            kernel_init_entry_reaches_smp_runtime(KernelInitTask, SmpRuntimePhase);
            kernel_init_task_stack_switch_committed(
                Scheduler,
                BootIdleTask,
                KernelInitTask
            );
            kernel_init_dispatched_to_pre_smp_init(KernelInitTask);
            payload_phase_next_boundary();
            selected_payload_ready();
            payload_execution_owned_by_kernel_init_task(PayloadPhase, KernelInitTask);
            PayloadExecSyncBoundaries.state == State::Ready;
            UserCloneDeferredBoundaries.state == State::Ready;
            selected_payload_no_return_handoff();
        }
    }
}
