/* RISC-V platform runtime system. Boot CPU registers are CPU-local. */

object Riscv64Platform: PlatformObject {
    initial_state: State::Ready;
    parent: Computer;

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    HardwareProject.state == State::Ready;
                    Riscv64.state == State::Online;
                    riscv64_platform_system_spec_established();
                    riscv64_platform_constructed();
                }

                emits {
                    OpenSBI.Transition::Enable;
                }
            }
        }
    }

    state State::Online {
    }
}
