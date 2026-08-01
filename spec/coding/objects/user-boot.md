# User boot coding constraints

KernelInitTask retains KernelInitFlow for its entire lifetime. KernelInitFlow owns one stable
UserAppRuntime. PID 1 exec replaces only `runtime.application`; successful replacement preserves the
identities and generations of Task, TaskRef, Flow, FlowRef and Runtime.

Fork allocates a fresh Task, TaskRef, UserTaskFlow, FlowRef, UserAppRuntime and ApplicationInstance. The
creation order is structural bind, Task context setup, Runtime publication, Flow publication, runqueue
publication, then Task publication. No child shares Runtime or Flow storage with its parent or siblings.

Syscall and user-mode traps are effective-flow children above the lifetime TaskFlow. They may schedule;
on return, the scheduler restores the saved trap leaf from TaskThreadContext before resuming the
TaskFlow-level continuation.

Exit quiesces the ApplicationInstance and Runtime, disables and cleans the fixed Flow, then disables and
cleans the Task. A pending yield must be resolved or terminated before Flow cleanup.
