# PageFaultExceptionFlowType

This occurrence binds to `PageFaultExceptionType` and separately records fault address/access/source, nesting,
hardirq and entry interrupt state. A validated kernel exception-table fixup is stored before any scheduling point.
The atomic path writes fixup `sepc` directly. The schedulable kernel task-context path observes published mailbox or
`need_resched`, invokes the owner Scheduler normally, and after restoration revalidates the exact root/exception/leaf,
generation, CPU and context epoch before writing `sepc`. Cleanup remains leaf→exception→root.

For a user occurrence, the flow snapshot contains Task/mm identity, VA, original `sepc`, access, VMA class and
resolver result. `RetrySameInstruction` is accepted only when the returned `sepc` equals the captured value.
The COW branch additionally snapshots the leaf PFN/COW bit and checked `UserFrameRef` count. A shared-copy or
unique-reference commit must revalidate that tuple before changing the leaf, flush the one VA and retry the same
instruction. Resource failure preserves the tuple and selects the Task `OutOfMemory` terminal result.
`Unmapped/SegvMaperr` and `Protection/SegvAccerr` snapshot the exact request VA as `si_addr`, record the current
Task's signal-11 terminal and enter the existing exit/wait/SIGCHLD handoff without changing the captured `sepc`.
They never enter the COW retry or kernel fixup branches, and `InvalidContext` is not lowered to a user signal.
User and kernel result variants are disjoint, so neither path can accidentally consume the other's recovery
state.

Mapping: charter [`page-fault-exception-flow-type.md`](../../charter/objects/page-fault-exception-flow-type.md), model
[`page_fault_exception_flow_type.spec`](../../model/objects/page_fault_exception_flow_type.spec), implementation
[`page_fault_exception_flow_type.rs`](../../../impl/arceos_ex/src/objects/page_fault_exception_flow_type.rs).
