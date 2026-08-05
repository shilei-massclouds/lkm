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
- 单个 CPU 的 class queue 容量覆盖当前全部可投递 TaskRef；测试必须包含至少九个普通用户 Task
  静态落到同一 CPU 的分布，且最后一次首次 activation 不得在 fork 发布后因队列容量失败。

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
- PID1 首次原地进入用户态开始 CPU0 slice；动态用户 Task 的首次与恢复 dispatch 都必须在提交其
  SATP/本地 fence 后精确增加一次 owner CPU 的 slice-begin 计数。A→B→A 验收分别观察 A、B、A
  三次 dispatch 的新 10 ms deadline，不得借用首次入口遗留的周期 deadline 代替恢复时重置。

## Linux differential 与 gates

差分 case 比较 Schedule entry、PreparePrev exit、PickNext、仅 non-identity 出现的 SwitchTo entry、
next-stack finish/return 等只读 checkpoint 字段与顺序。具体 Linux symbol/source/checkpoint 映射只进入
Testing cross-reference，不进入 Coding。最终实现变更门禁为从仓库根直接运行 `make test`。

## AP trap-overlay 双 Task 闭环

- `smp=2` acceptance 固定 CPU1：CPU0 创建两个带独立栈和完整 context 的普通内核 Task A/B，先发布 A；
  A 运行后 CPU0 release 发布 B 并发送 SSIP。CPU1 必须以完整 TrapFlow/InterruptFlow overlay 返回 A，
  handler 不消费 B mailbox，也不切换 Task。
- A 随后执行带正式 `__ex_table` 条目的真实 load page fault。可调度 page-fault leaf 消费 inbound B 并
  经 owner Scheduler 完成 A→B；B 普通 cooperative yield 完成 B→A。A 的 Dispatch/Enter 精确一次恢复
  原 page-fault leaf，重新验证 root/leaf 后提交 fixup，执行 fault-site 后续坐标并退出。B 后续恢复、
  退出，CPU1 最终回到 idle。
- 验收至少观察两次完整 SSIP overlay、A/B 的真实 switch、一次 leaf resume、一次 exception-table fixup、
  child/root Cleanup、对应 token 数和最终 idle；入口计数证明 A/B 均不重跑。
- 错误目标、stale generation、重复/倒退 ordinal、重复消费和 IPI 先于 release publication 必须在目标
  runqueue 变化前失败或安全合并。唯一 runnable Task 的 identity yield 不得制造伪 switch。
- 同一路径以 `smp=8` 运行，未选中的 AP 保持 idle；显式目标可替换为任一 online AP。native 与
  linux-object provider 都执行同一 overlay acceptance。UP scheduler、
  boot/AP bringup、user/rootfs/LTP gates 保持通过。
- 默认 composite stress 的 `kernel-smoke-native` 样本必须在每次 `smp=8` 重跑中通过上述 AP 闭环 basic
  gate；任何缺失 marker、panic、失败计数或超时都按该次压力样本失败分类，不能只比较聚合成功率。
- 本轮 AP 普通内核 Task 没有 Linux 用户/内核任务的同构执行对象，因此不制造伪造的 AP worker
  checkpoint 对照。默认 Linux/arceos_ex `rc-local` exact-checkpoint difftest 负责证明共享 boot、SMP bringup
  与后续用户路径没有差分回退；AP 专属 mailbox/IPI/context-switch 语义由 tools2 正反场景和 QEMU exact
  acceptance 负责。
