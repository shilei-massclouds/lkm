# BootInitFlow.Enable and Online coding constraints

Model 来源为 [`main.spec`](../../../model/flows/boot_init_flow/main.spec) 与
[`enable.spec`](../../../model/flows/boot_init_flow/enable.spec)。`BootInitRestInitPhase`、
`BootInitScheduleHandoffPhase` 和 `BootIdleEntryPhase` 继续由各自独立 Rust 文件实现；它们位于同一
owner 目录，但不是 BootInitFlow transition 的重复定义。

`rest_init` creates KernelInitTask/KernelInitFlow and KthreaddTask/KthreaddFlow as lifetime pairs. For each
pair, Task context setup precedes Flow publication; Flow becomes Online before Task publication and
enqueue visibility is committed before Task Enable.

BootInitScheduleHandoff performs only reversible scheduler preflight while BootInitFlow is being prepared.
After BootInitFlow becomes Online, its RequestSchedule action invokes `schedule_current()` as a yielding
call. On eventual return, the same BootInitFlow continuation prepares and runs BootIdleEntryPhase.

KernelInitFlow and KthreaddFlow are already Online before their first dispatch. Scheduler restores the
prepared TaskThreadContext, commits Task Continue, and calls the same contextual Flow Continue used for
all later dispatches.

BootInitFlow 本身的 Enable、首次 `schedule_current()`、恢复 continuation 和 Online 状态检查落在
`impl/arceos_ex/src/flows/boot_init_flow/enable.rs`。`idle.rs` 继续只承载私有 Online runtime state。
