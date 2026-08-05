# InterruptFlowType

`InterruptFlowType` is declared in its root record and binds to the entry CPU's `InterruptType`, never to its root Flow.
It saves/restores class gates and hardirq depth across the full Preset→Setup→Enable→Disable→Cleanup lifecycle. Setup
dispatches the bound handler; scheduling or unapproved ordinary IRQ reentry is terminal.

The software/reschedule handler is one bound policy in Setup. It clears SSIP and coalesces `need_resched`; inbox
consumption and Scheduler access remain outside the hardirq occurrence.

The timer handler follows the same rule after acknowledging and rearming the CPU-local clockevent. The common
return continuation may inspect work only after the concrete handler and leaf `Disable` complete and before leaf
or root `Cleanup`. An SPP=U origin acquire-drains all visible CPU-local inbox records, consumes the coalesced
reschedule request, and may save its still-resolvable trap overlay and schedule there; an SPP=S origin leaves the
request pending until the next return-to-user boundary. The A→B→A path must revalidate the original root, leaf,
TaskRef, FlowRef, generation, CPU and context epoch before resuming it and completing Cleanup.

Mapping: charter [`interrupt-flow-type.md`](../../charter/objects/interrupt-flow-type.md), model
[`interrupt_flow_type.spec`](../../model/objects/interrupt_flow_type.spec), implementation
[`interrupt_flow_type.rs`](../../../impl/arceos_ex/src/objects/interrupt_flow_type.rs).
