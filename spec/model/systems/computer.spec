/* Computer is the explicit root of the independent runtime-system tree. */

object Computer: ComputerObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    HardwareProject.state == State::Ready;
                    FirmwareProject.state == State::Ready;
                    KernelProject.state == State::Ready;
                    computer_assembled_from(Riscv64Platform, OpenSBI, Kernel);
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
                    Riscv64Platform.Transition::Preset;
                }
            }
        }
    }

    state State::Online {
        invariant {
            computer_assembled_from(Riscv64Platform, OpenSBI, Kernel);
        }
    }
}
