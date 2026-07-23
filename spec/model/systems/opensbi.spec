/* OpenSBI runtime system and the firmware-to-kernel handoff boundary. */

object OpenSBI: FirmwareObject {
    initial_state: State::Base;
    parent: Computer;
    source: firmware::opensbi;

    state State::Base {
        invariant {
            ordered_booting_enabled();
            primary_hart_only_at_kernel_entry();
            primary_hart_sie_clear_at_kernel_entry();
        }

        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    FirmwareProject.state == State::Ready;
                    SbiSpec.state == State::Online;
                    BootArgs.state == State::Online;
                    opensbi_system_spec_established();
                    opensbi_firmware_constructed();
                }

                ensures {
                    ordered_booting_enabled();
                    primary_hart_only_at_kernel_entry();
                    primary_hart_sie_clear_at_kernel_entry();
                    firmware_dtb_blob_in_ram_at_kernel_entry(BootArgs.dtb_pa);
                    firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa);
                    firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa);
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            ordered_booting_enabled();
            primary_hart_only_at_kernel_entry();
            primary_hart_sie_clear_at_kernel_entry();
            firmware_dtb_blob_in_ram_at_kernel_entry(BootArgs.dtb_pa);
            firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa);
            firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa);
        }

        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    Riscv64Platform.state == State::Online;
                    Lds.state == State::Online;
                    Config.state == State::Online;
                    kernel_image_constructed();
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
                may_change {
                    BootCpuRegisters.a0;
                    BootCpuRegisters.a1;
                }

                ensures {
                    BootCpuRegisters.a0 == BootArgs.boot_hartid;
                    BootCpuRegisters.a1 == BootArgs.dtb_pa;
                    task_ref_targets(BootTaskRef, BootTask);
                    task_ref_ready(BootTaskRef);
                }

                emits {
                    Kernel.Transition::Preset;
                }
            }
        }
    }

    state State::Online {
        invariant {
            SbiSpec.state == State::Online;
            BootArgs.state == State::Online;
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
        }
    }
}
