# BootInitFlow coding constraints

BootInitFlow is the BootTask's immutable lifetime Flow. It is statically bound before `_start`, becomes
Online once boot preparation and schedule preflight complete, and stays Online while BootTask alternates
between OnCpu and Online.

Its Online actions own idle setup, the `schedule_current()` yield boundary, post-schedule idle entry and
the idle loop. The call must return through the same Rust continuation when identity is selected and
through the restored TaskThreadContext plus contextual Continue after a non-identity A->B->A sequence.

No separate idle Flow storage, FlowRef, checkpoint family or dispatch branch may exist.
