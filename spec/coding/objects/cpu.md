# CPU coding contract

`Cpu` is the only Rust body for every logical CPU. Its canonical identity is the containing
`CpuGroup.cpus[logic_id]` slot; boot/AP names are formatting aliases and never allocate state.

- Store the authoritative `hartid`, lifecycle/possible/present/active/online facts and optional
  `active_translation_controller` in `Cpu`; derive logical ID from the slot. The entry assembly's saved boot-hart
  word is only a handoff value for the later `CPU.Action::AssignHartid`, not another CPU owner and not an assembly
  requirement to address `Cpu`. The association is an atomic CPU-local controller ref, not a page-table
  state copy. Raw byte `0` means unbound/absent only; `1..=4` encode PhysicalDirect, TrampolineVm, EarlyVm and
  SwapperVm. `TranslationController` itself has only those four real values. Any other nonzero byte is invalid and
  must produce `InvalidTranslationControllerEncoding` or fail-stop on a critical entry path.
- Keep CPU-local `InterruptType` and related local state inside `Cpu`; do not store a current-task slot.
- Do not expose `CpuView` or copy CPU facts into an identity-bearing projection.
- Lower `CpuRef` to a compact logical ID. Every dereference goes through `CpuGroup` and rejects an absent slot.
- Lower `CurrentCPU` as a stateless `CurrentCpu` borrow created from an effective `TaskFlow` plus `CpuGroup`.
  Resolve `CurrentTask` first, validate its active Flow, then dereference that Flow's CpuRef and prove the
  Task/Flow/CPU triple agrees. It cannot be constructed from a raw logical ID or an asynchronous emission.

Trace/checkpoint code that receives this capability records the canonical `CpuGroup.cpus[i]` target and the
source Flow/CpuRef. Recording only `CurrentCPU` is insufficient.

## 浮点与向量执行状态

`CPU.Action::DisableFpuVectorExecution` 必须作用于通过 effective TaskFlow 解析出的 canonical CPU。
在 RISC-V S-mode 中，它按以下单条状态更新 lowering：

```asm
li   t0, SR_FS_VS
csrc CSR_STATUS, t0
```

其中 `SR_FS_VS == SR_FS | SR_VS`，当前 S-mode 的 `CSR_STATUS` 是 `sstatus`；等价的
`csrrc zero, sstatus, t0` 也满足约束。该更新必须同时把当前 CPU 的 FS、VS 字段置为 Off，并位于
`KernelImage.Preset` 建立相对 `gp` 寻址基准之后、`KernelImage.Setup` 清理 BSS 之前。

这一 mapping 只关闭当前 CPU 的浮点与向量执行状态，不清理浮点/向量寄存器，不删除
`CpuCapabilities` 中的硬件支持事实，也不改变中断状态。后续内核态临时打开必须由明确受控的
save/enable/use/disable/restore 区间 lowering，且退出区间时恢复为关闭；用户态是否打开由任务状态和
系统策略 lowering。本入口 action 只建立默认关闭及这些使用策略，不提前实现后续受控区间。

`PhysicalDirect`、`TrampolineVm`、`EarlyVm`、`SwapperVm` 的 `activate_on_cpu(cpu_ref)` 是修改
association 的唯一语义入口。体系结构汇编执行 SATP 切换、同步和 activation commit，Rust 完成验证与
controller 的每 CPU 完成事实；两部分共同 lowering 同一次 `Action::ActivateOnCpu`，不得把 Rust 验证
命名为另一个 action，也不得在验证时重写或补造 trace。

CPU 将唯一的 `active_translation_controller` raw association 与最多四项的顺序 activation journal
放在一个稳定 `repr(C)` 存储中。每项 receipt 固定为 24 bytes：old/new/sync controller raw 分别位于
offset 0/1/2，offset 3 是 raw kind（`0=Empty`、`1=InitialActivation`、`2=Handoff`），offset 4..7
padding，SATP 位于 offset 8，sequence 位于 offset 16；实现必须用 compile-time assertions 固定 size、
alignment 与这些 offset。InitialActivation 的 semantic old controller 为 absent，receipt old raw 为
0；Handoff 的 old/new raw 都必须是四个真实 controller 编码。release 发布的 `committed_count` 是
receipt 可见性的唯一边界，读取者先 acquire count，且不得观察 count 之外的槽位。

提交前先解析 canonical CPU，检查 controller Ready、activation kind、准确旧 association、预期序号及
旧 controller 对应的 live `satp`；目标 SATP 和所需 fence 完成后，先填满 receipt，再发布新
association，最后 release 发布 count。InitialActivation 只允许 absent→PhysicalDirect；Handoff 只
允许 present→present。错误 kind/controller 编码、错误旧 association、错误 SATP、重复/乱序或容量
溢出必须拒绝或 fail-stop，并且不得推进 count、改变 live SATP/association 或留下可见部分记录。

`active_translation_controller()` 返回
`Result<Option<TranslationController>, InvalidTranslationControllerEncoding>`。正在执行内核入口的
CPU 必须恰有一个 controller；stopped AP 必须为 `Ok(None)`，且没有 live SATP。AP boot data 中的
期望 SATP 不写入 live translation state。controller 页表的全局 Ready 不等于任何指定 CPU 已激活它。
