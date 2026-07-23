/* RISC-V platform runtime system. Boot CPU registers are CPU-local. */

object Riscv64Platform: PlatformObject {
    initial_state: State::Base;
    parent: Computer;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    HardwareProject.state == State::Ready;
                    Riscv64.state == State::Online;
                    riscv64_platform_system_spec_established();
                    riscv64_platform_constructed();
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
                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                emits {
                    OpenSBI.Transition::Preset;
                }
            }
        }
    }

    state State::Online {
    }
}
