/* Firmware engineering specification and project-owned static products. */

predicate opensbi_system_spec_established() -> bool;
predicate opensbi_firmware_constructed() -> bool;
predicate firmware_boot_args_defined<T>(boot_args: T) -> bool;
predicate boot_args_read_only<T>(boot_args: T) -> bool;

object SbiSpec: PrepareObject {
    initial_state: State::Online;
    parent: FirmwareProject;
    source: external_spec::riscv_sbi;

    state State::Online {
        invariant {
            sbi_hsm_available();
        }
    }
}

object BootArgs: PrepareObject {
    initial_state: State::Online;
    parent: FirmwareProject;
    source: firmware_project::boot_abi;

    attrs {
        boot_hartid: HartId;
        dtb_pa: PhysAddr<Dtb>;
    }

    state State::Online {
        invariant {
            attrs_accessible(self);
            firmware_boot_args_defined(self);
            boot_args_read_only(self);
        }
    }
}

object FirmwareProject: ProjectObject {
    initial_state: State::Base;
    parent: ComputerProject;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    SbiSpec.state == State::Online;
                }

                ensures {
                    opensbi_system_spec_established();
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            SbiSpec.state == State::Online;
            opensbi_system_spec_established();
        }

        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    opensbi_firmware_constructed();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            SbiSpec.state == State::Online;
            BootArgs.state == State::Online;
            opensbi_system_spec_established();
            opensbi_firmware_constructed();
        }
    }
}
