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
语义入口。体系结构汇编执行 SATP 切换、同步和提交，Rust 完成验证与 controller 的每 CPU 完成事实；
两部分共同实现同一次 `Action::TakeOver`，不得把 Rust 验证命名为 `adopt`，也不得在验证时重写或补造
trace。

CPU 将唯一的 `active_translation_owner` 与最多四项的顺序 takeover journal 放在一个稳定 `repr(C)`
存储中。每项记录旧/新 owner、目标 SATP、同步完成事实和从 1 开始的提交序号；release 发布的
`committed_count` 是记录可见性的唯一边界，读取者不得观察 count 之外的槽位。提交前先解析 canonical
CPU，检查 controller Ready、允许的旧 owner、预期序号及旧 owner 对应的 live `satp`；目标 SATP 和
所需 fence 完成后，先填满 journal 槽位并发布 owner，最后 release 发布 count。错误旧 owner、错误
SATP、重复/乱序或容量溢出必须拒绝或 fail-stop，并且不得推进 count 或留下可见的部分记录。

正在执行内核入口的 CPU 必须有且仅有一个 owner；仅已发现但尚未进入内核的 AP 可以为 `None`。
controller 页表的全局 Ready 不等于任何指定 CPU 已经激活它。
