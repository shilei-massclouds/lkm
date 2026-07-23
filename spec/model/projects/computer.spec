/*
 * Computer Project Specification
 *
 * ComputerProject is the engineering-tree root. It prepares the three direct
 * child projects, records the static Computer assembly fact, and hands startup
 * to the independent runtime-system tree.
 */

predicate computer_assembled_from<P, F, K>(platform: P, firmware: F, kernel: K) -> bool;

object ComputerProject: ProjectObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                drives {
                    HardwareProject.Transition::Preset;
                    FirmwareProject.Transition::Preset;
                    KernelProject.Transition::Preset;
                }

                ensures {
                    HardwareProject.state == State::Prepared;
                    FirmwareProject.state == State::Prepared;
                    KernelProject.state == State::Prepared;
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            HardwareProject.state == State::Prepared;
            FirmwareProject.state == State::Prepared;
            KernelProject.state == State::Prepared;
        }

        transitions {
            on Transition::Setup -> State::Ready {
                drives {
                    HardwareProject.Transition::Setup;
                    FirmwareProject.Transition::Setup;
                    KernelProject.Transition::Setup;
                }

                ensures {
                    HardwareProject.state == State::Ready;
                    FirmwareProject.state == State::Ready;
                    KernelProject.state == State::Ready;
                    computer_assembled_from(Riscv64Platform, OpenSBI, Kernel);
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        invariant {
            HardwareProject.state == State::Ready;
            FirmwareProject.state == State::Ready;
            KernelProject.state == State::Ready;
            computer_assembled_from(Riscv64Platform, OpenSBI, Kernel);
        }

        transitions {
            on Transition::Enable -> State::Online {
                drives {
                    Computer.Transition::Enable;
                }
            }
        }
    }

    /* Online means startup ownership has been handed to Computer. */
    state State::Online {
        invariant {
            HardwareProject.state == State::Ready;
            FirmwareProject.state == State::Ready;
            KernelProject.state == State::Ready;
            computer_assembled_from(Riscv64Platform, OpenSBI, Kernel);
        }
    }
}
