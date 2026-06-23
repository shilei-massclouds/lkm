/*
 * Entry Successor Phase Specification
 *
 * This subphase starts at start_kernel() and ends after setup_arch()
 * completes paging_init(), where SwapperVm is online and MemBlock has been
 * enabled for later early allocation metadata growth.
 */

/*
 * InitMM 表示根任务关联的 init_mm 元数据。它连接 BootInitTask 与后续完整内核地址空间元数据，
 * 但不作为另一个页表对象与 Vm/SwapperVm 竞争。
 */
object InitMM: AddressSpaceObject {
    initial_state: State::Base;
    parent: BootInitTask;

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
                    BootInitTask.state == State::Online;
                    Lds.state == State::Online;
                }

                ensures {
                    init_mm_bounds_ready(InitMM, Lds);
                    init_task_active_mm_ready(BootInitTask, InitMM);
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
            init_task_active_mm_ready(BootInitTask, InitMM);
        }
    }
}

/*
 * MemBlock 表示早期物理内存区段和保留区段管理对象。
 * 固件占用内存不作为 OpenSBI 等具体固件的特例硬编码，而应通过 FDT header /memreserve/
 * 和 /reserved-memory 节点形成 FDT reserved ranges 后统一排除。
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
                    memblock_fdt_reserved_ranges_applied(MemBlock, EarlyDtb);
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
            memblock_fdt_reserved_ranges_applied(MemBlock, EarlyDtb);
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

        events {
            /*
             * Disable 对应 mm_core_init()/mem_init() 中 memblock_free_all() 完成后
             * 早期页分配服务退出普通运行路径。MemBlock 元数据仍保留，后续
             * page_alloc_init_late()/memblock_discard() 再推进 Cleanup。
             */
            on Event::Disable -> State::Offline {
                depends_on {
                    PageAllocator.state == State::Prepared;
                    Swiotlb.state == State::Ready;
                }

                ensures {
                    memblock_allocator_ready(MemBlock);
                    memblock_resize_allowed(MemBlock);
                    memblock_free_ranges_handed_to_page_allocator(MemBlock, PageAllocator);
                    memblock_metadata_retained_for_late_discard(MemBlock);
                }
            }
        }
    }

    /*
     * Offline 表示可释放页已经交接给 PageAllocator，但 memblock 私有元数据尚未销毁。
     */
    state State::Offline {
        invariant {
            memblock_allocator_ready(MemBlock);
            memblock_resize_allowed(MemBlock);
            memblock_free_ranges_handed_to_page_allocator(MemBlock, PageAllocator);
            memblock_metadata_retained_for_late_discard(MemBlock);
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
             * Setup 对应 parse_dtb() 的后续用途，提取 kernel command line、FDT reserved ranges
             * 并触发 MemBlock 候选区段建立。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    PhysicalMemory.state == State::Online;
                    PlatformCpuInfo.state == State::Online;
                }

                drives {
                    MemBlock.Event::Preset;
                    CommandLine.Event::Preset;
                }

                ensures {
                    early_dtb_parse_ready(EarlyDtb, RawDtb);
                    fdt_reserved_memory_ranges_ready(EarlyDtb, RawDtb);
                }
            }
        }
    }

    /*
     * Ready 表示早期 DTB 解析事实已可供 MemBlock 和 EarlyParam 使用。
     */
    state State::Ready {
        invariant {
            early_dtb_parse_ready(EarlyDtb, RawDtb);
            fdt_reserved_memory_ranges_ready(EarlyDtb, RawDtb);
            PlatformCpuInfo.state == State::Online;
            PhysicalMemory.state == State::Online;
            MemBlock.state == State::Prepared;
            CommandLine.state == State::Prepared;
        }

        events {
            /*
             * Cleanup 在本子阶段末尾退出早期 DTB 解析服务。
             */
            on Event::Cleanup -> State::Destroyed {
                depends_on {
                    MemBlock.state == State::Online;
                    EarlyParam.state == State::Ready;
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
 * CommandLine 表示启动命令行管理对象。它是顶层对象，管理 raw/saved/static
 * 三个命令行视图；参数解析对象 EarlyParam/BootParam/PayloadParam 归入 Params，
 * 不归入本对象。
 * 入口后继期只通过 Preset 建立 raw view，核心准备期再通过 Setup 建立 saved/static
 * 副本。
 */
object CommandLine: ResourceObject {
    initial_state: State::Base;

    /*
     * Base 表示尚未建立任何命令行视图。
     */
    state State::Base {
        events {
            /*
             * Preset 建立可供 EarlyParam 解析的 raw command line。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    RawDtb.state == State::Ready;
                }

                drives {
                    KernelCmdline.Event::Preset;
                }

                ensures {
                    command_line_raw_ready(CommandLine, KernelCmdline);
                }
            }
        }
    }

    /*
     * Prepared 表示 raw command line 已经可供早期参数解析。
     * Ready 状态在核心准备期由 CommandLine.Setup 推进。
     */
    state State::Prepared {
        invariant {
            command_line_raw_ready(CommandLine, KernelCmdline);
            KernelCmdline.state == State::Ready;
        }

        events {
            /*
             * Setup 对应 setup_command_line() 中建立 saved/static 两个命令行副本的部分。
             * 参数解析仍由 EarlyParam/BootParam/PayloadParam 各自按原时序完成。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    MemBlock.state == State::Online;
                    KernelCmdline.state == State::Ready;
                }

                drives {
                    SavedCommandLine.Event::Setup;
                    StaticCommandLine.Event::Setup;
                }

                ensures {
                    command_line_raw_ready(CommandLine, KernelCmdline);
                    command_line_copies_ready(CommandLine, SavedCommandLine, StaticCommandLine);
                    command_line_raw_preserved(CommandLine, KernelCmdline);
                }
            }
        }
    }

    /*
     * Ready 表示 raw/saved/static 三个命令行视图已经建立；后续 Param 对象可以
     * 按既有时序解析 static view 或 payload 边界。
     */
    state State::Ready {
        invariant {
            command_line_raw_ready(CommandLine, KernelCmdline);
            command_line_copies_ready(CommandLine, SavedCommandLine, StaticCommandLine);
            command_line_raw_preserved(CommandLine, KernelCmdline);
            KernelCmdline.state == State::Ready;
            SavedCommandLine.state == State::Ready;
            StaticCommandLine.state == State::Ready;
        }
    }
}

/*
 * KernelCmdline 表示 CommandLine 的 raw view，即从早期 DTB 中提取出来的原始
 * kernel command line 文本。它是 CommandLine 的子视图对象。
 */
object KernelCmdline: ResourceObject {
    initial_state: State::Base;
    parent: CommandLine;

    /*
     * Base 表示 raw command line 尚未从 EarlyDtb 中抽取。
     */
    state State::Base {
        events {
            /*
             * Preset 建立可供 EarlyParam 解析的原始命令行文本。
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
     * Ready 表示 raw command line 文本已经可供早期参数解析。
     */
    state State::Ready {
        invariant {
            kernel_cmdline_ready(KernelCmdline, RawDtb);
        }
    }
}

/*
 * Params 表示启动参数解析管理对象。它不同于 CommandLine：CommandLine 管理启动
 * 命令行的文本视图，Params 管理 EarlyParam/BootParam/PayloadParam 三类参数解析
 * 对象及其阶段性推进。Preset 驱动早期参数解析；Setup 在核心准备期驱动 BootParam
 * 和 PayloadParam，且实现必须在二者之间保留 print_unknown_bootoptions() checkpoint。
 */
object Params: KernelObject {
    initial_state: State::Base;

    /*
     * Base 表示任何参数解析对象尚未进入完成状态。
     */
    state State::Base {
        events {
            /*
             * Preset 对应入口后继期第一次 parse_early_param() 路径。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    CommandLine.state == State::Prepared;
                    SBI.state == State::Ready;
                    PrintkBuffer.state == State::Prepared;
                }

                drives {
                    EarlyParam.Event::Setup;
                }

                ensures {
                    params_early_ready(Params, EarlyParam);
                }
            }
        }
    }

    /*
     * Prepared 表示早期参数已经解析；普通 boot 参数和 payload 参数仍按后续时序解析。
     */
    state State::Prepared {
        invariant {
            params_early_ready(Params, EarlyParam);
            EarlyParam.state == State::Ready;
        }

        events {
            /*
             * Setup 在核心准备期继续推进普通 boot 参数和最终 payload 参数解析。
             * 该事件不重新推进 EarlyParam；第二次 parse_early_param() 只作为实现 checkpoint 保留。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    EarlyParam.state == State::Ready;
                    StaticCommandLine.state == State::Ready;
                }

                drives {
                    BootParam.Event::Setup;
                    PayloadParam.Event::Setup;
                }

                ensures {
                    params_early_ready(Params, EarlyParam);
                    params_boot_ready(Params, BootParam);
                    params_payload_ready(Params, PayloadParam);
                }
            }
        }
    }

    /*
     * Ready 状态在核心准备期由 Params.Setup 推进。
     */
    state State::Ready {
        invariant {
            params_early_ready(Params, EarlyParam);
            params_boot_ready(Params, BootParam);
            params_payload_ready(Params, PayloadParam);
            EarlyParam.state == State::Ready;
            BootParam.state == State::Ready;
            PayloadParam.state == State::Ready;
        }
    }
}

/*
 * EarlyParam 表示早期内核参数解析和分发机制。
 */
object EarlyParam: KernelObject {
    initial_state: State::Base;
    parent: Params;

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
                    CommandLine.state == State::Prepared;
                    SBI.state == State::Ready;
                    PrintkBuffer.state == State::Prepared;
                }

                drives {
                    EarlyCon.Event::Preset;
                    EarlyCon.Event::Setup;
                    EarlyCon.Event::Enable;
                }

                ensures {
                    early_params_dispatched(EarlyParam, KernelCmdline);
                    early_params_use_raw_command_line(EarlyParam, CommandLine);
                }
            }
        }
    }

    /*
     * Ready 表示早期参数解析和必要分发已完成。
     */
    state State::Ready {
        invariant {
            early_params_dispatched(EarlyParam, KernelCmdline);
            early_params_use_raw_command_line(EarlyParam, CommandLine);
            EarlyCon.state == State::Online;
        }
    }
}

/*
 * PrintkBuffer 表示 printk 的中间日志缓冲机制。
 */
context PrintkBufferSetupLocalInterruptContext: Context {
    /*
     * This context corresponds to Linux setup_log_buf() using
     * local_irq_save(flags) / local_irq_restore(flags) around the active
     * printk ring-buffer switch and the first copy of existing records. The
     * allocation and dynamic ring-buffer initialization before the switch, and
     * the remaining-record copy after the switch, are outside this protected
     * section. Ordered event body members let PrintkBuffer.Event::Setup express
     * that local guarded section directly.
     *
     * Because PrintkBuffer.Event::Setup is a Prepared -> Ready lifecycle event,
     * this guarded section is single-commit on the successful boot path. Guard
     * elision requires this lexical block to carry an explicit `only-once`
     * marker and pass model-tool reachability proof before relying on outer
     * Effective Context to erase guard code.
     */
    guard {
        entered_by {
            BootCpuLocalInterrupt.Event::SaveAndDisable;
        }

        exited_by {
            BootCpuLocalInterrupt.Event::Restore;
        }
    }

    obj_refs {
        PrintkBuffer;
        BootCpuLocalInterrupt;
    }
}

object PrintkBuffer: BufferObject {
    initial_state: State::Base;

    /*
     * Base 表示早期 printk 缓冲机制尚未准备。
     */
    state State::Base {
        events {
            /*
             * Preset 建立静态日志缓冲。arceos_ex 启动 banner 这类早期输出通过
             * 启动期内部输出前端或 PrintkBuffer.write(...) action 写入缓冲区，
             * 不依赖应用侧 axstd::println!，也不作为生命周期事件的后置状态。
             */
            on Event::Preset -> State::Prepared {
                ensures {
                    printk_buffer_static_storage_ready(PrintkBuffer);
                    printk_buffer_ready(PrintkBuffer);
                }
            }
        }
    }

    /*
     * Prepared 表示日志缓冲可接收输出，但后端控制台可能尚未启用。
     */
    state State::Prepared {
        invariant {
            printk_buffer_static_storage_ready(PrintkBuffer);
            printk_buffer_ready(PrintkBuffer);
        }

        events {
            /*
             * Setup 对应 setup_log_buf()，在 per-cpu 和启动参数准备后完成正式日志缓冲准备。
             * Linux 只在 active buffer switch 和既有 records 初次复制的局部临界区
             * 使用 local_irq_save()/local_irq_restore()；前置动态缓冲准备和后续
             * remaining-record copy 均在该局部临界区之外。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    MemBlock.state == State::Online;
                    PerCpuStorage.state == State::Ready;
                    BootParam.state == State::Ready;
                    BootCpuLocalInterrupt.state == State::Ready;
                    cpu_local_interrupts_disabled(BootCpuLocalInterrupt);
                }

                drives {
                    PrintkBuffer.Action::PrepareDynamicLogBuffer;
                }

                within PrintkBufferSetupLocalInterruptContext only-once {
                    drives {
                        PrintkBuffer.Action::SwitchActiveBufferAndCopyExistingRecords;
                    }

                    ensures {
                        printk_buffer_setup_local_irq_guard_used(PrintkBuffer, BootCpuLocalInterrupt);
                    }
                }

                drives {
                    PrintkBuffer.Action::CopyRemainingRecords;
                }

                ensures {
                    printk_buffer_static_storage_ready(PrintkBuffer);
                    printk_buffer_ready(PrintkBuffer);
                    printk_buffer_runtime_ready(PrintkBuffer);
                    printk_buffer_setup_prepared_dynamic_buffer(PrintkBuffer);
                    printk_buffer_setup_switched_active_buffer(PrintkBuffer);
                    printk_buffer_setup_copied_remaining_records(PrintkBuffer);
                }
            }
        }
    }

    /*
     * Ready 表示 printk 缓冲区已经完成 setup_log_buf() 对应的运行期准备。
     */
    state State::Ready {
        invariant {
            printk_buffer_static_storage_ready(PrintkBuffer);
            printk_buffer_ready(PrintkBuffer);
            printk_buffer_runtime_ready(PrintkBuffer);
            printk_buffer_records_preserved(PrintkBuffer);
            printk_buffer_setup_local_irq_save_restore_used(PrintkBuffer);
            printk_buffer_setup_local_irq_guard_used(PrintkBuffer, BootCpuLocalInterrupt);
            printk_buffer_setup_prepared_dynamic_buffer(PrintkBuffer);
            printk_buffer_setup_switched_active_buffer(PrintkBuffer);
            printk_buffer_setup_copied_remaining_records(PrintkBuffer);
        }
    }
}

/*
 * EarlyCon 表示正式 console 建立前的 SBI early console 后端。它只描述
 * backend lifecycle 和 direct backend access contract；BootConsole 才是
 * ConsoleRegistry 中包装 EarlyCon 的 CON_BOOT printk console entry。
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
                    CommandLine.state == State::Prepared;
                }

                ensures {
                    earlycon_sbi_config_ready(EarlyCon, KernelCmdline);
                    earlycon_config_uses_raw_command_line(EarlyCon, CommandLine);
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
            earlycon_config_uses_raw_command_line(EarlyCon, CommandLine);
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

                drives {
                    BootConsole.Event::Setup;
                    BootConsole.Event::Enable;
                    ConsoleRegistry.Event::Preset;
                }

                ensures {
                    earlycon_backend_online(EarlyCon);
                    printk_buffer_flushed_to_earlycon(PrintkBuffer, EarlyCon);
                    boot_console_registered(BootConsole, EarlyCon);
                    console_registry_has_boot_console(ConsoleRegistry, BootConsole);
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

        events {
            /*
             * Disable is driven by real console handoff. After this point
             * earlycon is no longer a valid printk backend for payload code.
             */
            on Event::Disable -> State::Offline {
                depends_on {
                    ConsoleRegistry.state == State::Ready;
                    console_registry_printk_route_real_console(ConsoleRegistry, Serial8250Console);
                }

                ensures {
                    earlycon_backend_disabled_after_handoff(EarlyCon, ConsoleRegistry);
                    earlycon_backend_access_panics_after_handoff(EarlyCon);
                    earlycon_offline_trace_emitted(EarlyCon);
                }
            }
        }
    }

    state State::Offline {
        invariant {
            earlycon_backend_disabled_after_handoff(EarlyCon, ConsoleRegistry);
            earlycon_backend_access_panics_after_handoff(EarlyCon);
            earlycon_offline_trace_emitted(EarlyCon);
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
 * CpuIdMap 表示逻辑 CPU ID 到 CPUObject 的映射表。入口后继期只建立
 * logical CPU 0 -> BootCPU 的基础映射；核心准备期在 CpuGroup.setup_smp()
 * 之后再完成当前阶段的 CPU 映射边界整理。Enable 保留给未来多 CPU 运行期
 * 或动态拓扑正式启用时使用。
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
             * Preset 建立 logical CPU 0 -> BootCPU 的基础映射。
             */
            on Event::Preset -> State::Prepared {
                depends_on {
                    BootCPU.state == State::Prepared;
                }

                ensures {
                    cpu_id_map_entry(CpuIdMap, 0, BootCPU);
                    cpu_id_map_boot_cpu_stable(CpuIdMap, BootCPU);
                }
            }
        }
    }

    /*
     * Prepared 表示 boot CPU 基础映射已建立，但尚未完成核心准备期 CPU 映射整理。
     */
    state State::Prepared {
        invariant {
            cpu_id_map_entry(CpuIdMap, 0, BootCPU);
            cpu_id_map_boot_cpu_stable(CpuIdMap, BootCPU);
        }

        events {
            /*
             * Setup 在 CpuGroup.setup_smp() 建立拓扑事实后，确认当前阶段可用的
             * CPU logical id 映射边界。它不启动 secondary CPU，也不占用 Enable。
             */
            on Event::Setup -> State::Ready {
                depends_on {
                    CpuGroup.state == State::Ready;
                }

                ensures {
                    cpu_id_map_ready(CpuIdMap, CpuGroup);
                    cpu_id_map_entry(CpuIdMap, 0, BootCPU);
                    cpu_id_map_boot_cpu_stable(CpuIdMap, BootCPU);
                    cpu_id_map_secondary_cpu_entries_ready(CpuIdMap, CpuGroup);
                    cpu_id_map_entries_have_unique_logical_ids(CpuIdMap);
                    cpu_id_map_entries_have_unique_hartids(CpuIdMap);
                    cpu_id_map_possible_cpu_boundary_ready(CpuIdMap, CpuGroup);
                }
            }
        }
    }

    /*
     * Ready 表示核心准备期需要的逻辑 ID 映射边界已建立。
     */
    state State::Ready {
        invariant {
            cpu_id_map_ready(CpuIdMap, CpuGroup);
            cpu_id_map_entry(CpuIdMap, 0, BootCPU);
            cpu_id_map_boot_cpu_stable(CpuIdMap, BootCPU);
            cpu_id_map_secondary_cpu_entries_ready(CpuIdMap, CpuGroup);
            cpu_id_map_entries_have_unique_logical_ids(CpuIdMap);
            cpu_id_map_entries_have_unique_hartids(CpuIdMap);
            cpu_id_map_possible_cpu_boundary_ready(CpuIdMap, CpuGroup);
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
                    BootInitTask.state == State::Online;
                    BootInitStack.state == State::Ready;
                    InterruptStream.state == State::Prepared;
                    RawDtb.state == State::Ready;
                    FixMap.state == State::Ready;
                    KernelImage.state == State::Online;
                }

                drives {
                    EntryPreludePhase.Event::Cleanup;
                    BootInitStack.Event::Enable;
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
                    Params.Event::Preset;
                    MemBlock.Event::Setup;
                    Vm.Event::Enable;
                    MemBlock.Event::Enable;
                    EarlyDtb.Event::Cleanup;
                }

                ensures {
                    early_boot_irqs_disabled_true();
                }

                deferred {
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
            early_boot_irqs_disabled_true();
            EntryPreludePhase.state == State::Destroyed;
            BootInitStack.state == State::Online;
            BootCPU.state == State::Online;
            CpuIdMap.state == State::Prepared;
            InterruptStream.state == State::Ready;
            PrintkBuffer.state == State::Prepared;
            EarlyDtb.state == State::Destroyed;
            KernelCmdline.state == State::Ready;
            InitMM.state == State::Ready;
            EarlyIoremap.state == State::Ready;
            SBI.state == State::Ready;
            Params.state == State::Prepared;
            EarlyParam.state == State::Ready;
            EarlyCon.state == State::Online;
            MemBlock.state == State::Online;
            Vm.state == State::Online;
            SwapperVm.state == State::Online;
            EarlyVm.state == State::Destroyed;
        }
    }
}
