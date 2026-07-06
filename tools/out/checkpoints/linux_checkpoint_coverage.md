# Linux Checkpoint Mapping Coverage

- total checkpoints: 429

## Mapping Kind Counts

| mapping_kind | count |
| --- | ---: |
| exact | 103 |
| range | 14 |
| unmapped | 312 |

## Confidence Counts

| confidence | count |
| --- | ---: |
| none | 312 |
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
| CpuGroup | 4 |
| EarlyCon | 4 |
| KthreaddTask | 4 |
| MemBlock | 4 |
| SyscallTable | 4 |
| VirtioBlk | 4 |
| BootCPU | 3 |
| Completion | 3 |
| EarlyDtb | 3 |
| InitStack | 3 |
| InterruptStream | 3 |
| KernelInitTask | 3 |
| SlubSubsystem | 3 |
| UserClone | 3 |
| Vm | 3 |
| BootCurrentCPU | 2 |
| BootPhase | 2 |
| CommandLine | 2 |
| EntrySuccessorPhase | 2 |
| InitTask | 2 |
| InterruptPhase | 2 |
| IrqChipInitTable | 2 |
| IrqOpenPreparePhase | 2 |
| IrqOpenPrepareTrimmedPaths | 2 |
| IrqTimeTrimmedPaths | 2 |
| KthreaddReadyGate | 2 |
| PhysicalMemory | 2 |
| PlatformCpuInfo | 2 |
| PlicIrqDomain | 2 |
| PreparePhase | 2 |
| PrintkBuffer | 2 |
| Randomness | 2 |
| RcuCore | 2 |
| Serial8250Console | 2 |
| SignalCore | 2 |
| SmpRuntimePhase | 2 |
| Softirq | 2 |
| SwapperVm | 2 |
| SystemState | 2 |
| TaskCreationCore | 2 |
| TasksRcu | 2 |
| Tick | 2 |
| TickBroadcast | 2 |
| UpMultitaskPhase | 2 |
| UserChildRecord | 2 |
| UserSignalWait | 2 |
| VirtioBus | 2 |
| Workqueue | 2 |

- singleton unmapped families: 175
