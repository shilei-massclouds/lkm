# CPU coding contract

`Cpu` is the only Rust body for every logical CPU. Its canonical identity is the containing
`CpuGroup.cpus[logic_id]` slot; boot/AP names are formatting aliases and never allocate state.

- Store `hartid`, lifecycle/possible/present/active/online facts and optional `active_translation_owner` in
  `Cpu`; derive logical ID from the slot. The owner is an atomic CPU-local enum/ref, not a page-table state copy.
- Keep CPU-local `InterruptType` and related local state inside `Cpu`; do not store a current-task slot.
- Do not expose `CpuView` or copy CPU facts into an identity-bearing projection.
- Lower `CpuRef` to a compact logical ID. Every dereference goes through `CpuGroup` and rejects an absent slot.
- Lower `CurrentCPU` as a stateless `CurrentCpu` borrow created from an effective `TaskFlow` plus `CpuGroup`.
  Resolve `CurrentTask` first, validate its active Flow, then dereference that Flow's CpuRef and prove the
  Task/Flow/CPU triple agrees. It cannot be constructed from a raw logical ID or an asynchronous emission.

Trace/checkpoint code that receives this capability records the canonical `CpuGroup.cpus[i]` target and the
source Flow/CpuRef. Recording only `CurrentCPU` is insufficient.

`PhysicalDirect`、`TrampolineVm`、`EarlyVm`、`SwapperVm` 的 `take_over(cpu_ref)` 是修改 owner 的唯一
入口。每次调用先解析 canonical CPU，检查 controller Ready、旧 owner 与该 CPU 的 live `satp` 一致，
再按 controller 规则写 `satp`/执行 fence，并用原子 compare-exchange 替换 owner；trace 必须记录 CPU、
旧/新 owner、目标 SATP 与同步结果。正在执行内核入口的 CPU 必须有且仅有一个 owner；仅已发现但尚未
进入内核的 AP 可以为 `None`。controller 页表的全局 Ready 不等于任何指定 CPU 已经激活它。
