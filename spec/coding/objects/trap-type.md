# TrapType

`TrapType` is embedded in each `Cpu`; `Context` must not own a second trap resource. It is the stable carrier for
fresh `TrapFlowType` response flows, in the same way that `Task` is the stable carrier for `TaskFlow`. Its RISC-V
lowering owns the public `stvec` service, the per-CPU `TrapEntryContext`, stack-capacity admission and the emergency
failure path.

- `Preset` writes `stvec` with the address of an assembly-defined temporary protection entry. The entry is an empty
  infinite loop: after the required alignment it contains only an unconditional branch to itself, with no `wfi`,
  Rust/C call, checkpoint, log, shutdown request or other side effect. The lowering is equivalent to:

  ```asm
  .align 2
  trap_temporary_protection_entry:
      j trap_temporary_protection_entry

      la t0, trap_temporary_protection_entry
      csrw stvec, t0
  ```

  The symbol loaded into `stvec` is the assembly entry itself, not a Rust wrapper or a data object.
- `Setup` starts with the CPU's `InterruptType` Ready and `ExceptionType` Base. It drives
  `ExceptionType.Preset`, which drives all four concrete exception Presets, before publishing the formal response
  entry. It then writes the address of an assembly-defined formal response function into `stvec`; that function is
  the architecture entry which declares and binds a fresh `TrapFlowType` for each accepted occurrence. The lowering
  is equivalent to:

  ```asm
  .align 2
  formal_trap_flow_entry:
      /* save the architectural entry and enter the TrapFlowType path */
      ...

      la t0, formal_trap_flow_entry
      csrw stvec, t0
      csrw sscratch, zero
  ```

  The value written to `stvec` must be the aligned assembly function entry address in the active address space, not
  a page-table value, Rust function pointer wrapper or data object. All failure-capable validation, exception
  fallback preparation and entry-context installation must finish before this two-write commit sequence. The
  `stvec` write publishes the formal response entry and the immediately following `sscratch=0` write records that
  execution is presently in the kernel. Until the `stvec` write commits, the Preset protection entry remains the
  active response entry.
- `Enable` accepts entries only after both children are online.
- While the CPU executes in the kernel, `sscratch` must be zero. Before `sret` to user mode it temporarily carries the
  kernel Task identity from `tp`. The formal entry starts with the Linux-style `csrrw tp, sscratch, tp`: zero selects a
  kernel-origin entry and the saved Task identity selects a user-origin entry; after recovering kernel `tp`, the entry
  clears `sscratch` again before any nested trap can occur.
- `sscratch` is not the `TrapEntryContext` locator. Each CPU's formal assembly entry representation must resolve its
  owning CPU's installed `TrapEntryContext` independently. BP entry, every AP entry and scheduler commit install or
  refresh that CPU-local context before an entry can occur.
- `TrapEntryContext` contains CPU identity, the active task binding, current kernel-stack bounds, emergency-stack
  bounds/state and an optional generation-checked root `TrapFlowRef`. It is entry lowering, not selector authority.
- The assembly record is `TrapFrame` followed by a naturally aligned `TrapExecutionRecord`. Admission compares the
  prospective complete record against the installed stack bounds. Insufficient capacity switches to the owning
  CPU's emergency stack and fail-stops; emergency reentry fail-stops without another switch.
- `sret` is reachable only after Rust returns a consumed, one-shot `TrapReturnToken` produced after Cleanup.
- Formal Rust entry resolves a `TrapRuntimeLease` from the entry context, `tp`, owner Scheduler `curr`, fixed
  TaskFlow CpuRef and the target-owned task registry. CPU0 may use its boot-owner lease; an AP may use only its
  published secondary lease and target-CPU task access, never a global `&'static mut Context`. Exception-table
  lookup uses one read-only runtime reference.
- A bounded per-CPU atomic observation records root/interrupt/exception/SSIP completions, token consumption,
  leaf-switch resume and last generation. It is diagnostic state, not a checkpoint provider.
- Before formal Rust entry initializes the stack-local `TrapExecutionRecord`, it snapshots the raw root
  `TrapFlowRef` published in `TrapEntryContext`. A terminal trap-occurrence lifecycle diagnostic emits that
  pre-initialization identity together with the failing Task root, the new record root, record bounds, CPU/Task/
  TaskFlow identities and architectural cause in one bounded record. The snapshot is identity evidence only: the
  diagnostic must not dereference a potentially stale root or change trap lifecycle behavior.

Mapping: charter [`trap-type.md`](../../charter/objects/trap-type.md), model
[`trap_type.spec`](../../model/objects/trap_type.spec), implementation
[`trap_type.rs`](../../../impl/arceos_ex/src/objects/trap_type.rs).
