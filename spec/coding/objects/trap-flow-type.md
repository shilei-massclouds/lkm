# TrapFlowType coding constraints

Trap/Interrupt/Exception Flow occurrences are stack-local effective-flow leaves above an Online lifetime
TaskFlow. Entering or leaving them does not change Task OnCpu or TaskFlow Online state.

`TaskThreadContext.root_trap_flow_ref` stores the optional root trap occurrence across a real task switch.
After context restore, contextual TaskFlow Enter first resumes the matching trap leaf, then unwinds to
the same lifetime TaskFlow. Signal payloads and yield tokens never copy architectural registers or trap
frames.

Every formal entry, including SSIP, constructs the stack-local record and completes child/root lifecycle;
there is no cause-specific fast return before Preset. `NextDispatch` snapshots the optional root ref and
context epoch. If the root exists, preflight and Enter resolve generation-checked root→active child→concrete
leaf, verify entry TaskFlow and owner CPU, reject a cleaned leaf, then record exactly one per-CPU leaf-resume
observation. The restored `ra/sp` remains the only continuation selector.
