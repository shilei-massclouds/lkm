# CPU / CpuGroup testing contract

This file is the testing authority for CPU ownership, references and the
`CurrentCPU` context selector.

## Identity and ownership

- Model and object-view tests must expose exactly one `CpuGroup` and one indexed
  owned collection `CpuGroup.cpus[logical_id]: CPU`. `BootCPU` and AP names are
  aliases of indexed elements and must not appear as independent object, state,
  lifecycle or snapshot keys.
- Static and runtime audits must reject parallel authoritative CPU stores or
  projections such as `CpuView`, `cpu_refs` and `SecondaryCpuStore`. Rebuildable
  possible/present/active/online bitmaps are allowed only when every member
  resolves back to the corresponding owned element.
- Every published CpuRef must dereference to an existing indexed element; a
  missing element, invalid logical ID or stale/unpublished target must fail at
  the dereference boundary. Tests must cover unique logical IDs, unique hartids
  and both logical-ID-to-hartid and hartid-to-logical-ID lookup.

## TaskFlow and CurrentCPU

- TaskFlow is the only Task-side owner of a `cpu_ref` field. Tests must reject a
  synonymous Task field and writes outside entry acceptance or scheduler
  migration commit.
- Leaving OnCpu retains the assigned/last CpuRef. Migration updates it at commit,
  and a Flow handoff copies it to the successor before active-flow publication
  and successor activation.
- `CurrentCPU` is tested as a selector, never an object. Every resolved Signal
  trace must record selector `CurrentCPU`, source Flow, source CpuRef and actual
  canonical target `CpuGroup.cpus[i]`. A synchronous `drives` subtree inherits
  the effective Flow context; asynchronous `emits` does not.

## Runtime observations and gates

- Kernel-environment smoke observes the production CpuGroup in Context and
  proves CPU0/AP roles, set membership, topology uniqueness, valid CpuRefs and
  the absence of a second boot/current CPU or current-task store. `current_cpu()`
  must derive the effective Flow through CurrentTask, dereference that Flow's
  CpuRef and reject any Task/Flow/CPU disagreement.
- The same smoke boundary must observe each CPU's final translation owner and
  complete ordered takeover journal. BP records
  `None -> PhysicalDirect -> TrampolineVm -> EarlyVm -> SwapperVm`; every AP
  records `None -> PhysicalDirect -> TrampolineVm -> SwapperVm` and no EarlyVm
  receipt. Every record carries the canonical CPU, old/new owner, installed
  SATP, completed synchronization and contiguous one-based sequence, while the shared
  `PhysicalDirect`, `TrampolineVm`, `EarlyVm` and `SwapperVm` controllers remain
  `Ready` rather than being destroyed by any CPU-local handoff.
- Negative CPU translation-state tests cover wrong old owner, target/live SATP
  mismatch, duplicate and out-of-order receipts, release count hiding a
  half-written slot, and exact/overflowing trampoline-window endpoints. Every
  rejected commit preserves owner and committed journal count.
- A disassembly test included by the root regression checks both BP and AP
  entry symbols and proves that target SATP installation and required
  `sfence.vma` precede journal fill, owner publication and the release-published
  committed count. Final runtime state is not sufficient evidence for this
  transient ordering contract.
- Focused tool tests cover indexed publication/rollback and selector isolation;
  scheduler and user-flow smoke cover migration and handoff. The final gate
  after implementation changes is the direct repository-root `make test`.
