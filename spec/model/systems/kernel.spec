/* Kernel specification, image construction, runtime handoff, and phase composition. */

include "../phases/boot/main.spec";
include "../phases/interrupt/main.spec";
include "../phases/boot-init/main.spec";
include "../phases/smp-runtime/main.spec";
include "../phases/payload/main.spec";

object Config: PrepareObject {
    initial_state: State::Ready;
    parent: Kernel;
    source: config::entry_prelude;

    attrs {
        page_size: Size;
        pt_size_on_stack: Size;
        boot_stack_size: Size;
        pmd_size: Size;
        kernel_link_addr: VirtAddr<KernelImage>;
        kernel_image_va_window_size: Size;
        satp_mode: SatpMode;
        fixmap: FixMapConfig;
        selected_payload_kind: SelectedPayloadKind;
        exec_argument_limits: ExecArgumentLimits;
    }

    state State::Ready {
        invariant {
            attrs_accessible(self);
            page_size > 0;
            pmd_size >= page_size;
            aligned(pmd_size, page_size);
            pt_size_on_stack > 0;
            pt_size_on_stack < page_size;
            boot_stack_size >= page_size;
            aligned(boot_stack_size, page_size);
            kernel_link_addr != 0;
            page_aligned(kernel_link_addr);
            valid_virt_addr(kernel_link_addr);
            kernel_image_va_window_size > 0;
            kernel_image_va_window_size >= pmd_size;
            valid_satp_mode(satp_mode);
            valid_fixmap_config(fixmap);
        }

        transitions {
            on Transition::Enable -> State::Online {
            }
        }
    }

    state State::Online {
        invariant {
            attrs_accessible(self);
            page_size > 0;
            pmd_size >= page_size;
            aligned(pmd_size, page_size);
            pt_size_on_stack > 0;
            pt_size_on_stack < page_size;
            boot_stack_size >= page_size;
            aligned(boot_stack_size, page_size);
            kernel_link_addr != 0;
            page_aligned(kernel_link_addr);
            valid_virt_addr(kernel_link_addr);
            kernel_image_va_window_size > 0;
            kernel_image_va_window_size >= pmd_size;
            valid_satp_mode(satp_mode);
            valid_fixmap_config(fixmap);
        }
    }

    reference linux_6_12 {
        kernel_link_addr = symbol("KERNEL_LINK_ADDR");
        kernel_image_va_window_size = symbol("SZ_2G");
    }
}

object Lds: PrepareObject {
    initial_state: State::Ready;
    parent: Kernel;
    source: linker::linux_6_12;

    attrs {
        global_pointer: SymbolAddr;
        text_start: SymbolAddr;
        text_end: SymbolAddr;
        rodata_start: SymbolAddr;
        rodata_end: SymbolAddr;
        data_start: SymbolAddr;
        data_end: SymbolAddr;
        elf_entry: SymbolAddr;
        head_text_range: AddrRange;
        pre_mmu_text_range: AddrRange;
        trampoline_safe_text_range: AddrRange;
        bss_start: SymbolAddr;
        bss_end: SymbolAddr;
        per_cpu_start: SymbolAddr;
        per_cpu_end: SymbolAddr;
        per_cpu_load: SymbolAddr;
        init_stack_start: SymbolAddr;
        init_stack_end: SymbolAddr;
        boot_stack_size: Size;
        kernel_start: SymbolAddr;
        kernel_end: SymbolAddr;
    }

    state State::Ready {
        invariant {
            attrs_accessible(self);
            global_pointer != 0;
            kernel_start != 0;
            text_start == kernel_start;
            text_end > text_start;
            rodata_end >= rodata_start;
            data_end >= data_start;
            elf_entry == kernel_start;
            kernel_end > kernel_start;
            entry_head_text_layout_ready(Lds);
            pre_mmu_access_discipline_ready(Lds);
            trampoline_access_discipline_ready(Lds);
            inside(text_start, text_end, kernel_start, kernel_end);
            inside(rodata_start, rodata_end, kernel_start, kernel_end);
            inside(data_start, data_end, kernel_start, kernel_end);
            bss_start != 0;
            bss_end > bss_start;
            inside(bss_start, bss_end, kernel_start, kernel_end);
            per_cpu_start != 0;
            per_cpu_end > per_cpu_start;
            per_cpu_load != 0;
            per_cpu_static_image_layout_ready(Lds);
            init_stack_start != 0;
            init_stack_end > init_stack_start;
            page_aligned(init_stack_start);
            page_aligned(init_stack_end);
            boot_stack_size == Config.boot_stack_size;
            init_stack_end - init_stack_start == boot_stack_size;
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    Config.state == State::Online;
                }
            }
        }
    }

    state State::Online {
        invariant {
            Config.state == State::Online;
            attrs_accessible(self);
            global_pointer != 0;
            kernel_start != 0;
            text_start == kernel_start;
            text_end > text_start;
            rodata_end >= rodata_start;
            data_end >= data_start;
            elf_entry == kernel_start;
            kernel_end > kernel_start;
            entry_head_text_layout_ready(Lds);
            pre_mmu_access_discipline_ready(Lds);
            trampoline_access_discipline_ready(Lds);
            inside(text_start, text_end, kernel_start, kernel_end);
            inside(rodata_start, rodata_end, kernel_start, kernel_end);
            inside(data_start, data_end, kernel_start, kernel_end);
            bss_start != 0;
            bss_end > bss_start;
            inside(bss_start, bss_end, kernel_start, kernel_end);
            per_cpu_start != 0;
            per_cpu_end > per_cpu_start;
            per_cpu_load != 0;
            per_cpu_static_image_layout_ready(Lds);
            init_stack_start != 0;
            init_stack_end > init_stack_start;
            page_aligned(init_stack_start);
            page_aligned(init_stack_end);
            boot_stack_size == Config.boot_stack_size;
            init_stack_end - init_stack_start == boot_stack_size;
        }
    }

    reference linux_6_12 {
        global_pointer = symbol("__global_pointer$");
        kernel_start = symbol("_start");
        text_start = symbol("_start");
        text_end = symbol("_etext");
        rodata_start = symbol("_srodata");
        rodata_end = symbol("_erodata");
        data_start = symbol("_sdata");
        data_end = symbol("_edata");
        elf_entry = symbol("_start");
        head_text_range = section(".head.text");
        pre_mmu_text_range = section(".head.text");
        trampoline_safe_text_range = symbol_range("relocate_enable_mmu", ".Lsecondary_park");
        kernel_end = symbol("_end");
        bss_start = symbol("__bss_start");
        bss_end = symbol("__bss_stop");
        per_cpu_start = symbol("__per_cpu_start");
        per_cpu_end = symbol("__per_cpu_end");
        per_cpu_load = symbol("__per_cpu_load");
        init_stack_start = symbol("init_thread_union");
        init_stack_end = expr("init_thread_union + THREAD_SIZE");
        boot_stack_size = symbol("THREAD_SIZE");
    }
}

type ExecArgumentLimits {
    max_arg_strings: Size;
    max_arg_strlen: Size;
    arg_max_floor: Size;
    stack_rlimit: Size;
    stk_lim: Size;
    argument_bytes: Size;
}

object Kernel: KernelObject {
    initial_state: State::Base;
    parent: Computer;

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                ensures {
                    kernel_system_spec_established();
                }
            }
        }
    }

    state State::Prepared {
        invariant {
            kernel_system_spec_established();
        }

        transitions {
            on Transition::Setup -> State::Ready {
                drives {
                    Config.Transition::Enable;
                    Lds.Transition::Enable;
                }

                ensures {
                    Config.state == State::Online;
                    Lds.state == State::Online;
                    kernel_image_constructed();
                    kernel_enable_accept_available(self);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            Config.state == State::Online;
            Lds.state == State::Online;
            kernel_system_spec_established();
            kernel_image_constructed();
            kernel_enable_accept_available(self);
        }

        actions {
            on Action::AcceptEnable {
                depends_on {
                    kernel_enable_accept_available(self);
                }
                ensures {
                    kernel_enable_accepted(self);
                }
            }
        }

        transitions {
            on Transition::Enable -> State::Online {
                depends_on {
                    Riscv64.state == State::Online;
                    SbiSpec.state == State::Online;
                    BootArgs.state == State::Online;
                    Riscv64Platform.state == State::Online;
                    OpenSBI.state == State::Online;
                    BootCpuRegisters.a0 == BootArgs.boot_hartid;
                    BootCpuRegisters.a1 == BootArgs.dtb_pa;
                    Lds.state == State::Online;
                    Config.state == State::Online;
                    kernel_image_constructed();
                    BootTask.state == State::OnCpu;
                    BootInitFlow.state == State::Base;
                }

                drives {
                    Kernel.Action::AcceptEnable;
                    BootInitFlow.Transition::Preset;
                    BootInitFlow.Transition::Setup;
                    BootInitFlow.Transition::Enable;
                    Scheduler.Action::Schedule;
                    KernelInitFlow.Transition::Setup;
                    KernelInitFlow.Transition::Enable;
                }

                ensures {
                    BootInitFlow.state == State::Online;
                    scheduler_first_schedule_committed(Scheduler);
                    KernelInitTask.state == State::OnCpu;
                    KernelInitFlow.state == State::Online;
                    PayloadHandoffPreparePhase.state == State::Online;
                    SelectedPayloadHandoff.state == State::Online;
                    kernel_application_environment_ready(
                        self,
                        BootInitFlow,
                        Scheduler,
                        KernelInitTask,
                        KernelInitFlow,
                        PayloadHandoffPreparePhase,
                        SelectedPayloadHandoff
                    );
                }

                emits {
                    KernelInitFlow.Action::CommitPayloadHandoff;
                }
            }
        }
    }

    state State::Online {
        invariant {
            Riscv64Platform.state == State::Online;
            OpenSBI.state == State::Online;
            Config.state == State::Online;
            Lds.state == State::Online;
            kernel_image_constructed();
            kernel_enable_accepted(self);
            BootCpuRegisters.a0 == BootArgs.boot_hartid;
            BootCpuRegisters.a1 == BootArgs.dtb_pa;
            kernel_application_environment_ready(
                self,
                BootInitFlow,
                Scheduler,
                KernelInitTask,
                KernelInitFlow,
                PayloadHandoffPreparePhase,
                SelectedPayloadHandoff
            );
        }
    }
}

/*
 * Stable completion fact captured at the Kernel.Online commit. The concrete
 * lower states above are checked at that instant; an emitted UserBoot handoff
 * may then replace and destroy KernelInitFlow without rolling Kernel back.
 */
predicate kernel_application_environment_ready<K, B, S, T, F, P, H>(
    kernel: K,
    boot_flow: B,
    scheduler: S,
    init_task: T,
    init_flow: F,
    prepare_phase: P,
    handoff: H
) -> bool;
