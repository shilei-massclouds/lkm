# Charter Objects

Object-level charter notes live here when they need a dedicated topic file.

## Exec core objects

| Object | Authoritative charter |
| --- | --- |
| `CPU` | [`cpu.md`](cpu.md) |
| `CpuGroup` | [`cpu-group.md`](cpu-group.md) |
| `Scheduler` | [`scheduler.md`](scheduler.md) |
| `KernelImage` | [`kernel-image.md`](kernel-image.md) |
| `KernelAddrSpace` | [`kernel-address-space.md`](kernel-address-space.md) |
| `FixMap` | [`fix-map.md`](fix-map.md) |
| `LinearMap` | [`linear-map.md`](linear-map.md) |
| `UserSpaceReserve` | [`user-space-reserve.md`](user-space-reserve.md) |
| `RawDtb` | [`raw-dtb.md`](raw-dtb.md) |
| `Vm` | [`vm.md`](vm.md) |
| `PhysicalDirect` | [`physical-direct.md`](physical-direct.md) |
| `TrampolineVm` | [`trampoline-vm.md`](trampoline-vm.md) |
| `EarlyVm` | [`early-vm.md`](early-vm.md) |
| `SwapperVm` | [`swapper-vm.md`](swapper-vm.md) |
| `Task` | [`task.md`](task.md) |
| `TaskFlow` | [`task-flow.md`](task-flow.md) |
| `TrapType` | [`trap-type.md`](trap-type.md) |
| `InterruptType` | [`interrupt-type.md`](interrupt-type.md) |
| `ExceptionType` | [`exception-type.md`](exception-type.md) |
| `PageFaultExceptionType` | [`page-fault-exception-type.md`](page-fault-exception-type.md) |
| `SyscallExceptionType` | [`syscall-exception-type.md`](syscall-exception-type.md) |
| `BreakpointExceptionType` | [`breakpoint-exception-type.md`](breakpoint-exception-type.md) |
| `UnexpectedExceptionType` | [`unexpected-exception-type.md`](unexpected-exception-type.md) |
| `TrapFlowType` | [`trap-flow-type.md`](trap-flow-type.md) |
| `InterruptFlowType` | [`interrupt-flow-type.md`](interrupt-flow-type.md) |
| `ExceptionFlowType` | [`exception-flow-type.md`](exception-flow-type.md) |
| `PageFaultExceptionFlowType` | [`page-fault-exception-flow-type.md`](page-fault-exception-flow-type.md) |
| `SyscallExceptionFlowType` | [`syscall-exception-flow-type.md`](syscall-exception-flow-type.md) |
| `BreakpointExceptionFlowType` | [`breakpoint-exception-flow-type.md`](breakpoint-exception-flow-type.md) |
| `UnexpectedExceptionFlowType` | [`unexpected-exception-flow-type.md`](unexpected-exception-flow-type.md) |
| Runtime Type instance / `declare` | [`dynamic-instance-declaration.md`](dynamic-instance-declaration.md) |
| `ExecTransaction` | [`exec-transaction.md`](exec-transaction.md) |
| `UserStack` | [`user-stack.md`](user-stack.md) |
| `UserAddressSpace` | [`user-address-space.md`](user-address-space.md) |
| `UserFrame` | [`user-frame.md`](user-frame.md) |
| `UserProcessRegistry` | [`user-process-registry.md`](user-process-registry.md) |
| `SchedulerClockevent` | [`scheduler-clockevent.md`](scheduler-clockevent.md) |
| `BinaryFormatRegistry` | [`binary-format-registry.md`](binary-format-registry.md) |
| `ExecSyncBoundaries` | [`exec-sync-boundaries.md`](exec-sync-boundaries.md) |
| `ElfObject` | [`elf-object.md`](elf-object.md) |

The trap entries define CPU-owned resident resources and per-entry short-lived Flow occurrences. No global
Trap/Flow instance or compatibility stream alias exists. The exec entries are lifecycle objects in the shared boot/runtime exec pipeline; the runtime-instance topic
defines the common creation and identity mechanism used by any declared Type. `BinaryFormatHandler` and
`ElfBinaryFormat` are registry entry/type contracts; `ExecArguments`, `ExecError` and `ExecOwner` are value
types. None of them adds another four-layer object family.

Formal object specifications live under [`../../model/objects/`](../../model/objects/). Their complete
file-level implementation mapping is maintained in
[`../../coding/objects/README.md`](../../coding/objects/README.md); charter topics describe intent and do
not replace that coverage table.
