# Scheduler testing contract

本文件规定 per-CPU `Scheduler` 的测试与 Linux 差分验证责任。它只从 Charter、Model 与 Coding
导出观察项，不重新定义调度语义。

## Inventory 与 lifecycle

- 对每个 possible `CpuGroup.cpus[i]`，测试必须观察恰好一个 owned Scheduler，且不存在全局
  Scheduler singleton、独立 RunQueue object body 或 `[CpuRunQueueMetadata; MAX_CPUS]` 镜像。
- `sched_init` 后所有 possible CPU Scheduler 为 Ready；CPU0 Scheduler 随 boot CPU scheduling
  handoff 进入 Online，AP Scheduler 只在对应 CPU online handoff 时进入 Online。
- Scheduler 的 `curr/idle/stop` 与 stop/DL/RT/fair/idle 队列只保存稳定 `TaskRef`；共享 root/sched
  domain 必须保持 Scheduler 之外的独立对象。

## PreparePrev 与 class handoff

- 正例覆盖 running/runnable prev 保持 Runnable，以及 sleeping prev 在 matching pending wake signal
  存在时一次性恢复 running。
- 反例覆盖非抢占 sleeping prev 且无 pending wake signal：`DeactivateTask` 必须在 pick 之前移除 class
  membership 并令 on-rq=false；后续 `PutPrevTask` 不得把 blocked prev 放回。
- stop→DL→RT→fair→idle 的选择顺序必须逐级验证。组合 callback 与
  `pick_task -> PutPrevTask -> SetNextTask` fallback 对相同输入必须选择相同 next；put/set 不得出现在
  PreparePrev 或 pick 之前。
- identity 允许 callback 定义的 class 记账，但不得产生真实 SwitchTo、Task lifecycle/context 或
  CurrentTask/CurrentStack 变化。

## Signal 与 lifecycle 因果

- Schedule before-send snapshot 必须观察 `BootInitFlow.Online`、`BootTask.OnCpu`，且尚无 Schedule
  Signal、PreparePrev、PickNextTask 或 SwitchTo occurrence。发出 Schedule 及 PreparePrev 均不得改变
  BootTask lifecycle。
- 非 identity 顺序必须观察：current active TaskFlow emits `Scheduler.Schedule`；Scheduler drives
  PreparePrev/PickNextTask；随后 SaveCoreContext → Suspend prev → RestoreCoreContext/CurrentTask binding
  → next-stack finish；Scheduler emits `next Task.Continue`；next Task 接受后才 emits initial Flow.Startup
  或 active Flow.Continue。Scheduler 不直接 emits next TaskFlow Startup/Continue。
- identity 必须保持 current Task OnCpu，并只由 Scheduler 向原 sender TaskFlow emits Continue；不得发送
  Task.Continue 或伪造一次 Suspend/Restore。
- blocked prev 切出后可保持 `Online/None/Valid`，但在 wake/enqueue 恢复 class membership 前不可被选中。
- cross-CPU、stale/non-active Flow 与错误 CurrentTask binding 必须在首个状态修改前失败，并保持 Scheduler、
  Task、Flow、context 与后续信号容量的精确 before snapshot。

## Linux differential

差分 case 至少比较以下只读 checkpoint 字段与顺序：Schedule entry 的 CPU/mode/prev/state/on-rq；
PreparePrev exit 的 signal-recovery/block 结果和 on-rq；PickNext exit 的 prev/next/class；仅非 identity
出现的 SwitchTo entry；next-stack finish/return。具体 Linux symbol、源码 anchor 与 marker 只进入
[`checkpoint-cross-reference.md`](checkpoint-cross-reference.md) 所属 inventory/mapping，不进入 Coding。

Boot 首次 `schedule_preempt_disabled()` 的差分必须证明 BootTask 在 pick 前仍为 running/runnable，且只有
`next != prev` 的 SwitchTo 才令其 OnCpu→Online。最终实现变更门禁为从仓库根直接运行 `make test`。
