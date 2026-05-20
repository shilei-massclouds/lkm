/*
 * Entry Successor Phase Specification
 *
 * This subphase starts at start_kernel() and ends after setup_arch()
 * completes paging_init(), where SwapperVm is online and MemBlock has been
 * enabled for later early allocation metadata growth.
 */

/*
 * InitMM 表示根任务关联的 init_mm 元数据。它连接 InitTask 与后续完整内核地址空间元数据，
 * 但不作为另一个页表对象与 Vm/SwapperVm 竞争。
 */
object InitMM: AddressSpaceObject {
    initial_state: State::Base;
    parent: InitTask;

    /*
     * Base 表示 init_mm 元数据尚未填入内核映像边界。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 setup_initial_init_mm()，记录内核代码段、数据段和 brk 边界。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    InitTask.state == State::Online;
                    StaticObjects.state == State::Online;
                    Lds.state == State::Online;
                }

                ensures {
                    init_mm_bounds_ready(InitMM, Lds);
                    init_task_active_mm_ready(InitTask, InitMM);
                }
            }
        }
    }

    /*
     * Ready 表示 init_mm 已记录本阶段需要的内核映像边界并关联到根任务 active_mm。
     */
    state State::Ready {
        invariant {
            init_mm_bounds_ready(InitMM, Lds);
            init_task_active_mm_ready(InitTask, InitMM);
        }
    }
}

/*
 * MemBlock 表示早期物理内存区段和保留区段管理对象。
 */
object MemBlock: MemoryObject {
    initial_state: State::Base;

    attrs {
        usable_ranges: PhysRangeSet<Ram>;
        reserved_ranges: PhysRangeSet<ReservedMemory>;
    }

    /*
     * Base 表示尚未从早期 DTB 收集候选物理内存区段。
     */
    state State::Base {
        events {
            /*
             * Preset 由 EarlyDtb.Setup 触发，基于已发布的 PhysicalMemory 收集候选 RAM 区段和粗略边界。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    EarlyDtb.state == State::Prepared;
                    PhysicalMemory.state == State::Online;
                    RawDtb.state == State::Ready;
                }

                ensures {
                    memblock_candidate_ranges_ready(MemBlock, PhysicalMemory, RawDtb);
                }
            }
        }
    }

    /*
     * Prepared 表示候选物理内存区段已经从早期 DTB/FDT 信息中形成。
     */
    state State::Prepared {
        invariant {
            memblock_candidate_ranges_ready(MemBlock, PhysicalMemory, RawDtb);
        }

        events {
            /*
             * Setup 对应 setup_bootmem()，施加保留区、对齐、memory limit 和可映射范围约束。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    EarlyDtb.state == State::Ready;
                    MemBlock.state == State::Prepared;
                    KernelImage.state == State::Online;
                    RawDtb.state == State::Ready;
                    Config.state == State::Online;
                    PhysicalMemory.state == State::Online;
                }

                ensures {
                    memblock_candidate_ranges_ready(MemBlock, PhysicalMemory, RawDtb);
                    memblock_reserved_ranges_ready(MemBlock, KernelImage, RawDtb);
                    memblock_allocator_ready(MemBlock);
                }
            }
        }
    }

    /*
     * Ready 表示 MemBlock 已具备早期物理内存分配所需的安全约束。
     */
    state State::Ready {
        invariant {
            memblock_candidate_ranges_ready(MemBlock, PhysicalMemory, RawDtb);
            memblock_reserved_ranges_ready(MemBlock, KernelImage, RawDtb);
            memblock_allocator_ready(MemBlock);
        }

        events {
            /*
             * Enable 对应 memblock_allow_resize()，在完整线性映射可用后允许元数据扩展。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    MemBlock.state == State::Ready;
                    Vm.state == State::Online;
                    SwapperVm.state == State::Online;
                }

                ensures {
                    memblock_allocator_ready(MemBlock);
                    memblock_resize_allowed(MemBlock);
                }
            }
        }
    }

    /*
     * Online 表示 MemBlock 已允许元数据扩展，可继续服务后续早期内存管理。
     */
    state State::Online {
        invariant {
            memblock_allocator_ready(MemBlock);
            memblock_resize_allowed(MemBlock);
        }
    }
}

/*
 * EarlyDtb 表示入口后继期短暂存在的早期 DTB 解析对象。
 */
object EarlyDtb: ResourceObject {
    initial_state: State::Base;

    /*
     * Base 表示尚未把 RawDtb 解析为本子阶段需要的基础平台事实。
     */
    state State::Base {
        events {
            /*
             * Preset 提取最小平台事实：/cpus 发布 PlatformCpuInfo，/memory 发布 PhysicalMemory。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    RawDtb.state == State::Ready;
                    EarlyVm.state == State::Online;
                }

                drives {
                    PlatformCpuInfo.Event::Preset;
                    PlatformCpuInfo.Event::Enable;
                    PhysicalMemory.Event::Preset;
                    PhysicalMemory.Event::Enable;
                }

                ensures {
                    early_dtb_platform_facts_ready(EarlyDtb, RawDtb);
                }
            }
        }
    }

    /*
     * Prepared 表示基础平台事实已发布，后续对象可依赖 PlatformCpuInfo 与 PhysicalMemory。
     */
    state State::Prepared {
        invariant {
            early_dtb_platform_facts_ready(EarlyDtb, RawDtb);
            PlatformCpuInfo.state == State::Online;
            PhysicalMemory.state == State::Online;
        }

        events {
            /*
             * Setup 对应 parse_dtb() 的后续用途，提取 kernel command line 并触发 MemBlock 候选区段建立。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    PhysicalMemory.state == State::Online;
                    PlatformCpuInfo.state == State::Online;
                }

                drives {
                    MemBlock.Event::Preset;
                    KernelCmdline.Event::Preset;
                }

                ensures {
                    early_dtb_parse_ready(EarlyDtb, RawDtb);
                }
            }
        }
    }

    /*
     * Ready 表示早期 DTB 解析事实已可供 MemBlock 和 KernelParam 使用。
     */
    state State::Ready {
        invariant {
            early_dtb_parse_ready(EarlyDtb, RawDtb);
            PlatformCpuInfo.state == State::Online;
            PhysicalMemory.state == State::Online;
            MemBlock.state == State::Prepared;
            KernelCmdline.state == State::Ready;
        }

        events {
            /*
             * Cleanup 在本子阶段末尾退出早期 DTB 解析服务。
             */
            on Event::Cleanup -> State::Destroyed {
                depends_on {
                    MemBlock.state == State::Online;
                    KernelParam.state == State::Ready;
                }
            }
        }
    }

    /*
     * Destroyed 表示早期解析对象完成使命；RawDtb 物理内容并未因此消失。
     */
    state State::Destroyed {
        invariant {
            RawDtb.state == State::Ready;
            no_service(EarlyDtb);
        }
    }
}

/*
 * KernelCmdline 表示从早期 DTB 中提取出来的原始 kernel command line 文本。
 */
object KernelCmdline: ResourceObject {
    initial_state: State::Base;

    /*
     * Base 表示命令行文本尚未从 EarlyDtb 中抽取。
     */
    state State::Base {
        events {
            /*
             * Preset 建立可供 KernelParam 解析的原始命令行文本。
             */
            on Event::Preset -> State::Ready {
                depends_on {
                    RawDtb.state == State::Ready;
                }

                ensures {
                    kernel_cmdline_ready(KernelCmdline, RawDtb);
                }
            }
        }
    }

    /*
     * Ready 表示命令行文本已经可供早期参数解析。
     */
    state State::Ready {
        invariant {
            kernel_cmdline_ready(KernelCmdline, RawDtb);
        }
    }
}

/*
 * KernelParam 表示早期内核参数解析和分发机制。
 */
object KernelParam: KernelObject {
    initial_state: State::Base;

    /*
     * Base 表示早期参数尚未解析。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 parse_early_param()，当前要求处理 earlycon=sbi 并驱动 EarlyCon。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    KernelCmdline.state == State::Ready;
                    StaticObjects.state == State::Online;
                    SBI.state == State::Ready;
                    PrintkBuffer.state == State::Prepared;
                }

                drives {
                    EarlyCon.Event::Preset;
                    EarlyCon.Event::Setup;
                    EarlyCon.Event::Enable;
                }

                ensures {
                    early_params_dispatched(KernelParam, KernelCmdline);
                }
            }
        }
    }

    /*
     * Ready 表示早期参数解析和必要分发已完成。
     */
    state State::Ready {
        invariant {
            early_params_dispatched(KernelParam, KernelCmdline);
            EarlyCon.state == State::Online;
        }
    }
}

/*
 * PrintkBuffer 表示 printk 的中间日志缓冲机制。
 */
object PrintkBuffer: BufferObject {
    initial_state: State::Base;

    /*
     * Base 表示早期 printk 缓冲机制尚未准备。
     */
    state State::Base {
        events {
            /*
             * Preset 建立静态日志缓冲，并承接 Linux Banner 这类早期输出。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    StaticObjects.state == State::Online;
                }

                ensures {
                    printk_buffer_ready(PrintkBuffer);
                    linux_banner_buffered(PrintkBuffer);
                }
            }
        }
    }

    /*
     * Prepared 表示日志缓冲可接收输出，但后端控制台可能尚未启用。
     */
    state State::Prepared {
        invariant {
            printk_buffer_ready(PrintkBuffer);
            linux_banner_buffered(PrintkBuffer);
        }
    }
}

/*
 * EarlyCon 表示正式 console 建立前的 SBI early console 后端。
 */
object EarlyCon: ConsoleObject {
    initial_state: State::Base;

    /*
     * Base 表示尚未从 earlycon 参数记录后端配置。
     */
    state State::Base {
        events {
            /*
             * Preset 记录 earlycon=sbi 参数配置。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    KernelCmdline.state == State::Ready;
                }

                ensures {
                    earlycon_sbi_config_ready(EarlyCon, KernelCmdline);
                }
            }
        }
    }

    /*
     * Prepared 表示 earlycon=sbi 配置已经记录。
     */
    state State::Prepared {
        invariant {
            earlycon_sbi_config_ready(EarlyCon, KernelCmdline);
        }

        events {
            /*
             * Setup 基于 SBI 能力视图建立 early console 输出后端。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    SBI.state == State::Ready;
                }

                ensures {
                    earlycon_sbi_backend_ready(EarlyCon, SBI);
                }
            }
        }
    }

    /*
     * Ready 表示 SBI early console 后端已经建立，等待注册启用。
     */
    state State::Ready {
        invariant {
            earlycon_sbi_backend_ready(EarlyCon, SBI);
        }

        events {
            /*
             * Enable 注册并启用后端，同时 replay/flush PrintkBuffer 中的历史输出。
             */
            on Event::Enable -> State::Online {
                depends_on {
                    PrintkBuffer.state == State::Prepared;
                }

                ensures {
                    earlycon_backend_online(EarlyCon);
                    printk_buffer_flushed_to_earlycon(PrintkBuffer, EarlyCon);
                }
            }
        }
    }

    /*
     * Online 表示 early console 后端已经启用。
     */
    state State::Online {
        invariant {
            earlycon_backend_online(EarlyCon);
            printk_buffer_flushed_to_earlycon(PrintkBuffer, EarlyCon);
        }
    }
}

/*
 * EarlyIoremap 表示使用 FixMap FIX_BTMAP 区域的早期临时映射服务。
 */
object EarlyIoremap: AddressSpaceObject {
    initial_state: State::Base;

    /*
     * Base 表示早期临时映射服务元数据尚未初始化。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 early_ioremap_setup()，初始化 boot-time mapping slot 元数据。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    FixMap.state == State::Ready;
                }

                ensures {
                    early_ioremap_slots_ready(EarlyIoremap, FixMap);
                }
            }
        }
    }

    /*
     * Ready 表示后续可执行 early_ioremap()/early_iounmap() action。
     */
    state State::Ready {
        invariant {
            early_ioremap_slots_ready(EarlyIoremap, FixMap);
        }
    }
}

/*
 * SBI 表示内核基于固件 SBI 接口建立的平台服务能力视图。
 */
object SBI: PlatformServiceObject {
    initial_state: State::Base;

    /*
     * Base 表示尚未探测 SBI 规范版本、固件标识和扩展能力。
     */
    state State::Base {
        events {
            /*
             * Setup 对应 sbi_init()，只记录能力事实，不把具体 SBI 调用建模为生命周期事件。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    SbiSpec.state == State::Online;
                    OpenSbiFirmware.state == State::Online;
                }

                ensures {
                    sbi_capability_view_ready(SBI, SbiSpec, OpenSbiFirmware);
                }
            }
        }
    }

    /*
     * Ready 表示 SBI 能力视图已经可供后续对象依赖。
     */
    state State::Ready {
        invariant {
            sbi_capability_view_ready(SBI, SbiSpec, OpenSbiFirmware);
        }
    }
}

/*
 * CpuIdMap 表示逻辑 CPU ID 到 hartid 的基础映射。
 */
object CpuIdMap: HardwareObject {
    initial_state: State::Base;
    parent: CpuGroup;

    /*
     * Base 表示逻辑 ID 映射尚未建立。
     */
    state State::Base {
        events {
            /*
             * Preset 建立 logic id 0 -> BootCPU.hartid 的基础映射。
             */
            on Event::Preset -> State::Ready {
                depends_on {
                    BootCPU.state == State::Prepared;
                }

                ensures {
                    cpu_id_map_ready(CpuIdMap, 0, BootCPU);
                }
            }
        }
    }

    /*
     * Ready 表示基础逻辑 ID 映射已建立。
     */
    state State::Ready {
        invariant {
            cpu_id_map_ready(CpuIdMap, 0, BootCPU);
        }
    }
}

/*
 * EntrySuccessorPhase 表示入口后继子阶段对象。它承接入口前导期完成边界，
 * 并推进到 SwapperVm.Online 与 MemBlock.Online。
 */
object EntrySuccessorPhase: PhaseObject {
    initial_state: State::Base;
    parent: BootPhase;

    /*
     * Base 表示入口后继期刚开始，仍处于系统独占上下文。
     */
    state State::Base {
        invariant {
            interrupt_concurrency_closed();
            task_concurrency_closed();
            context_is(SystemExclusive);
        }

        events {
            /*
             * Setup 按 start_kernel/setup_arch 到 paging_init() 的最小核心路径编排对象推进。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    EntryPreludePhase.state == State::Ready;
                    Vm.state == State::Ready;
                    EarlyVm.state == State::Online;
                    InitTask.state == State::Online;
                    InitStack.state == State::Ready;
                    InterruptStream.state == State::Prepared;
                    RawDtb.state == State::Ready;
                    FixMap.state == State::Ready;
                    KernelImage.state == State::Online;
                }

                drives {
                    EntryPreludePhase.Event::Cleanup;
                    InitStack.Event::Enable;
                    EarlyDtb.Event::Preset;
                    CpuIdMap.Event::Preset;
                    InterruptStream.Event::Setup;
                    BootCPU.Event::Setup;
                    BootCPU.Event::Enable;
                    PrintkBuffer.Event::Preset;
                    EarlyDtb.Event::Setup;
                    InitMM.Event::Setup;
                    EarlyIoremap.Event::Setup;
                    SBI.Event::Setup;
                    KernelParam.Event::Setup;
                    MemBlock.Event::Setup;
                    Vm.Event::Enable;
                    MemBlock.Event::Enable;
                    EarlyDtb.Event::Cleanup;
                }

                deferred {
                    "jump_label_init() 后续抽象为 StaticKey/静态分支对象。"
                    "efi_init() 后续在支持 EFI 启动路径时抽象为 FirmwareInterface/EFI 对象。"
                }
            }
        }
    }

    /*
     * Ready 表示入口后继期核心路径完成，完整内核虚拟内存空间已经启用。
     */
    state State::Ready {
        invariant {
            interrupt_concurrency_closed();
            task_concurrency_closed();
            context_is(SystemExclusive);
            EntryPreludePhase.state == State::Destroyed;
            InitStack.state == State::Online;
            BootCPU.state == State::Online;
            CpuIdMap.state == State::Ready;
            InterruptStream.state == State::Ready;
            PrintkBuffer.state == State::Prepared;
            EarlyDtb.state == State::Destroyed;
            KernelCmdline.state == State::Ready;
            InitMM.state == State::Ready;
            EarlyIoremap.state == State::Ready;
            SBI.state == State::Ready;
            KernelParam.state == State::Ready;
            EarlyCon.state == State::Online;
            MemBlock.state == State::Online;
            Vm.state == State::Online;
            SwapperVm.state == State::Online;
            EarlyVm.state == State::Destroyed;
        }
    }
}
