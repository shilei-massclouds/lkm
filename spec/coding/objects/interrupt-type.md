# InterruptType

Each `TrapType` embeds one `InterruptType`. It directly owns the CPU-local total gate (`sstatus.SIE`), class gates and
pending state (`sie`/`sip`), handler bindings, and nested save/disable/restore bookkeeping. PLIC/IrqChip objects still
own device-source gates.

- For the BootCPU `Preset`, lower the model requirement to close every interrupt class gate as `csrw sie, zero`, then
  lower the pending-clear completion fact as one `csrw sip, zero`. The instruction order is mandatory: the `sie` write
  completes before the `sip` write. This is a write-completion boundary, not a guarantee that hardware-driven live
  `sip` bits remain zero afterward.
- Rust adoption of the completed head `Preset` may read and require only live `sie == 0`. It must not read or write
  `sip`, infer any later live `sip == 0` fact, or reset/install handler or fallback policy.
- `Preset` must not read, write, adopt, or infer `sstatus.SIE`; it does not install a handler, fallback, or formal
  dispatch framework. In particular, `sie == 0` is not evidence that the CPU-local total gate is closed.
- `Setup` first closes `sstatus.SIE` through the CPU-local control setup path, then establishes fallback dispatch
  readiness and the Ready boundary; timer/external class handlers remain bound by their later class-routing setup.
- `Enable` opens the total gate only after class routing is ready.
- `SaveAndDisable` records the prior `SIE` value by nesting depth; `Restore` consumes exactly the matching saved value.
- `InterruptFlowType` dispatch is hardirq context: ordinary IRQ reentry and scheduling are rejected unless a named
  return/softirq interval explicitly reopens interrupts.
- Online secondary CPUs enable `sie.SSIE`. The SBI IPI sender runs only after release inbox publication. SSIP
  dispatch clears the local pending bit and release-sets the per-CPU `need_resched` flag; it does not borrow a
  Scheduler, consume an inbox or switch context. Duplicate SSIP occurrences are idempotent at that flag.
- Software interrupt cause has an explicit reschedule handler policy. It runs inside the same fresh
  TrapFlow→InterruptFlow lifecycle as other interrupts and cannot bypass Cleanup/token consumption.
- Each online CPU enables its timer source only after its `SchedulerClockevent` is ready. Timer dispatch asks the
  CPU-local deadline mux to acknowledge and rearm the earliest of the CPU0 compatibility one-shot and 10 ms
  scheduler deadline, then coalesces `need_resched`. It never consumes an inbox, takes a runqueue lock or switches
  context in hardirq.

Mapping: charter [`interrupt-type.md`](../../charter/objects/interrupt-type.md), model
[`interrupt_type.spec`](../../model/objects/interrupt_type.spec), implementation
[`interrupt_type.rs`](../../../impl/arceos_ex/src/objects/interrupt_type.rs).
