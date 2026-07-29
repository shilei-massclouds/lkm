# InterruptType

Each `TrapType` embeds one `InterruptType`. It directly owns the CPU-local total gate (`sstatus.SIE`), class gates and
pending state (`sie`/`sip`), handler bindings, and nested save/disable/restore bookkeeping. PLIC/IrqChip objects still
own device-source gates.

- For the BootCPU `Preset`, lower the model requirement to close every interrupt class gate as `csrw sie, zero`, then
  lower the model requirement to clear every pending interrupt signal as `csrw sip, zero`. The instruction order is
  mandatory: the `sie` write completes before the `sip` write.
- `Preset` must not read, write, adopt, or infer `sstatus.SIE`; it does not install a handler, fallback, or formal
  dispatch framework. In particular, `sie == 0` is not evidence that the CPU-local total gate is closed.
- `Setup` binds timer/external handlers while leaving the total gate closed.
- `Enable` opens the total gate only after class routing is ready.
- `SaveAndDisable` records the prior `SIE` value by nesting depth; `Restore` consumes exactly the matching saved value.
- `InterruptFlowType` dispatch is hardirq context: ordinary IRQ reentry and scheduling are rejected unless a named
  return/softirq interval explicitly reopens interrupts.

Mapping: charter [`interrupt-type.md`](../../charter/objects/interrupt-type.md), model
[`interrupt_type.spec`](../../model/objects/interrupt_type.spec), implementation
[`interrupt_type.rs`](../../../impl/arceos_ex/src/objects/interrupt_type.rs).
