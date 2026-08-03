# InterruptFlowType

`InterruptFlowType` is declared in its root record and binds to the entry CPU's `InterruptType`, never to its root Flow.
It saves/restores class gates and hardirq depth across the full Preset→Setup→Enable→Disable→Cleanup lifecycle. Setup
dispatches the bound handler; scheduling or unapproved ordinary IRQ reentry is terminal.

The software/reschedule handler is one bound policy in Setup. It clears SSIP and coalesces `need_resched`; mailbox
consumption and Scheduler access remain outside the hardirq occurrence.

Mapping: charter [`interrupt-flow-type.md`](../../charter/objects/interrupt-flow-type.md), model
[`interrupt_flow_type.spec`](../../model/objects/interrupt_flow_type.spec), implementation
[`interrupt_flow_type.rs`](../../../impl/arceos_ex/src/objects/interrupt_flow_type.rs).
