# CPU coding contract

`Cpu` is the only Rust body for every logical CPU. Its canonical identity is the containing
`CpuGroup.cpus[logic_id]` slot; boot/AP names are formatting aliases and never allocate state.

- Store `hartid` and lifecycle/possible/present/active/online facts in `Cpu`; derive logical ID from the slot.
- Keep CPU-local `LocalInterruptControl` and related local state inside `Cpu`; do not store a current-task slot.
- Do not expose `CpuView` or copy CPU facts into an identity-bearing projection.
- Lower `CpuRef` to a compact logical ID. Every dereference goes through `CpuGroup` and rejects an absent slot.
- Lower `CurrentCPU` as a stateless `CurrentCpu` borrow created from an effective `TaskFlow` plus `CpuGroup`.
  Resolve `CurrentTask` first, validate its active Flow, then dereference that Flow's CpuRef and prove the
  Task/Flow/CPU triple agrees. It cannot be constructed from a raw logical ID or an asynchronous emission.

Trace/checkpoint code that receives this capability records the canonical `CpuGroup.cpus[i]` target and the
source Flow/CpuRef. Recording only `CurrentCPU` is insufficient.
