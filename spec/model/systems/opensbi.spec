/* OpenSBI runtime system and the firmware-to-kernel handoff boundary. */

object OpenSBI: FirmwareObject {
    initial_state: State::Ready;
    parent: Computer;
    source: firmware::opensbi;

    state State::Ready {
        invariant {
            ordered_booting_enabled();
            primary_hart_only_at_kernel_entry();
            primary_hart_sie_clear_at_kernel_entry();
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    FirmwareProject.state == State::Ready;
                    SbiSpec.state == State::Online;
                    BootArgs.state == State::Online;
                    opensbi_system_spec_established();
                    opensbi_firmware_constructed();
                    Riscv64Platform.state == State::Online;
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
