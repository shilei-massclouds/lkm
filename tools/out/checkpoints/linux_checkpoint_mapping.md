# Linux Checkpoint Mapping

- exact: 70
- range: 11
- unmapped: 321

| checkpoint_index | checkpoint_name | checkpoint_variant | mapping_kind | confidence | linux_file | linux_symbol | linux_anchor | notes |
| ---: | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | StartupTimeline.Started | StartupTimelineStarted | exact | high | init/main.c | start_kernel | start_kernel() definition line 903 | Linux C boot timeline entry anchor. |
| 1 | StartupTimeline.Ready | StartupTimelineReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 2 | PreparePhase.Ready | PreparePhaseReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 3 | PreparePhase.Online | PreparePhaseOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 4 | BootPhase.Started | BootPhaseStarted | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 5 | BootPhase.Ready | BootPhaseReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 6 | InterruptPhase.Started | InterruptPhaseStarted | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 7 | InterruptPhase.Ready | InterruptPhaseReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 8 | EntryPreludePhase.Started | EntryPreludePhaseStarted | exact | high | arch/riscv/kernel/head.S | _start | _start definition line 21 | RISC-V64 Linux boot image entry symbol; architecture-scoped head.S mapping. |
| 9 | EntryPreludePhase.Ready | EntryPreludePhaseReady | exact | high | arch/riscv/kernel/head.S | _start_kernel | _start_kernel line 330: tail start_kernel | RISC-V64 head.S handoff from _start_kernel to Linux start_kernel(). |
| 10 | EntrySuccessorPhase.Started | EntrySuccessorPhaseStarted | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 11 | EntrySuccessorPhase.Ready | EntrySuccessorPhaseReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 12 | EntryPreludePhase.Destroyed | EntryPreludePhaseDestroyed | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 13 | InterruptStream.Prepared | InterruptStreamPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 14 | InterruptStream.Ready | InterruptStreamReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 15 | KernelImage.Prepared | KernelImagePrepared | range | medium | arch/riscv/kernel/head.S | _start_kernel | _start_kernel lines 286-290: .Lclear_bss: .. .Lclear_bss_done: | RISC-V64 head.S BSS clear interval; not a portable Linux kernel-image object boundary. |
| 16 | RootStream.Prepared | RootStreamPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 17 | KernelImage.Ready | KernelImageReady | range | medium | arch/riscv/mm/init.c | setup_vm | setup_vm() lines 1092-1207: kernel_map.virt_addr = KERNEL_LINK_ADDR + kernel_map.virt_offset; .. create_kernel_page_table(early_pg_dir, true); | RISC-V64 setup_vm() kernel_map initialization through early kernel mapping construction. |
| 18 | BootCurrentCPU.Ready | BootCurrentCpuReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 19 | BootCurrentCPU.Online | BootCurrentCpuOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 20 | BootCPU.Prepared | BootCpuPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 21 | BootCPU.Ready | BootCpuReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 22 | BootCPU.Online | BootCpuOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 23 | BootCpuLocalInterrupt.Ready | BootCpuLocalInterruptReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 24 | BootCpuCurrentTask.Ready | BootCpuCurrentTaskReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 25 | CpuGroup.Prepared | CpuGroupPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 26 | InitTask.Prepared | InitTaskPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 27 | InitStack.Prepared | InitStackPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 28 | EventStream.Prepared | EventStreamPrepared | exact | medium | arch/riscv/kernel/head.S | _start_kernel | _start_kernel line 310: csrw CSR_TVEC, a3 | RISC-V64 early fallback trap-vector setup before setup_vm(); architecture-scoped mapping. |
| 29 | ExceptionStream.Prepared | ExceptionStreamPrepared | exact | medium | arch/riscv/kernel/head.S | _start_kernel | _start_kernel line 310: csrw CSR_TVEC, a3 | RISC-V64 early fallback exception path uses the temporary spin trap vector. |
| 30 | TrampolineVm.Ready | TrampolineVmReady | range | medium | arch/riscv/mm/init.c | setup_vm | setup_vm() lines 1180-1190: /* Setup trampoline PGD and PMD */ .. create_pmd_mapping(trampoline_pmd, kernel_map.virt_addr, | RISC-V64 setup_vm() trampoline page-table construction interval. |
| 31 | TrampolineVm.Online | TrampolineVmOnline | exact | high | arch/riscv/kernel/head.S | relocate_enable_mmu | relocate_enable_mmu line 106: csrw CSR_SATP, a0 | RISC-V64 relocate_enable_mmu loads the trampoline page directory into satp. |
| 32 | TrampolineVm.Destroyed | TrampolineVmDestroyed | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 33 | RawDtb.Prepared | RawDtbPrepared | exact | medium | arch/riscv/mm/init.c | setup_vm | setup_vm() line 1210: create_fdt_early_page_table(__fix_to_virt(FIX_FDT), dtb_pa); | RISC-V64 setup_vm() consumes the boot DTB physical address for early FDT mapping. |
| 34 | RawDtb.Ready | RawDtbReady | exact | high | arch/riscv/mm/init.c | create_fdt_early_page_table | create_fdt_early_page_table() line 990: dtb_early_pa = dtb_pa; | RISC-V64 early FDT helper records dtb_early_pa after creating the fixmap-backed DTB view. |
| 35 | FixMap.Ready | FixMapReady | range | medium | arch/riscv/mm/init.c | setup_vm | setup_vm() lines 1165-1178: /* Setup early PGD for fixmap */ .. create_pmd_mapping(fixmap_pmd, FIXADDR_START, | RISC-V64 setup_vm() early fixmap page-table construction interval. |
| 36 | EarlyVm.Prepared | EarlyVmPrepared | range | medium | arch/riscv/mm/init.c | setup_vm | setup_vm() lines 1163-1207: pt_ops_set_early(); .. create_kernel_page_table(early_pg_dir, true); | RISC-V64 setup_vm() early page-table preparation interval. |
| 37 | EarlyVm.Ready | EarlyVmReady | exact | high | arch/riscv/mm/init.c | setup_vm | setup_vm() line 1207: create_kernel_page_table(early_pg_dir, true); | RISC-V64 setup_vm() constructs the early kernel page table. |
| 38 | EarlyVm.Online | EarlyVmOnline | exact | high | arch/riscv/kernel/head.S | relocate_enable_mmu | relocate_enable_mmu line 122: csrw CSR_SATP, a2 | RISC-V64 relocate_enable_mmu switches from trampoline mappings to the early kernel page table. |
| 39 | Vm.Prepared | VmPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 40 | Vm.Ready | VmReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 41 | Vm.Online | VmOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 42 | KernelImage.Online | KernelImageOnline | exact | medium | arch/riscv/kernel/head.S | relocate_enable_mmu | relocate_enable_mmu line 114: load_global_pointer | RISC-V64 relocation boundary after virtual addressing is active; object equivalence is partial. |
| 43 | EventStream.Ready | EventStreamReady | exact | high | arch/riscv/kernel/head.S | _start | _start line 185: la a0, handle_exception | RISC-V64 formal trap-vector target in .Lsetup_trap_vector. |
| 44 | InitTask.Online | InitTaskOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 45 | InitStack.Ready | InitStackReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 46 | InitStack.Online | InitStackOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 47 | Soc.Prepared | SocPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 48 | EarlyDtb.Prepared | EarlyDtbPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 49 | EarlyDtb.Ready | EarlyDtbReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 50 | EarlyDtb.Destroyed | EarlyDtbDestroyed | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 51 | PlatformCpuInfo.Ready | PlatformCpuInfoReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 52 | PlatformCpuInfo.Online | PlatformCpuInfoOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 53 | PhysicalMemory.Ready | PhysicalMemoryReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 54 | PhysicalMemory.Online | PhysicalMemoryOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 55 | CommandLine.Prepared | CommandLinePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 56 | KernelCmdline.Ready | KernelCmdlineReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 57 | InitMM.Ready | InitMmReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 58 | EarlyIoremap.Ready | EarlyIoremapReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 59 | SBI.Ready | SbiReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 60 | EarlyParam.Ready | EarlyParamReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 61 | MemBlock.Prepared | MemBlockPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 62 | MemBlock.Ready | MemBlockReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 63 | MemBlock.Online | MemBlockOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 64 | MemBlock.Offline | MemBlockOffline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 65 | SwapperVm.Ready | SwapperVmReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 66 | SwapperVm.Online | SwapperVmOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 67 | EarlyVm.Destroyed | EarlyVmDestroyed | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 68 | PrintkBuffer.Prepared | PrintkBufferPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 69 | EarlyCon.Prepared | EarlyConPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 70 | EarlyCon.Ready | EarlyConReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 71 | EarlyCon.Online | EarlyConOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 72 | EarlyCon.Offline | EarlyConOffline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 73 | Serial8250Console.Online | Serial8250ConsoleOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 74 | Serial8250Console.IrqDrivenReady | Serial8250ConsoleIrqDrivenReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 75 | Serial8250ConsoleLongBurstIrqTx.Ready | Serial8250ConsoleLongBurstIrqTxReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 76 | Serial8250ConsoleTxQuiesce.Ready | Serial8250ConsoleTxQuiesceReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 77 | Serial8250RxLoopback.Ready | Serial8250RxLoopbackReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 78 | Serial8250RxBatchLoopback.Ready | Serial8250RxBatchLoopbackReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 79 | BootConsole.Offline | BootConsoleOffline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 80 | CorePreparePhase.Started | CorePreparePhaseStarted | exact | medium | init/main.c | start_kernel | start_kernel() line 925: setup_arch(&command_line); | Linux start_kernel() architecture setup call; RISC-V paging_init is inside setup_arch(). |
| 81 | CorePreparePhase.Ready | CorePreparePhaseReady | exact | medium | init/main.c | start_kernel | start_kernel() line 963: trap_init(); | Linux start_kernel() trap_init() call near the CorePreparePhase ready boundary. |
| 82 | DeviceTree.Ready | DeviceTreeReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 83 | Zones.Ready | ZonesReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 84 | ResourceTree.Ready | ResourceTreeReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 85 | CpuGroup.Ready | CpuGroupReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 86 | CacheBlockInfo.Ready | CacheBlockInfoReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 87 | CpuCapabilities.Ready | CpuCapabilitiesReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 88 | DmaCachePolicy.Ready | DmaCachePolicyReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 89 | StaticBranch.Ready | StaticBranchReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 90 | CommandLine.Ready | CommandLineReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 91 | SavedCommandLine.Ready | SavedCommandLineReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 92 | StaticCommandLine.Ready | StaticCommandLineReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 93 | SetupNrCpuIds.Checkpoint | SetupNrCpuIdsCheckpoint | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 94 | PerCpuStaticImage.Ready | PerCpuStaticImageReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 95 | PerCpuFirstChunk.Ready | PerCpuFirstChunkReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 96 | PerCpuOffsetTable.Ready | PerCpuOffsetTableReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 97 | PerCpuStorage.Ready | PerCpuStorageReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 98 | CpuHotplugState.Ready | CpuHotplugStateReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 99 | SecondParseEarlyParam.Checkpoint | SecondParseEarlyParamCheckpoint | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 100 | BootParam.Ready | BootParamReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 101 | PrintUnknownBootoptions.Checkpoint | PrintUnknownBootoptionsCheckpoint | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 102 | PayloadParam.Ready | PayloadParamReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 103 | Randomness.Prepared | RandomnessPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 104 | PrintkBuffer.Ready | PrintkBufferReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 105 | ExceptionTable.Ready | ExceptionTableReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 106 | ExceptionStream.Ready | ExceptionStreamReady | exact | high | arch/riscv/kernel/head.S | _start | _start line 185: la a0, handle_exception | RISC-V64 formal exception entry target installed by .Lsetup_trap_vector. |
| 107 | MmCoreInitPhase.Started | MmCoreInitPhaseStarted | exact | high | init/main.c | start_kernel | start_kernel() line 964: mm_core_init(); | Linux start_kernel() call site for mm_core_init(). |
| 108 | MmCoreInitPhase.Ready | MmCoreInitPhaseReady | exact | high | mm/mm_init.c | mm_core_init | mm_core_init() definition line 2636 | Linux mm_core_init() function boundary. |
| 109 | MemoryTopology.Ready | MemoryTopologyReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 110 | MemoryNode.Ready | MemoryNodeReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 111 | ZoneSet.Ready | ZoneSetReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 112 | PageMetadataMap.Ready | PageMetadataMapReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 113 | ZonelistSet.Ready | ZonelistSetReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 114 | PageAllocator.Prepared | PageAllocatorPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 115 | MemoryDebugHardening.Ready | MemoryDebugHardeningReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 116 | StackDepot.Ready | StackDepotReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 117 | Swiotlb.Ready | SwiotlbReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 118 | PageAllocator.Ready | PageAllocatorReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 119 | PageAllocator.MemBlockHandoffReady | PageAllocatorMemBlockHandoffReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 120 | SlubSubsystem.Prepared | SlubSubsystemPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 121 | SlubCacheRegistry.Ready | SlubCacheRegistryReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 122 | KmallocCaches.Ready | KmallocCachesReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 123 | SlubSubsystem.Ready | SlubSubsystemReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 124 | KernelGlobalAllocator.Ready | KernelGlobalAllocatorReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 125 | DynamicContainerRuntime.Ready | DynamicContainerRuntimeReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 126 | PageTableLockCache.Ready | PageTableLockCacheReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 127 | PageTableCaches.Ready | PageTableCachesReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 128 | VmapAreaCache.Ready | VmapAreaCacheReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 129 | VmapAddressSpace.Ready | VmapAddressSpaceReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 130 | VmapNodeSet.Ready | VmapNodeSetReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 131 | VmapBlockQueues.Ready | VmapBlockQueuesReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 132 | VfreeDeferredSet.Ready | VfreeDeferredSetReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 133 | VmallocAllocator.Ready | VmallocAllocatorReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 134 | Ioremap.Ready | IoremapReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 135 | MmStructCache.Ready | MmStructCacheReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 136 | MmCoreTrimmedPaths.Ready | MmCoreTrimmedPathsReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 137 | SchedInitPhase.Started | SchedInitPhaseStarted | exact | high | init/main.c | start_kernel | start_kernel() line 976: sched_init(); | Linux start_kernel() call site for sched_init(). |
| 138 | SchedInitPhase.Ready | SchedInitPhaseReady | exact | high | kernel/sched/core.c | sched_init | sched_init() definition line 8366 | Linux sched_init() function boundary. |
| 139 | PokingInit.Noop | PokingInitNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 140 | FtraceInit.TrimmedNoop | FtraceInitTrimmedNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 141 | EarlyTraceInit.Deferred | EarlyTraceInitDeferred | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 142 | SchedInitPreludeTrimmedPaths.Ready | SchedInitPreludeTrimmedPathsReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 143 | DefaultSchedRootDomain.Ready | DefaultSchedRootDomainReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 144 | BitWaitQueueTable.Prepared | BitWaitQueueTablePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 145 | BootIdleRcuReadSide.Prepared | BootIdleRcuReadSidePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 146 | Scheduler.Prepared | SchedulerPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 147 | BootRunQueue.Ready | BootRunQueueReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 148 | BootRunQueueLock.Ready | BootRunQueueLockReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 149 | BootInitPreemption.Ready | BootInitPreemptionReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 150 | BootIdleTask.Ready | BootIdleTaskReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 151 | BootIdlePiLock.Ready | BootIdlePiLockReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 152 | BootIdlePreemption.Ready | BootIdlePreemptionReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 153 | Scheduler.Ready | SchedulerReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 154 | Scheduler.Online | SchedulerOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 155 | SchedInitIrqsDisabled.Checkpoint | SchedInitIrqsDisabledCheckpoint | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 156 | RadixTree.Ready | RadixTreeReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 157 | MapleTree.Ready | MapleTreeReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 158 | Workqueue.Prepared | WorkqueuePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 159 | Softirq.Prepared | SoftirqPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 160 | TasksRcu.Prepared | TasksRcuPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 161 | RcuCore.Ready | RcuCoreReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 162 | TraceInit.Deferred | TraceInitDeferred | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 163 | ContextTrackingInit.TrimmedNoop | ContextTrackingInitTrimmedNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 164 | SchedInitTraceContextBoundaries.Ready | SchedInitTraceContextBoundariesReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 165 | Scheduler.PickNextTask.Exit | SchedulerPickNextTaskExit | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 166 | Scheduler.SwitchTo.Entry | SchedulerSwitchToEntry | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 167 | Scheduler.SwitchTo.Exit | SchedulerSwitchToExit | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 168 | Scheduler.Schedule | SchedulerSchedule | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 169 | Scheduler.Schedule.Exit | SchedulerScheduleExit | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 170 | IrqTimeInitPhase.Started | IrqTimeInitPhaseStarted | exact | high | init/main.c | start_kernel | start_kernel() line 1007: early_irq_init(); | Linux start_kernel() begins the IRQ/time init call interval at early_irq_init(). |
| 171 | IrqTimeInitPhase.Ready | IrqTimeInitPhaseReady | range | medium | init/main.c | start_kernel | start_kernel() lines 1007-1016: early_irq_init(); .. time_init(); | Linux start_kernel() IRQ/tick/timer/time initialization interval. |
| 172 | IrqController.Ready | IrqControllerReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 173 | RiscvIntc.Ready | RiscvIntcReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 174 | RiscvIrqStackSet.Ready | RiscvIrqStackSetReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 175 | IrqChipInitTable.Prepared | IrqChipInitTablePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 176 | PlicDriver.Prepared | PlicDriverPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 177 | IrqChipInitTable.Ready | IrqChipInitTableReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 178 | IrqDispatchTree.Ready | IrqDispatchTreeReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 179 | Plic.Ready | PlicReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 180 | PlicIrqDomain.Prepared | PlicIrqDomainPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 181 | PlicIrqDomain.Ready | PlicIrqDomainReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 182 | IrqHandlerRegistry.Ready | IrqHandlerRegistryReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 183 | TickBroadcast.Prepared | TickBroadcastPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 184 | Tick.Prepared | TickPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 185 | RcuInitNohz.TrimmedNoop | RcuInitNohzTrimmedNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 186 | IrqTimeTrimmedPaths.Prepared | IrqTimeTrimmedPathsPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 187 | TimerWheel.Ready | TimerWheelReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 188 | SrcuCore.Ready | SrcuCoreReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 189 | HrtimerCore.Ready | HrtimerCoreReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 190 | ClocksourceCore.Prepared | ClocksourceCorePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 191 | JiffiesClocksource.Prepared | JiffiesClocksourcePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 192 | Timekeeper.Ready | TimekeeperReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 193 | RiscvTimerProvider.Ready | RiscvTimerProviderReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 194 | TickBroadcast.Ready | TickBroadcastReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 195 | Tick.Ready | TickReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 196 | Softirq.Ready | SoftirqReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 197 | Randomness.Ready | RandomnessReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 198 | KfenceInit.TrimmedNoop | KfenceInitTrimmedNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 199 | IrqTimeTrimmedPaths.Ready | IrqTimeTrimmedPathsReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 200 | BootStackCanary.Ready | BootStackCanaryReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 201 | PerfEventCore.Ready | PerfEventCoreReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 202 | ProfileCore.Ready | ProfileCoreReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 203 | IpiMux.Ready | IpiMuxReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 204 | SbiIpi.Ready | SbiIpiReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 205 | SmpCallFunction.Ready | SmpCallFunctionReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 206 | LocalIrqEnablePhase.Started | LocalIrqEnablePhaseStarted | exact | high | init/main.c | start_kernel | start_kernel() line 1030: early_boot_irqs_disabled = false; | Linux start_kernel() clears the early IRQ-disabled guard before enabling local IRQs. |
| 207 | LocalIrqEnablePhase.Ready | LocalIrqEnablePhaseReady | exact | high | init/main.c | start_kernel | start_kernel() line 1031: local_irq_enable(); | Linux start_kernel() local_irq_enable() boundary. |
| 208 | InterruptStream.Online | InterruptStreamOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 209 | IrqOpenPreparePhase.Started | IrqOpenPreparePhaseStarted | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 210 | IrqOpenPreparePhase.Ready | IrqOpenPreparePhaseReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 211 | SlubSubsystem.FlushWorkqueueReady | SlubFlushWorkqueueReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 212 | TtyLineDisciplineRegistry.Prepared | TtyLineDisciplineRegistryPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 213 | ConsoleDriverSet.Prepared | ConsoleDriverSetPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 214 | Console.Prepared | ConsolePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 215 | IrqOpenPrepareTrimmedPaths.Prepared | IrqOpenPrepareTrimmedPathsPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 216 | IrqOpenPrepareTrimmedPaths.Ready | IrqOpenPrepareTrimmedPathsReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 217 | PanicLater.Clear | PanicLaterClearCheckpoint | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 218 | LockdepInit.Noop | LockdepInitNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 219 | LockingSelftest.Noop | LockingSelftestNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 220 | InitrdBounds.Trimmed | InitrdBoundsTrimmed | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 221 | PageAllocator.PerCpuPagesetsDeferred | PageAllocatorPerCpuPagesetsDeferred | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 222 | NumaPolicy.Noop | NumaPolicyNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 223 | AcpiEarly.Noop | AcpiEarlyNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 224 | LateTimeInit.Noop | LateTimeInitNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 225 | SchedClock.Ready | SchedClockReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 226 | DelayLoop.Ready | DelayLoopReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 227 | ArchCpuFinalize.Noop | ArchCpuFinalizeNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 228 | ProcessPreparePhase.Started | ProcessPreparePhaseStarted | exact | medium | init/main.c | start_kernel | start_kernel() line 1073: pid_idr_init(); | Linux start_kernel() process/task namespace preparation interval starts at pid_idr_init(). |
| 229 | ProcessPreparePhase.Ready | ProcessPreparePhaseReady | range | medium | init/main.c | start_kernel | start_kernel() lines 1073-1098: pid_idr_init(); .. delayacct_init(); | Linux start_kernel() process/task/credential/cache preparation interval before rest_init(). |
| 230 | RootPidNamespace.Ready | RootPidNamespaceReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 231 | AnonVmaCore.Ready | AnonVmaCoreReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 232 | X86EfiRuntimeSwitch.Trimmed | X86EfiRuntimeSwitchTrimmed | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 233 | TaskCreationCore.Prepared | TaskCreationCorePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 234 | CredentialCore.Prepared | CredentialCorePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 235 | VectorContext.Prepared | VectorContextPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 236 | UprobeCore.Ready | UprobeCoreReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 237 | TaskCreationCore.Ready | TaskCreationCoreReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 238 | ShadowCallStackInit.Noop | ShadowCallStackInitNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 239 | LockdepInitTask.Noop | LockdepInitTaskNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 240 | SignalCore.Prepared | SignalCorePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 241 | TaskFileContext.Prepared | TaskFileContextPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 242 | VmaCore.Prepared | VmaCorePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 243 | NsProxy.Prepared | NsProxyPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 244 | UtsNamespace.Prepared | UtsNamespacePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 245 | KeyringCore.Ready | KeyringCoreReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 246 | SecurityCore.Ready | SecurityCoreReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 247 | ProcessPrepareTrimmedPaths.Prepared | ProcessPrepareTrimmedPathsPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 248 | DbgLateInit.Noop | DbgLateInitNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 249 | NetNamespace.Deferred | NetNamespaceDeferred | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 250 | PageCache.Deferred | PageCacheDeferred | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 251 | SignalCore.SetupDeferred | SignalCoreSetupDeferred | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 252 | SeqFileCore.Deferred | SeqFileCoreDeferred | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 253 | Procfs.Deferred | ProcfsDeferred | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 254 | Nsfs.Deferred | NsfsDeferred | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 255 | Pidfs.Deferred | PidfsDeferred | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 256 | Cpuset.Noop | CpusetNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 257 | Cgroup.Noop | CgroupNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 258 | Taskstats.Noop | TaskstatsNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 259 | DelayAccounting.Noop | DelayAccountingNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 260 | AcpiSubsystem.Noop | AcpiSubsystemNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 261 | ArchPostAcpi.Noop | ArchPostAcpiNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 262 | Kcsan.Noop | KcsanNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 263 | UpMultitaskPhase.Started | UpMultitaskPhaseStarted | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 264 | UpMultitaskPhase.Ready | UpMultitaskPhaseReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 265 | BootInitRestInitPhase.Ready | BootInitRestInitPhaseReady | exact | high | init/main.c | rest_init | rest_init() definition line 701 | Linux rest_init() creates init/kthreadd and enters boot idle. |
| 266 | BootInitScheduleHandoffPhase.Ready | BootInitScheduleHandoffPhaseReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 267 | BootIdleEntryPhase.Ready | BootIdleEntryPhaseReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 268 | RcuCore.SchedulerStartingReady | RcuSchedulerStartingReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 269 | KernelInitTaskPiLock.Ready | KernelInitTaskPiLockReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 270 | KthreaddTaskPiLock.Ready | KthreaddTaskPiLockReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 271 | KthreaddReadyGateWaitLock.Ready | KthreaddReadyGateWaitLockReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 272 | KernelInitTask.Prepared | KernelInitTaskPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 273 | KernelInitTask.Ready | KernelInitTaskReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 274 | KernelInitTask.Online | KernelInitTaskOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 275 | NumaDefaultPolicy.Noop | NumaDefaultPolicyNoop | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 276 | KthreaddTask.Prepared | KthreaddTaskPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 277 | KthreaddTask.Ready | KthreaddTaskReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 278 | KthreaddTask.Online | KthreaddTaskOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 279 | KthreaddTask.GlobalRefBound | KthreaddTaskGlobalRefBound | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 280 | SimpleWaitQueue.Ready | SimpleWaitQueueReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 281 | Completion.Prepared | CompletionPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 282 | Completion.Ready | CompletionReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 283 | Completion.Online | CompletionOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 284 | SystemState.Prepared | SystemStatePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 285 | SystemState.Ready | SystemStateReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 286 | KthreaddReadyGate.Ready | KthreaddReadyGateReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 287 | KthreaddReadyGate.Online | KthreaddReadyGateOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 288 | BootIdleRuntime.Ready | BootIdleRuntimeReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 289 | PreSmpInitPhase.Started | PreSmpInitPhaseStarted | exact | high | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1562: smp_prepare_cpus(setup_max_cpus); | Linux kernel_init_freeable() starts pre-SMP preparation at smp_prepare_cpus(). |
| 290 | PreSmpInitPhase.Ready | PreSmpInitPhaseReady | range | medium | init/main.c | kernel_init_freeable | kernel_init_freeable() lines 1562-1570: smp_prepare_cpus(setup_max_cpus); .. lockup_detector_init(); | Linux kernel_init_freeable() pre-SMP preparation interval before smp_init(). |
| 291 | PageAllocator.FullGfpMaskOpen | PageAllocatorFullGfpMaskOpen | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 292 | CpuGroup.PreSmpReady | CpuGroupPreSmpReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 293 | Workqueue.Ready | WorkqueueReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 294 | VmstatCore.Prepared | VmstatCorePrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 295 | TasksRcu.Ready | TasksRcuReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 296 | PreSmpInitcalls.Ready | PreSmpInitcallsReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 297 | PreSmpBoundary.Ready | PreSmpBoundaryReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 298 | SmpRuntimePhase.Started | SmpRuntimePhaseStarted | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 299 | SmpRuntimePhase.Ready | SmpRuntimePhaseReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 300 | SmpBringupPhase.Started | SmpBringupPhaseStarted | exact | high | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1572: smp_init(); | Linux kernel_init_freeable() SMP bringup call. |
| 301 | SmpBringupPhase.Ready | SmpBringupPhaseReady | exact | medium | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1573: sched_init_smp(); | Linux kernel_init_freeable() scheduler SMP completion call after smp_init(). |
| 302 | SecondaryIdleTasks.Prepared | SecondaryIdleTasksPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 303 | SmpbootThreadsLock.Ready | SmpbootThreadsLockReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 304 | CpuHotplugSync.Prepared | CpuHotplugSyncPrepared | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 305 | CpuHotplugSync.CpuHotplugReadGuardUsed | CpuHotplugReadGuardUsed | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 306 | CpuHotplugSync.SmpbootThreadsMutexGuardUsed | SmpbootThreadsMutexGuardUsed | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 307 | CpuAddRemoveLock.Ready | CpuAddRemoveLockReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 308 | CpuStartProvider.CpuHotplugWriteGuardUsed | CpuHotplugWriteGuardUsed | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 309 | CpuRunningWaitLock.Ready | CpuRunningWaitLockReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 310 | CpuDoneUpWaitLock.Ready | CpuDoneUpWaitLockReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 311 | CpuStartProvider.Ready | CpuStartProviderReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 312 | CpuStartProvider.BootDataPublished | CpuStartProviderBootDataPublished | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 313 | CpuHotplugSync.CpuRunningObserved | CpuRunningObserved | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 314 | CpuHotplugSync.CpuRunningWaitLockGuardUsed | CpuRunningWaitLockGuardUsed | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 315 | SecondaryCpuStartupAck.Ready | SecondaryCpuStartupAckReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 316 | CpuHotplugSync.DoneUpObserved | CpuDoneUpObserved | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 317 | CpuHotplugSync.DoneUpWaitLockGuardUsed | CpuDoneUpWaitLockGuardUsed | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 318 | CpuGroup.SecondaryCpusOnline | SecondaryCpusOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 319 | SecondaryCpuOnlineAck.ApLocalSyncSummary | SecondaryCpuApLocalSyncSummary | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 320 | SecondaryCpuOnlineAck.Ready | SecondaryCpuOnlineAckReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 321 | SmpBringupBoundary.Ready | SmpBringupBoundaryReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 322 | RuntimeCorePhase.Started | RuntimeCorePhaseStarted | exact | medium | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1575: workqueue_init_topology(); | Linux kernel_init_freeable() runtime core follow-up interval starts after SMP scheduler setup. |
| 323 | RuntimeCorePhase.Ready | RuntimeCorePhaseReady | range | medium | init/main.c | kernel_init_freeable | kernel_init_freeable() lines 1575-1578: workqueue_init_topology(); .. page_alloc_init_late(); | Linux kernel_init_freeable() runtime core topology/async/padata/page-alloc-late interval. |
| 324 | Scheduler.SmpReady | SchedulerSmpReady | exact | medium | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1573: sched_init_smp(); | Linux kernel_init_freeable() sched_init_smp() call; reuses the SmpBringupPhase.Ready anchor but records the scheduler SMP runtime object fact. |
| 325 | Workqueue.TopologyReady | WorkqueueTopologyReady | exact | medium | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1575: workqueue_init_topology(); | Linux kernel_init_freeable() workqueue_init_topology() call; records the Workqueue topology object fact after SMP scheduler setup. |
| 326 | AsyncCore.DeferredReady | AsyncCoreDeferredReady | exact | medium | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1576: async_init(); | Linux kernel_init_freeable() async_init() call; arceos_ex keeps async internals deferred and maps only the deferred runtime-core boundary. |
| 327 | PadataCore.DeferredReady | PadataCoreDeferredReady | exact | medium | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1577: padata_init(); | Linux kernel_init_freeable() padata_init() call; maps the deferred padata boundary without expanding padata object details. |
| 328 | PageAllocator.LateReady | PageAllocatorLateReady | exact | medium | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1578: page_alloc_init_late(); | Linux kernel_init_freeable() page_alloc_init_late() call; records the PageAllocator late object fact without changing the earlier allocator lifecycle view. |
| 329 | RuntimeCoreBoundary.Ready | RuntimeCoreBoundaryReady | exact | medium | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1578: page_alloc_init_late(); | Linux kernel_init_freeable() page_alloc_init_late() call; marks the Runtime Core window end boundary before do_basic_setup(), not an independent Linux object. |
| 330 | InitcallPhase.Started | InitcallPhaseStarted | exact | high | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1580: do_basic_setup(); | Linux kernel_init_freeable() enters do_basic_setup(). |
| 331 | InitcallPhase.Ready | InitcallPhaseReady | exact | high | init/main.c | do_basic_setup | do_basic_setup() line 1366: do_initcalls(); | Linux do_basic_setup() initcall execution anchor. |
| 332 | CpusetSmp.TrimmedReady | CpusetSmpTrimmedReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 333 | DriverCore.DeferredReady | DriverCoreDeferredReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 334 | IrqProcView.DeferredReady | IrqProcViewDeferredReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 335 | CtorTable.Ready | CtorTableReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 336 | OfPlatformDefaultPopulate.ScanComplete | OfPlatformDefaultPopulateScanComplete | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 337 | VirtioBus.Ready | VirtioBusReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 338 | VirtioBus.DeviceAdded | VirtioBusDeviceAdded | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 339 | VirtioBlk.Ready | VirtioBlkReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 340 | VirtioBlk.ReadReady | VirtioBlkReadReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 341 | VirtioBlk.LiveReadSubmitted | VirtioBlkLiveReadSubmitted | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 342 | VirtioBlk.LiveReadCompleted | VirtioBlkLiveReadCompleted | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 343 | VirtioRng.EntropyReady | VirtioRngEntropyReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 344 | InitcallTable.Ready | InitcallTableReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 345 | Serial8250ConsoleBurstIrqTx.Ready | Serial8250ConsoleBurstIrqTxReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 346 | Serial8250ConsoleLongIrqTx.Ready | Serial8250ConsoleLongIrqTxReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 347 | TtyXmitFifoProbe.Ready | TtyXmitFifoProbeReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 348 | TtyWriteRuntimeTx.Ready | TtyWriteRuntimeTxReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 349 | TtyWriteBatchRuntimeTx.Ready | TtyWriteBatchRuntimeTxReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 350 | InitcallBoundary.Ready | InitcallBoundaryReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 351 | RootfsPhase.Started | RootfsPhaseStarted | exact | high | init/main.c | kernel_init_freeable | kernel_init_freeable() line 1593: prepare_namespace(); | Linux kernel_init_freeable() call site for prepare_namespace(). |
| 352 | RootfsPhase.Ready | RootfsPhaseReady | exact | high | init/do_mounts.c | prepare_namespace | prepare_namespace() definition line 464 | Linux prepare_namespace() rootfs preparation boundary. |
| 353 | KUnitRuntime.TrimmedReady | KUnitRuntimeTrimmedReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 354 | InitramfsSync.DeferredReady | InitramfsSyncDeferredReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 355 | RootfsConsole.DeferredReady | RootfsConsoleDeferredReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 356 | RootfsPrepareNamespacePaths.Ready | RootfsPrepareNamespacePathsReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 357 | RamdiskExecuteCommand.EaccessCheckpoint | RamdiskExecuteCommandEaccessCheckpoint | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 358 | RootFS.Online | RootFSOnline | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 359 | IntegrityKeys.DeferredReady | IntegrityKeysDeferredReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 360 | RootfsBoundary.Ready | RootfsBoundaryReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 361 | FinalizePhase.Started | FinalizePhaseStarted | exact | medium | init/main.c | kernel_init | kernel_init() line 1471: async_synchronize_full(); | Linux kernel_init() begins final async/initmem cleanup after kernel_init_freeable(). |
| 362 | FinalizePhase.Ready | FinalizePhaseReady | exact | high | init/main.c | kernel_init | kernel_init() line 1487: system_state = SYSTEM_RUNNING; | Linux kernel_init() marks SYSTEM_RUNNING before payload selection. |
| 363 | AsyncFullSync.DeferredReady | AsyncFullSyncDeferredReady | exact | medium | init/main.c | kernel_init | kernel_init() line 1471: async_synchronize_full(); | Linux kernel_init() waits for async __init work before initmem cleanup; arceos_ex models this as a deferred finalize boundary. |
| 364 | SystemState.FreeingInitmemCheckpoint | SystemStateFreeingInitmemCheckpoint | exact | high | init/main.c | kernel_init | kernel_init() line 1473: system_state = SYSTEM_FREEING_INITMEM; | Linux kernel_init() explicitly marks the system_state transition into initmem freeing. |
| 365 | InitMemoryCleanup.DeferredReady | InitMemoryCleanupDeferredReady | exact | medium | init/main.c | kernel_init | kernel_init() line 1478: free_initmem(); | Linux kernel_init() initmem cleanup call site; arceos_ex keeps this as a deferred cleanup boundary rather than a separate Linux object. |
| 366 | KernelMappingProtection.DeferredReady | KernelMappingProtectionDeferredReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 367 | PtiFinalize.TrimmedReady | PtiFinalizeTrimmedReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 368 | SystemState.Online | SystemStateOnline | exact | high | init/main.c | kernel_init | kernel_init() line 1487: system_state = SYSTEM_RUNNING; | Linux kernel_init() marks SYSTEM_RUNNING after initmem cleanup and before payload/init selection. |
| 369 | RcuCore.InkernelBootEnded | RcuInkernelBootEnded | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 370 | RcuBootEnd.Ready | RcuBootEndReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 371 | SysctlArgs.DeferredReady | SysctlArgsDeferredReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 372 | FinalizeBoundary.Ready | FinalizeBoundaryReady | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 373 | BreakpointException.Handled | BreakpointExceptionHandled | unmapped | none | null | null | null | No reliable Linux alignment rule is defined for this checkpoint in this mapping-only pass. |
| 374 | UserBoot.MainElfReady | UserBootMainElfReady | exact | high | fs/binfmt_elf.c | load_elf_binary | load_elf_binary() line 855: elf_phdata = load_elf_phdrs(elf_ex, bprm->file); | Boot-time init exec view of the main ELF program-header parse; this reuses the UserExec.MainElfReady Linux anchor but is reached from kernel_init() via kernel_execve(). |
| 375 | UserBoot.InterpreterReady | UserBootInterpreterReady | exact | high | fs/binfmt_elf.c | load_elf_binary | load_elf_binary() line 955: interp_elf_phdata = load_elf_phdrs(interp_elf_ex, | Boot-time init exec view of the PT_INTERP program-header parse when an interpreter is present; the same Linux loader anchor is used by runtime UserExec. |
| 376 | UserBoot.InitAttemptFailed | UserBootInitAttemptFailed | range | medium | init/main.c | try_to_run_init_process | try_to_run_init_process() lines 1397-1404: ret = run_init_process(init_filename); .. return ret; | Linux folds a default init candidate attempt, non-ENOENT diagnostic and fallback return into try_to_run_init_process(); there is no separate stage/reason checkpoint. |
| 377 | UserBoot.AddressSpaceSetupStart | UserBootAddressSpaceSetupStart | exact | medium | fs/binfmt_elf.c | load_elf_binary | load_elf_binary() line 996: retval = begin_new_exec(bprm); | Boot-time init exec reaches the first new-exec/address-space handoff in load_elf_binary(); Linux has no UserBoot-specific address-space object boundary. |
| 378 | UserInitProcess.EnterUserMode | UserModeEntry | exact | medium | arch/riscv/kernel/entry.S | ret_from_exception | ret_from_exception line 279: sret | RISC-V ret_from_exception final sret is the architecture-scoped boot init return-to-user handoff. |
| 379 | UserAddressSpace.Ready | UserAddressSpaceReady | exact | medium | fs/exec.c | exec_mmap | exec_mmap() line 1003: activate_mm(active_mm, mm); | Linux exec_mmap() installs and activates the new mm; Linux does not expose a separate UserAddressSpace object boundary. |
| 380 | SyscallTable.ExecveArgsReady | SyscallTableExecveArgsReady | exact | high | fs/exec.c | do_execveat_common | do_execveat_common() line 1935: retval = copy_strings(bprm->argc, argv, bprm); | Linux do_execveat_common() has copied filename, envp and argv into linux_binprm before bprm_execve(). |
| 381 | UserExec.MainElfReady | UserExecMainElfReady | exact | high | fs/binfmt_elf.c | load_elf_binary | load_elf_binary() line 855: elf_phdata = load_elf_phdrs(elf_ex, bprm->file); | Linux ELF loader has read the main executable program headers. |
| 382 | UserExec.InterpreterReady | UserExecInterpreterReady | exact | high | fs/binfmt_elf.c | load_elf_binary | load_elf_binary() line 955: interp_elf_phdata = load_elf_phdrs(interp_elf_ex, | Linux PT_INTERP path has opened and parsed the interpreter ELF program headers when an interpreter is present. |
| 383 | UserExec.AddressSpaceReady | UserExecAddressSpaceReady | range | medium | fs/binfmt_elf.c | load_elf_binary | load_elf_binary() lines 1014-1286: retval = setup_arg_pages(bprm, randomize_stack_top(STACK_TOP), .. mm->start_stack = bprm->p; | Linux load_elf_binary() stack setup, PT_LOAD mapping, interpreter load, ELF tables and mm layout interval. |
| 384 | UserExec.TrapFrameReady | UserExecTrapFrameReady | exact | high | arch/riscv/kernel/process.c | start_thread | start_thread() line 151: regs->epc = pc; | RISC-V start_thread() installs the user entry PC and stack in pt_regs for exec return. |
| 385 | UserExec.SatpReady | UserExecSatpReady | exact | medium | fs/exec.c | exec_mmap | exec_mmap() line 1003: activate_mm(active_mm, mm); | Linux exec_mmap() installs and activates the new mm; Linux has no separate arceos_ex satp token boundary. |
| 386 | UserExec.ContextReplaced | UserExecContextReplaced | exact | medium | fs/exec.c | begin_new_exec | begin_new_exec() line 1280: retval = exec_mmap(bprm->mm); | Linux begin_new_exec() crosses the point-of-no-return and hands the nascent exec mm to exec_mmap(). |
| 387 | UserExec.SatpSwitched | UserExecSatpSwitched | exact | medium | arch/riscv/kernel/entry.S | ret_from_exception | ret_from_exception line 265: csrw CSR_STATUS, a0 | RISC-V return-to-user path restores trap CSRs before sret; this is architecture-scoped and not a separate Linux satp object boundary. |
| 388 | UserExec.ReturnFrameReady | UserExecReturnFrameReady | exact | medium | arch/riscv/kernel/entry.S | ret_from_exception | ret_from_exception line 279: sret | RISC-V ret_from_exception reaches the final sret return-to-user boundary. |
| 389 | SyscallTable.OpenAt | SyscallTableOpenAt | exact | high | fs/open.c | SYSCALL_DEFINE4(openat) | SYSCALL_DEFINE4(openat) line 1446: return do_sys_open(dfd, filename, flags, mode); | Linux openat syscall wrapper dispatches to do_sys_open(). |
| 390 | SyscallTable.Read | SyscallTableRead | exact | high | fs/read_write.c | SYSCALL_DEFINE3(read) | SYSCALL_DEFINE3(read) line 722: return ksys_read(fd, buf, count); | Linux read syscall wrapper dispatches to ksys_read(). |
| 391 | SyscallTable.Write | SyscallTableWrite | exact | high | fs/read_write.c | SYSCALL_DEFINE3(write) | SYSCALL_DEFINE3(write) line 748: return ksys_write(fd, buf, count); | Linux write syscall wrapper dispatches to ksys_write(). |
| 392 | SyscallTable.Writev | SyscallTableWritev | exact | high | fs/read_write.c | SYSCALL_DEFINE3(writev) | SYSCALL_DEFINE3(writev) line 1184: return do_writev(fd, vec, vlen, 0); | Linux writev syscall wrapper dispatches to do_writev(). |
| 393 | SyscallTable.Close | SyscallTableClose | exact | high | fs/open.c | SYSCALL_DEFINE1(close) | SYSCALL_DEFINE1(close) line 1557: file = file_close_fd(fd); | Linux close syscall wrapper removes the fd entry before flushing and fput handling. |
| 394 | SyscallTable.NewFstatAt | SyscallTableNewFstatAt | exact | high | fs/stat.c | SYSCALL_DEFINE4(newfstatat) | SYSCALL_DEFINE4(newfstatat) line 505: error = vfs_fstatat(dfd, filename, &stat, flag); | Linux newfstatat syscall wrapper dispatches to vfs_fstatat() before stat copyout. |
| 395 | SyscallTable.SetTidAddress | SyscallTableSetTidAddress | exact | high | kernel/fork.c | SYSCALL_DEFINE1(set_tid_address) | SYSCALL_DEFINE1(set_tid_address) line 1922: current->clear_child_tid = tidptr; | Linux set_tid_address syscall wrapper records current->clear_child_tid and returns task_pid_vnr(). |
| 396 | SyscallTable.Clone | SyscallTableClone | exact | medium | kernel/fork.c | kernel_clone | kernel_clone() line 2786: p = copy_process(NULL, trace, NUMA_NO_NODE, args); | Mapped to shared kernel_clone() because the legacy clone syscall ABI wrapper is arch/config conditional on CONFIG_CLONE_BACKWARDS variants. |
| 397 | SyscallTable.Wait4 | SyscallTableWait4 | exact | high | kernel/exit.c | SYSCALL_DEFINE4(wait4) | SYSCALL_DEFINE4(wait4) line 1879: long err = kernel_wait4(upid, stat_addr, options, ru ? &r : NULL); | Linux wait4 syscall wrapper dispatches to kernel_wait4(). |
| 398 | UserChild.ParentWaitResumed | UserChildParentWaitResumed | exact | medium | kernel/exit.c | kernel_wait4 | kernel_wait4() line 1853: if (ret > 0 && stat_addr && put_user(wo.wo_stat, stat_addr)) | Linux kernel_wait4() wait completion and status copyout boundary before returning the child pid to the parent. |
| 399 | SyscallTable.Exit | SyscallTableExit | exact | medium | kernel/exit.c | do_group_exit | do_group_exit() line 1088: do_exit(exit_code); | Checkpoint covers the first slice of exit/exit_group; this anchor is the shared exit_group path while plain sys_exit reaches adjacent do_exit(). |
| 400 | PayloadPhase.Ready | PayloadPhaseReady | exact | medium | init/main.c | kernel_init | kernel_init() line 1492: do_sysctl_args(); | Linux kernel_init() reaches the post-finalize payload-selection boundary. |
| 401 | PayloadPhase.Online | PayloadPhaseOnline | exact | medium | init/main.c | kernel_init | kernel_init() line 1525: if (!try_to_run_init_process("/sbin/init") \|\| | Linux kernel_init() default init candidate handoff anchor. |
