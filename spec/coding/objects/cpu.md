# CPU coding contract

`Cpu` is the only Rust body for every logical CPU. Its canonical identity is the containing
`CpuGroup.cpus[logic_id]` slot; boot/AP names are formatting aliases and never allocate state.

- Store `hartid`, lifecycle/possible/present/active/online facts and optional `active_translation_controller` in
  `Cpu`; derive logical ID from the slot. The association is an atomic CPU-local controller ref, not a page-table
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
