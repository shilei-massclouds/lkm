# rest_init coding constraints

`rest_init` creates KernelInitTask/KernelInitFlow and KthreaddTask/KthreaddFlow as lifetime pairs. For each
pair, Task context setup precedes Flow publication; Flow becomes Online before Task publication and
enqueue visibility is committed before Task Enable.

BootInitScheduleHandoff performs only reversible scheduler preflight while BootInitFlow is being prepared.
After BootInitFlow becomes Online, its RequestSchedule action invokes `schedule_current()` as a yielding
call. On eventual return, the same BootInitFlow continuation prepares and runs BootIdleEntryPhase.

KernelInitFlow and KthreaddFlow are already Online before their first dispatch. Scheduler restores the
prepared TaskThreadContext, commits Task Continue, and calls the same contextual Flow Continue used for
all later dispatches.
