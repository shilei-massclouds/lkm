# SchedInitPhase Coding

## Overview

调度初始化子阶段，对应 model `spec/model/phases/boot/sched-init/phase.spec` 中 `SchedInitPhase` 对象的 Preset → Online 生命周期。

本阶段从 `MmCoreInitPhase.Online` 开始，驱动 Scheduler、RadixTree、MapleTree、Workqueue、Softirq、RcuCore 等调度与同步子系统的建立，对应 Linux `sched_init()` 核心路径。

按[阶段链式映射规则](../../mapping.md#阶段链式映射规则)，每个迁移对应一个概念函数。

## preset()

### 1. depends_on

由 `MmCoreInitPhase.enable()` 保证：
- `MmCoreInitPhase.state == Online`
- `PageAllocator.state == Ready`
- `SlubSubsystem.state == Ready`、`KmallocCaches.state == Ready`
- `CpuGroup.state == Ready`
- `PerCpuStorage.state == Ready`、`CpuHotplugState.state == Ready`
- `StaticBranch.state == Ready`
- `PrintkBuffer.state == Ready`

### 2. drives

模型 `SchedInitPhase.Preset` 按以下顺序驱动：

| # | Model drives | Impl |
|---|---|---|
| 1 | `SchedInitPreludeTrimmedPaths.Transition::Setup` | `ctx.sched_init_prelude_trimmed_paths.setup()` |
| 2 | `Scheduler.Transition::Preset` | `ctx.scheduler.preset()` |
| 3 | `Scheduler.Transition::Setup` | `ctx.scheduler.setup()` |
| 4 | `Scheduler.Transition::Enable` | `ctx.scheduler.enable()` |
| 5 | `RadixTree.Transition::Setup` | `ctx.radix_tree.setup()` |
| 6 | `MapleTree.Transition::Setup` | `ctx.maple_tree.setup()` |
| 7 | `Workqueue.Transition::Preset` | `ctx.workqueue.preset()` |
| 8 | `Softirq.Transition::Preset` | `ctx.softirq.preset()` |
| 9 | `RcuCore.Transition::Setup` | `ctx.rcu_core.setup()` |
| 10 | `SchedInitTraceContextBoundaries.Transition::Setup` | `ctx.sched_init_trace_context_boundaries.setup()` |

### 3. ensures

驱动完成后检查：
- `interrupt_concurrency_closed()`
- `task_concurrency_closed()`
- `context_is(SystemExclusive)`
- IRQs disabled

### 4. emits

推进 Base → Prepared，调用 `setup()`。

## setup()

### 1. depends_on

由 `preset()` 保证。

### 2. drives

无。

### 3. ensures

检查所有被驱动对象已到达模型约定的状态（参见下文 Invariant 表）。

### 4. emits

推进 Prepared → Ready，调用 `enable()`。

## enable()

### 1. depends_on

由 `setup()` 保证。

### 2. drives

无。

### 3. ensures

推进 Ready → Online，发出 `SchedInitPhaseOnline` checkpoint。

### 4. emits

→ `BootPhase.setup_after_children()`

## 迁移间调用关系

```
preset()  ← 由 MmCoreInitPhase.enable() 调用
  │
  ├─ preset_objects()  ← 按模型 drives 顺序驱动全部对象 transition
  │
  ├─ adopt_prepared_with_check()  ← 检查 IRQs disabled，标记 Prepared
  │
  setup()  ← emits
  │
  ├─ adopt_ready()  ← 检查 Online invariant，标记 Ready
  │
  enable()  ← emits
  │
  ├─ enable_event()  ← 标记 Online
  │
  └─ BootPhase.setup_after_children()
```

## Invariant（模型 SchedInitPhase.Ready / Online）

| Object | Required State |
|---|---|
| MmCoreInitPhase | Online |
| Scheduler | Online |
| BootRunQueue | Ready |
| BootIdleTask | Ready |
| RadixTree | Ready |
| MapleTree | Ready |
| Workqueue | Prepared |
| Softirq | Prepared |
| RcuCore | Ready |
| TasksRcu | Prepared |
| SchedInitPreludeTrimmedPaths | Ready |
| SchedInitTraceContextBoundaries | Ready |

## Checkpoints

| Checkpoint | Phase State | Position |
|---|---|---|
| `SchedInitPhaseStarted` | Base | `preset()` 入口 |
| `SchedInitPhaseReady` | Prepared | `adopt_prepared_with_check()` |
| `SchedInitPhaseOnline` | Online | `enable_event()` |

## Coding Constraints

- `Scheduler.setup()` 必须引导 boot CPU runqueue、idle task、bit wait queue table，并完成 `rq_lock`/`raw_spin_rq_lock` + `irqsave` 的 guard 协议。
- `RcuCore.setup()` 必须在 `Softirq.Preset` 之后运行，因为 RCU 需要将 RCU_SOFTIRQ action 注册到软中断表中。
- Workqueue 当前只完成 Preset（建立系统队列槽和 worker pool），setup/enable 暂缓。
- 所有 `sched_init()` 中的 trimmed 路径（poking_init/ftrace_init/early_trace_init/trace_init/context_tracking_init）必须保持 observable deferred/trimmed 标记。
- `SchedInitPreludeTrimmedPaths` 必须记录 `poking_init`/`ftrace_init` 的 trimmed 状态及其原因（如 `CONFIG_FTRACE_MCOUNT_RECORD=n`）。
- `SchedInitTraceContextBoundaries` 必须记录 `trace_init` 的 deferred 状态及 `context_tracking_init` 的 trimmed 状态。
