/*
 * Kernel Project Specification
 *
 * KernelProject models the lifecycle of the kernel engineering work product:
 * establish the kernel system specification, build the image, then boot and
 * evaluate the resulting kernel instance.
 *
 * Four-level chain:
 * spec/charter/projects/kernel.md -> this model -> spec/coding/projects/kernel.spec
 * -> impl/arceos_ex/src/projects/kernel.rs.
 */

/*
 * Lds 表示链接脚本形成的内核映像布局对象。
 * 它提供符号地址、BSS 边界、根栈边界、内核映像边界和入口前导期早期代码布局。
 */
object Lds: PrepareObject {
    initial_state: State::Online;
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

    /*
     * Online 表示链接布局在入口前导期开始前已经确定。
     * 本状态约束关键符号存在、范围有序且栈边界页对齐。
     */
    state State::Online {
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

/*
 * Config 表示入口前导期可见的构建配置和静态参数。
 * 它约束页大小、内核虚拟区域、地址转换模式和 fixmap 布局。
 */
object Config: PrepareObject {
    initial_state: State::Online;
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
    }

    /*
     * Online 表示配置对象在入口前导期开始前已经确定。
     * 本状态保证当前推导依赖的配置项存在并满足基本边界约束。
     */
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

include "../systems/kernel.spec";

object KernelProject: ProjectObject {
    initial_state: State::Base;
    parent: ComputerProject;

    /*
     * Base 表示内核工程已经进入模型空间，但尚未建立内核系统规格。
     */
    state State::Base {
        transitions {
            /*
             * Preset 建立内核系统规格和工程模型前置。
             */
            on Transition::Preset -> State::Prepared {
                ensures {
                    kernel_system_spec_established();
                }

                emits {
                    Transition::Setup;
                }
            }
        }
    }

    /*
     * Prepared 表示内核系统规格已经建立，可以在规格约束下生成代码并组装 image。
     */
    state State::Prepared {
        invariant {
            kernel_system_spec_established();
        }

        transitions {
            /*
             * Setup 覆盖 lds/conf/image layout 前置，生成代码并构造内核 image。
             */
            on Transition::Setup -> State::Ready {
                depends_on {
                    Lds.state == State::Online;
                    Config.state == State::Online;
                }

                ensures {
                    kernel_image_constructed();
                }

                emits {
                    Transition::Enable;
                }
            }
        }
    }

    /*
     * Ready 表示内核 image 已经构造完成，可以启动并评估。
     */
    state State::Ready {
        invariant {
            Lds.state == State::Online;
            Config.state == State::Online;
            kernel_image_constructed();
        }

        transitions {
            /*
             * Enable 启动内核实例并执行测试/评估；Kernel 自身的完成事件链
             * 负责从 Preset 自动推进到 Online。
             */
            on Transition::Enable -> State::Online {
                drives {
                    Kernel.Transition::Preset;
                }
            }
        }
    }

    /*
     * Online 表示内核工程产物已经启动为在线内核实例。
     */
    state State::Online {
        invariant {
            Kernel.state == State::Online;
        }
    }
}
