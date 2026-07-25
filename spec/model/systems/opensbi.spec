/* OpenSBI specification, firmware construction, and kernel handoff. */

predicate opensbi_system_spec_established() -> bool;
predicate opensbi_firmware_constructed() -> bool;
predicate firmware_boot_args_defined<T>(boot_args: T) -> bool;
predicate boot_args_read_only<T>(boot_args: T) -> bool;

object SbiSpec: PrepareObject {
    initial_state: State::Online;
    parent: OpenSBI;
    source: external_spec::riscv_sbi;

    state State::Online {
        invariant {
            sbi_hsm_available();
        }
    }
}

object BootArgs: PrepareObject {
    initial_state: State::Online;
    parent: OpenSBI;
    source: firmware::boot_abi;

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

object OpenSBI: FirmwareObject {
    initial_state: State::Base;
    parent: Computer;
    source: firmware::opensbi;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    SbiSpec.state == State::Online;
                    BootArgs.state == State::Online;
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
            BootArgs.state == State::Online;
            opensbi_system_spec_established();
        }

        transitions {
            on Transition::Setup -> State::Ready {
                ensures {
                    opensbi_system_spec_established();
                    opensbi_firmware_constructed();
                }
            }
        }
    }

    state State::Ready {
        invariant {
            SbiSpec.state == State::Online;
            BootArgs.state == State::Online;
            ordered_booting_enabled();
            primary_hart_only_at_kernel_entry();
            primary_hart_sie_clear_at_kernel_entry();
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    Computer.state == State::Online;
                    Riscv64Platform.state == State::Online;
                    SbiSpec.state == State::Online;
                    BootArgs.state == State::Online;
                    opensbi_system_spec_established();
                    opensbi_firmware_constructed();
                    Kernel.state == State::Ready;
                    Lds.state == State::Online;
                    Config.state == State::Online;
                    kernel_image_constructed();
                }

                may_change {
                    BootCpuRegisters.a0;
                    BootCpuRegisters.a1;
                }

                ensures {
                    ordered_booting_enabled();
                    primary_hart_only_at_kernel_entry();
                    primary_hart_sie_clear_at_kernel_entry();
                    firmware_dtb_blob_in_ram_at_kernel_entry(BootArgs.dtb_pa);
                    firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa);
                    firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa);
                    BootCpuRegisters.a0 == BootArgs.boot_hartid;
                    BootCpuRegisters.a1 == BootArgs.dtb_pa;
                    task_ref_targets(BootTaskRef, BootTask);
                    task_ref_ready(BootTaskRef);
                    task_execution_authority_is(
                        BootTask,
                        TaskExecutionAuthority::Live
                    );
                    task_breakpoint_state_is(
                        BootTask,
                        TaskBreakpointState::Invalid
                    );
                }

                emits {
                    Kernel.Transition::Enable;
                }
            }
        }
    }

    state State::Online {
        invariant {
            SbiSpec.state == State::Online;
            BootArgs.state == State::Online;
            opensbi_system_spec_established();
            opensbi_firmware_constructed();
            BootCpuRegisters.a0 == BootArgs.boot_hartid;
            BootCpuRegisters.a1 == BootArgs.dtb_pa;
            Lds.state == State::Online;
            Config.state == State::Online;
            ordered_booting_enabled();
            primary_hart_only_at_kernel_entry();
            primary_hart_sie_clear_at_kernel_entry();
            firmware_dtb_blob_in_ram_at_kernel_entry(BootArgs.dtb_pa);
            firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa);
            firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa);
            task_execution_authority_is(
                BootTask,
                TaskExecutionAuthority::Live
            );
            task_breakpoint_state_is(
                BootTask,
                TaskBreakpointState::Invalid
            );
        }
    }
}
