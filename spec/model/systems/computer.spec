/* Computer is the explicit root of the independent runtime-system tree. */

object Computer: ComputerObject {
    initial_state: State::Ready;

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    HardwareProject.state == State::Ready;
                    FirmwareProject.state == State::Ready;
                    KernelProject.state == State::Ready;
                    computer_assembled_from(Riscv64Platform, OpenSBI, Kernel);
                }

                emits {
                    Riscv64Platform.Transition::Enable;
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
