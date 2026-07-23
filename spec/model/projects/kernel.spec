/*
 * Kernel Project Specification
 *
 * KernelProject establishes the Kernel system specification, constructs Config
 * completely, then constructs Lds and the kernel image. It stops at Ready.
 */

object Config: PrepareObject {
    initial_state: State::Base;
    parent: KernelProject;
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

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                emits { Transition::Setup; }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                emits { Transition::Enable; }
            }
        }
    }

    state State::Ready {
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
    initial_state: State::Base;
    parent: KernelProject;
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

    state State::Base {
        transitions {
            on Transition::Preset -> State::Prepared {
                depends_on {
                    Config.state == State::Online;
                }

                emits { Transition::Setup; }
            }
        }
    }

    state State::Prepared {
        transitions {
            on Transition::Setup -> State::Ready {
                emits { Transition::Enable; }
            }
        }
    }

    state State::Ready {
        transitions {
            on Transition::Enable -> State::Online {
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

object KernelProject: ProjectObject {
    initial_state: State::Base;
    parent: ComputerProject;

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
                    Config.Transition::Preset;
                    Lds.Transition::Preset;
                }

                ensures {
                    Config.state == State::Online;
                    Lds.state == State::Online;
                    kernel_image_constructed();
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
        }
    }
}
