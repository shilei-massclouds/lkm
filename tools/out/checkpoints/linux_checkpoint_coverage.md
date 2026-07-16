# Linux Checkpoint Mapping Coverage

- total checkpoints: 485

## Mapping Kind Counts

| mapping_kind | count |
| --- | ---: |
| exact | 103 |
| range | 14 |
| unmapped | 368 |

## Confidence Counts

| confidence | count |
| --- | ---: |
| none | 368 |
| medium | 62 |
| high | 55 |

## Mapped Linux File Counts

| linux_file | count |
| --- | ---: |
| init/main.c | 53 |
| arch/riscv/kernel/head.S | 14 |
| arch/riscv/mm/init.c | 7 |
| arch/riscv/kernel/smpboot.c | 6 |
| fs/binfmt_elf.c | 6 |
| arch/riscv/kernel/cpu_ops_sbi.c | 4 |
| fs/exec.c | 4 |
| arch/riscv/kernel/entry.S | 3 |
| fs/read_write.c | 3 |
| init/do_mounts.c | 3 |
| kernel/exit.c | 3 |
| fs/open.c | 2 |
| kernel/cpu.c | 2 |
| kernel/fork.c | 2 |
| arch/riscv/kernel/process.c | 1 |
| fs/stat.c | 1 |
| kernel/sched/core.c | 1 |
| kernel/sched/idle.c | 1 |
| mm/mm_init.c | 1 |

## Unmapped Checkpoint Family Counts

| family | count |
| --- | ---: |
| Scheduler | 8 |
| CpuHotplugSync | 7 |
| PageAllocator | 5 |
| BootIdleEntryPhase | 4 |
| BootInitScheduleHandoffPhase | 4 |
| BootPhase | 4 |
| CpuGroup | 4 |
| EarlyCon | 4 |
| EntrySuccessorPhase | 4 |
| InterruptPhase | 4 |
| IrqOpenPreparePhase | 4 |
| KthreaddTask | 4 |
| MemBlock | 4 |
| SmpRuntimePhase | 4 |
| SyscallTable | 4 |
| UpMultitaskPhase | 4 |
| VirtioBlk | 4 |
| BootCPU | 3 |
| BootInitRestInitPhase | 3 |
| Completion | 3 |
| EarlyDtb | 3 |
| InitStack | 3 |
| InterruptStream | 3 |
| KernelInitTask | 3 |
| SlubSubsystem | 3 |
| UserClone | 3 |
| Vm | 3 |
| ApEntryPreludePhase | 2 |
| ApOnlineIdlePhase | 2 |
| ApSmpCallinPhase | 2 |
| BootCurrentCPU | 2 |
| CommandLine | 2 |
| CorePreparePhase | 2 |
| EntryPreludePhase | 2 |
| FinalizePhase | 2 |
| InitTask | 2 |
| InitcallPhase | 2 |
| IrqChipInitTable | 2 |
| IrqOpenPrepareTrimmedPaths | 2 |
| IrqTimeInitPhase | 2 |
| IrqTimeTrimmedPaths | 2 |
| KthreaddReadyGate | 2 |
| LocalIrqEnablePhase | 2 |
| MmCoreInitPhase | 2 |
| PayloadPhase | 2 |
| PhysicalMemory | 2 |
| PlatformCpuInfo | 2 |
| PlicIrqDomain | 2 |
| PreSmpInitPhase | 2 |
| PreparePhase | 2 |
| PrintkBuffer | 2 |
| ProcessPreparePhase | 2 |
| Randomness | 2 |
| RcuCore | 2 |
| RootfsPhase | 2 |
| RuntimeCorePhase | 2 |
| SchedInitPhase | 2 |
| Serial8250Console | 2 |
| SignalCore | 2 |
| SmpBringupPhase | 2 |
| Softirq | 2 |
| SwapperVm | 2 |
| SystemState | 2 |
| TaskCreationCore | 2 |
| TasksRcu | 2 |
| Tick | 2 |
| TickBroadcast | 2 |
| UserChildRecord | 2 |
| UserSignalWait | 2 |
| UserStack | 2 |
| VirtioBus | 2 |
| Workqueue | 2 |

- singleton unmapped families: 172
