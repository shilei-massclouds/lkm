# EntryPreludePhase Coding

入口前导子阶段对应 model `EntryPreludePhase` 的完整 `Preset -> Setup -> Enable` 生命周期，实现
落点为 `impl/arceos_ex/src/phases/boot/entry_prelude.rs`。它是 BootPhase.Preset 的直接 child；
`_start` 虽由 OpenSBI 跳入，阶段所有权仍是 `Kernel.Preset drives Boot.Preset drives
EntryPrelude.Preset`。

## 入口例外与 adoption

`_start` 在任何对象 drive 前依次输出 `Kernel.Started` 的 `R`、`BootPhase.Started` 的 `B` 和
`EntryPreludePhase.Started` 的 `A`。三个 checkpoint 都是 Preset 接受事件，输出时对应状态仍为
Base。汇编随后完成必须发生在 Rust 前的 CSR/GPR/BSS 操作。

进入 `entry_prelude_rust_entry()` 后，先 adoption Prepare、Kernel、Boot 和 EntryPrelude 的入口
边界。EntryPrelude adoption 必须检查自身精确 Base 以及 model 的 Riscv64、SbiSpec、OpenSBI、
Lds、Config 依赖；不得再次输出 Started。Boot adoption 同时在这个最早 Rust 边界确认
`sstatus.SIE == 0`。

## Preset: Base -> Prepared

Preset 横跨三个物理实现段，但仍是一个 model transition：

| 段 | model drives 与实现 |
| --- | --- |
| `_start` head | `InterruptStream.Preset` 清 `sie/sip`；`KernelImage.Preset` 建立 `gp`；`RootStream.Preset` 禁用 FPU/vector；`KernelImage.Setup` 清 BSS；adopt boot hart、`init_task` 和 init stack |
| `preset_until_vm_switch()` | adoption head 对象事实；驱动 `BootCurrentCPU.Setup -> CpuGroup.Preset -> BootCurrentCPU.Enable`、`EventStream.Preset`、`ExceptionStream.Preset` 和 `Vm.Preset` |
| `after_vm_setup()` | `Vm.Setup` 地址空间 continuation 返回后驱动 `EventStream.Setup`、`BootInitTask.Enable`、`BootInitStack.Setup` 和 `Soc.Preset` |

`Vm.Setup` 必须在同一个 Preset 内完成 TrampolineVm 到 EarlyVm 的切换，并通过
`after_vm_setup_continuation()` 回到 EntryPrelude owner。所有 drives 成功后检查 Preset 后置对象
事实，提交 Prepared，读回并发出 `EntryPreludePhase.Prepared`，随后按 emits 调用 Setup。

## Setup: Prepared -> Ready

Setup start 检查精确 Prepared。该 transition 没有 drives；确认 Prepared 后置事实仍成立后提交
Ready，读回并发出 `EntryPreludePhase.Ready`，随后按 emits 调用 Enable。

## Enable: Ready -> Online

Enable start 检查精确 Ready，并验证 model Online invariant：Root/Interrupt/Exception/Event streams、
KernelImage、RawDtb、BootInitTask、BootInitStack、Vm/TrampolineVm/EarlyVm、BootCurrentCPU/BootCPU/
CpuGroup 和 Soc 必须处于 model 规定状态。成功后提交 Online，发出
`EntryPreludePhase.Online`，再返回 `boot::preset_after_entry_prelude()`。本阶段不得直接启动
EntrySuccessorPhase。

## 状态与 checkpoint

`ENTRY_PRELUDE_PHASE_STATE` 持久记录四状态；`is_online()` 只匹配 Online。

| Checkpoint | owner state | Position |
| --- | --- | --- |
| `EntryPreludePhase.Started` | Base | `_start` 的 `A`，Rust 不重复 |
| `EntryPreludePhase.Prepared` | Prepared | `after_vm_setup()` drives 完成并检查后 |
| `EntryPreludePhase.Ready` | Ready | Setup 提交后 |
| `EntryPreludePhase.Online` | Online | Enable 提交后、返回父 continuation 前 |

## Coding Constraints

- RISC-V early alternatives (`apply_early_boot_alternatives`) 在 `Vm.Preset` 中保持显式 deferred；
  不得假装 absent 或 implemented。
- head 汇编只执行 Rust 前不可延迟的架构动作；每个已完成动作由 Rust adoption 进入对象状态，
  不重复对应 checkpoint。
- TrampolineVm 到 EarlyVm 的 translation synchronization 和 continuation identity 必须保持；地址
  空间切换不能制造第二个 Preset 或绕过中间阶段状态。
- `Started` 不是第五种状态，不新增 `Phase.Base` checkpoint。
