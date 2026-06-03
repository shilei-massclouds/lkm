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
 * 它不固定 payload 运行形态：当前可以是内核态 Unikernel app，后续也可以是
 * 加载首个用户态程序并切换到用户态的宏内核入口。
 */
object PayloadPhase: PhaseObject {
    initial_state: State::Base;
    parent: StartupTimeline;

    /*
     * Base 表示 payload 阶段已经进入模型空间，但尚未确认 selected payload
     * 及其运行前置条件。
     */
    state State::Base {
        events {
            /*
             * Setup 确认 selected payload 可进入。当前规格只抽象 payload 选择和
             * 前置条件，不在这里区分 Unikernel、测试 payload 或用户态首进程。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    PreparePhase.state == State::Online;
                    BootPhase.state == State::Ready;
                    InterruptPhase.state == State::Ready;
                }

                ensures {
                    selected_payload_ready();
                }
            }
        }
    }

    /*
     * Ready 表示 selected payload 已经确定，且启动链可以执行最终交接。
     */
    state State::Ready {
        invariant {
            PreparePhase.state == State::Online;
            BootPhase.state == State::Ready;
            InterruptPhase.state == State::Ready;
            selected_payload_ready();
        }

        events {
            /*
             * Enable 表示启动编排链不可逆地移交给 selected payload。该 handoff
             * 的运行期语义是不返回：payload 要么进入服务循环，要么最终停机。
             */
            on Event::Enable -> State::Online {
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
            PreparePhase.state == State::Online;
            BootPhase.state == State::Ready;
            InterruptPhase.state == State::Ready;
            selected_payload_ready();
            selected_payload_no_return_handoff();
        }
    }
}
