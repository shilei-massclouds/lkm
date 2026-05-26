/*
 * Prepare Phase Specification
 *
 * Preparation inputs are modeled as already-online facts, then gathered
 * into the explicit PreparePhase boundary used by later boot phases.
 */

/*
 * Riscv64 表示当前模型引用的 RISC-V 64 位体系结构对象。
 * 它提供入口前导期需要读取或改写的通用寄存器和监管者 CSR。
 */
object Riscv64: IsaObject {
    initial_state: State::Online;
    source: external_spec::riscv_isa;

    attrs {
        a0: Gpr<HartId>;
        a1: Gpr<PhysAddr<Dtb>>;
        sp: Gpr<Addr>;
        tp: Gpr<Addr>;
        gp: Gpr<Addr>;

        sstatus: Csr<Sstatus>;
        sie: Csr<Sie>;
        sip: Csr<Sip>;
        stvec: Csr<TrapVector>;
        sscratch: Csr<usize>;
        satp: Csr<Satp>;
    }

    /*
     * Online 表示体系结构对象在推导起点已经可用。
     * 本状态只要求当前模型声明的体系结构属性都可读取。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
        }
    }
}

/*
 * BootArgs 表示启动 ABI 对入口寄存器的语义解释。
 * RISC-V64 下 a0 是启动 hartid，a1 是原始 dtb 物理地址。
 */
object BootArgs: PrepareObject {
    initial_state: State::Online;

    attrs {
        boot_hartid: HartId;
        dtb_pa: PhysAddr<Dtb>;
    }

    /*
     * Online 表示启动参数在入口前导期开始前已经由启动 ABI 给出。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            boot_hartid == Riscv64.a0;
            dtb_pa == Riscv64.a1;
        }
    }
}

/*
 * SbiSpec 表示 RISC-V SBI 规范中当前模型依赖的 HSM 语义。
 * 它描述规范能力，不描述具体固件版本的实现细节。
 */
object SbiSpec: PrepareObject {
    initial_state: State::Online;
    source: external_spec::riscv_sbi;

    state State::Online {
        invariant {
            sbi_hsm_available();
        }
    }
}

/*
 * OpenSbiFirmware 表示当前启动路径中由 OpenSBI 固件提供的 SBI 交接语义。
 * 它把 SBI 规范能力落实到本次内核入口的固件状态。
 */
object OpenSbiFirmware: PrepareObject {
    initial_state: State::Online;
    source: firmware::opensbi;

    state State::Online {
        invariant {
            SbiSpec.state == State::Online;
            BootArgs.state == State::Online;
            ordered_booting_enabled();
            primary_hart_only_at_kernel_entry();
            primary_hart_sie_clear_at_kernel_entry();
            firmware_dtb_blob_in_ram_at_kernel_entry(BootArgs.dtb_pa);
            firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa);
            firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa);
        }
    }
}

/*
 * Lds 表示链接脚本形成的内核映像布局对象。
 * 它提供符号地址、BSS 边界、根栈边界、内核映像边界和入口前导期早期代码布局。
 */
object Lds: PrepareObject {
    initial_state: State::Online;
    source: linker::linux_6_12_37;

    attrs {
        global_pointer: SymbolAddr;
        text_start: SymbolAddr;
        elf_entry: SymbolAddr;
        head_text_range: AddrRange;
        pre_mmu_text_range: AddrRange;
        trampoline_safe_text_range: AddrRange;
        bss_start: SymbolAddr;
        bss_end: SymbolAddr;
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
            elf_entry == kernel_start;
            kernel_end > kernel_start;
            entry_head_text_layout_ready(Lds);
            pre_mmu_access_discipline_ready(Lds);
            trampoline_access_discipline_ready(Lds);
            bss_start != 0;
            bss_end > bss_start;
            inside(bss_start, bss_end, kernel_start, kernel_end);
            init_stack_start != 0;
            init_stack_end > init_stack_start;
            page_aligned(init_stack_start);
            page_aligned(init_stack_end);
            boot_stack_size == Config.boot_stack_size;
            init_stack_end - init_stack_start == boot_stack_size;
        }
    }

    reference linux_6_12_37 {
        global_pointer = symbol("__global_pointer$");
        kernel_start = symbol("_start");
        text_start = symbol("_start");
        elf_entry = symbol("_start");
        head_text_range = section(".head.text");
        pre_mmu_text_range = section(".head.text");
        trampoline_safe_text_range = symbol_range("relocate_enable_mmu", ".Lsecondary_park");
        kernel_end = symbol("_end");
        bss_start = symbol("__bss_start");
        bss_end = symbol("__bss_stop");
        init_stack_start = symbol("init_thread_union");
        init_stack_end = expr("init_thread_union + THREAD_SIZE");
        boot_stack_size = symbol("THREAD_SIZE");
    }
}

/*
 * StaticObjects 表示构建和静态初始化阶段已经分配好的静态对象集合。
 * 入口前导期只引用这些底层存储或函数符号，不动态创建它们。
 */
object StaticObjects: PrepareObject {
    initial_state: State::Online;
    source: static::linux_6_12_37;

    attrs {
        init_task: ObjectStorage<InitTask>;
        early_event_entry: FunctionSymbol<EventEntryPrototype>;
        formal_event_entry: FunctionSymbol<EventEntryPrototype>;
        trampoline_pg_dir: PageTableStorage;
        early_pg_dir: PageTableStorage;
        swapper_pg_dir: PageTableStorage;
    }

    /*
     * Online 表示静态对象集合在推导起点已经可引用。
     * 本状态检查静态任务存储、事件入口符号和页表存储的基本有效性。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            valid_object_storage(init_task);
            valid_function_symbol(early_event_entry);
            valid_function_symbol(formal_event_entry);
            valid_page_table_storage(trampoline_pg_dir);
            valid_page_table_storage(early_pg_dir);
            valid_page_table_storage(swapper_pg_dir);
        }
    }

    reference linux_6_12_37 {
        init_task = symbol("init_task");
        early_event_entry = symbol(".Lsecondary_park");
        formal_event_entry = symbol("handle_exception");
        trampoline_pg_dir = symbol("trampoline_pg_dir");
        early_pg_dir = symbol("early_pg_dir");
        swapper_pg_dir = symbol("swapper_pg_dir");
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

    reference linux_6_12_37 {
        kernel_link_addr = symbol("KERNEL_LINK_ADDR");
        kernel_image_va_window_size = symbol("SZ_2G");
    }
}

/*
 * PhysicalMemory 表示平台提供的物理 RAM 和设备 I/O 地址布局。
 * 它由入口后继期 EarlyDtb.Preset 从 RawDtb 的 /memory 描述中建立。
 */
object PhysicalMemory: PrepareObject {
    initial_state: State::Base;
    access: Access::ReadOnly;
    source: fdt::memory;

    attrs {
        ram: PhysRangeSet<Ram>;
        iomap: PhysRangeSet<Io>;
    }

    /*
     * Base 表示物理内存事实尚未从 RawDtb 的 /memory 描述中抽取。
     */
    state State::Base {
        events {
            /*
             * Preset 由 EarlyDtb.Preset 触发，解析 /memory 并形成平台物理内存布局事实。
             */
            on Event::Preset -> State::Ready {
                depends_on {
                    RawDtb.state == State::Ready;
                }

                ensures {
                    physical_memory_ranges_ready(PhysicalMemory, RawDtb);
                }
            }
        }
    }

    /*
     * Ready 表示 FDT /memory 已解析为物理资源布局事实，等待发布为后续对象可依赖输入。
     */
    state State::Ready {
        invariant {
            attrs_accessible(self);
            readonly(self);
            valid_phys_range_set(ram);
            valid_phys_range_set(iomap);
            disjoint(ram, iomap);
            physical_memory_ranges_ready(PhysicalMemory, RawDtb);
        }

        events {
            /*
             * Enable 将已解析的物理内存布局发布为后续 MemBlock 可依赖的事实。
             */
            on Event::Enable -> State::Online {
                ensures {
                    physical_memory_ranges_published(PhysicalMemory);
                }
            }
        }
    }

    /*
     * Online 表示物理资源布局已经从 RawDtb 的 FDT /memory 描述中抽取并可读取。
     * 本状态要求 RAM 和 I/O 范围结构良好、非空且互不重叠。
     */
    state State::Online {
        invariant {
            attrs_accessible(self);
            readonly(self);
            valid_phys_range_set(ram);
            valid_phys_range_set(iomap);
            disjoint(ram, iomap);
            physical_memory_ranges_published(PhysicalMemory);
        }
    }
}

/*
 * PlatformCpuInfo 表示从平台 CPU 描述中提取出的有效 hart 集合。
 * 当前只建模启动参数中的 hart id 是否属于该集合，完整 CPU 拓扑留给后续阶段。
 */
object PlatformCpuInfo: PrepareObject {
    initial_state: State::Base;
    source: fdt::cpus;

    /*
     * Base 表示平台 CPU 事实尚未从 RawDtb 的 /cpus 描述中抽取。
     */
    state State::Base {
        events {
            /*
             * Preset 由 EarlyDtb.Preset 触发，解析 /cpus 并确认启动 hart 属于平台有效集合。
             */
            on Event::Preset -> State::Ready {
                depends_on {
                    BootArgs.state == State::Online;
                    RawDtb.state == State::Ready;
                }

                ensures {
                    platform_cpu_info_ready(PlatformCpuInfo, RawDtb);
                    platform_hart_id_valid(BootArgs.boot_hartid);
                }
            }
        }
    }

    /*
     * Ready 表示 FDT /cpus 已解析为平台 CPU 事实，且启动 hartid 已通过该集合验证。
     */
    state State::Ready {
        invariant {
            platform_cpu_info_ready(PlatformCpuInfo, RawDtb);
            platform_hart_id_valid(BootArgs.boot_hartid);
        }

        events {
            /*
             * Enable 将平台 CPU 事实发布为 BootCPU 后续推进可依赖的输入。
             */
            on Event::Enable -> State::Online {
                ensures {
                    platform_cpu_info_published(PlatformCpuInfo);
                }
            }
        }
    }

    state State::Online {
        invariant {
            platform_cpu_info_published(PlatformCpuInfo);
            platform_hart_id_valid(BootArgs.boot_hartid);
        }
    }
}

/*
 * PreparePhase 表示准备期阶段对象。
 * 当前模型不展开准备期内部过程，只验证入口前导期依赖的启动 ABI、固件、链接布局、
 * 静态对象和配置输入均已在线且满足各自不变量。
 */
object PreparePhase: PhaseObject {
    initial_state: State::Base;
    parent: StartupTimeline;

    /*
     * Base 表示准备期阶段对象已经进入模型空间，但尚未形成当前规格所需的准备期完成边界。
     */
    state State::Base {
        events {
            /*
             * Setup 汇总并验证入口前导期所需的准备期输入对象，形成准备期完成边界。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    Riscv64.state == State::Online;
                    SbiSpec.state == State::Online;
                    OpenSbiFirmware.state == State::Online;
                    Lds.state == State::Online;
                    StaticObjects.state == State::Online;
                    Config.state == State::Online;
                }
            }
        }
    }

    /*
     * Ready 表示入口前导期所需的准备期输入对象均已通过阶段边界验证。
     */
    state State::Ready {
        invariant {
            Riscv64.state == State::Online;
            SbiSpec.state == State::Online;
            OpenSbiFirmware.state == State::Online;
            Lds.state == State::Online;
            StaticObjects.state == State::Online;
            Config.state == State::Online;
        }

        events {
            /*
             * Enable 将准备期完成边界发布为后续引导期可依赖的输入边界。
             * 当前不执行额外动作，只保留阶段生命周期中的显式边界。
             */
            on Event::Enable -> State::Online {
            }
        }
    }

    /*
     * Online 表示准备期输入边界已经对后续引导期生效。
     */
    state State::Online {
        invariant {
            Riscv64.state == State::Online;
            SbiSpec.state == State::Online;
            OpenSbiFirmware.state == State::Online;
            Lds.state == State::Online;
            StaticObjects.state == State::Online;
            Config.state == State::Online;
        }
    }
}
