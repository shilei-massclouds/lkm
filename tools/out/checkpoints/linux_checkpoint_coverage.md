# Linux Checkpoint Mapping Coverage

- total checkpoints: 402

## Mapping Kind Counts

| mapping_kind | count |
| --- | ---: |
| exact | 76 |
| range | 11 |
| unmapped | 315 |

## Confidence Counts

| confidence | count |
| --- | ---: |
| none | 315 |
| medium | 47 |
| high | 40 |

## Mapped Linux File Counts

| linux_file | count |
| --- | ---: |
| init/main.c | 42 |
| arch/riscv/kernel/head.S | 10 |
| arch/riscv/mm/init.c | 7 |
| fs/binfmt_elf.c | 6 |
| fs/exec.c | 4 |
| arch/riscv/kernel/entry.S | 3 |
| fs/read_write.c | 3 |
| kernel/exit.c | 3 |
| fs/open.c | 2 |
| kernel/fork.c | 2 |
| arch/riscv/kernel/process.c | 1 |
| fs/stat.c | 1 |
| init/do_mounts.c | 1 |
| kernel/sched/core.c | 1 |
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
| VirtioBlk | 4 |
| BootCPU | 3 |
| Completion | 3 |
| CpuStartProvider | 3 |
| EarlyDtb | 3 |
| InitStack | 3 |
| InterruptStream | 3 |
| KernelInitTask | 3 |
| RcuCore | 3 |
| SlubSubsystem | 3 |
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
| SecondaryCpuOnlineAck | 2 |
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
| VirtioBus | 2 |
| Workqueue | 2 |

- singleton unmapped families: 183
