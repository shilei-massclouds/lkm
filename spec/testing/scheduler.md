# Scheduler testing contract

本文件从 Charter、Model 与 Coding 导出 per-CPU Scheduler 的验证责任，不重新定义语义。

## Inventory 与选择

- 每个 possible CPU 恰有一个 owned Scheduler；`curr/idle/stop` 与各 class queue 只保存稳定
  TaskRef。共享 root/sched domain 保持独立对象。
- `sched_init` 后所有 possible CPU Scheduler 为 Ready；CPU0 在 boot 调度边界进入 Online，AP
  Scheduler 在各自 CPU-online handoff 才进入 Online。
- PreparePrev 覆盖 runnable、blocked 与 matching pending wake；blocked Task 从 runqueue/class
  membership 移除但保持 `Online/None/Valid`。选择顺序固定为 stop→DL→RT→fair→idle，且
  PreparePrev、Pick、PutPrev、SetNext 的次序可观察。

## `yields Schedule` 与显式 switch

- Schedule before-send snapshot 观察当前固定 TaskFlow Online、Task OnCpu、Scheduler Online，且
  尚无 Schedule occurrence、YieldToken、PreparePrev、Pick 或 context switch。投递本身不产生任何
  Task、TaskFlow、CPU、context、CurrentTask、CurrentStack 或 runqueue delta。
- rejection 在 YieldToken 提交前完成，必须保持 source lane、Scheduler 与全部对象的精确 before
  snapshot。目标 handler post-commit failure、stale、错误 CPU/Flow/context epoch 或重复恢复均为
  terminal failure，不回滚、不重试。
- identity 允许 class 记账，但不得 Save/Restore context、改变 Task lifecycle/authority/breakpoint 或
  CPU-local binding，也不得投递 Dispatch/Enter；目标正常完成后的通用 resume attempt 精确消费
  source token。
- non-identity 的显式顺序为 PreparePrev/Pick → SaveCoreContext → prev
  `OnCpu --Suspend--> Online` → RestoreCoreContext 与 CurrentTask/CurrentStack commit → next
  `Online --Dispatch--> OnCpu` → embedded TaskFlow contextual Enter。Enter 不区分首次/恢复，只精确消费
  当前 handler、YieldToken 或 machine coordinate。
- A→B→A 与嵌套 Schedule occurrence 必须证明 B 的 switch 不会错误消费 A 的 token；恢复 A 时用
  dispatch record、CPU、TaskRef、FlowRef、generation 与 context epoch 精确一次恢复 source lane。

## Linux differential 与 gates

差分 case 比较 Schedule entry、PreparePrev exit、PickNext、仅 non-identity 出现的 SwitchTo entry、
next-stack finish/return 等只读 checkpoint 字段与顺序。具体 Linux symbol/source/checkpoint 映射只进入
Testing cross-reference，不进入 Coding。最终实现变更门禁为从仓库根直接运行 `make test`。
