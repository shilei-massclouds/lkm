# RuntimeCorePhase coding

RuntimeCorePhase 是 KernelInitFlow.Setup 的第 1 个直接子阶段，由 KernelInitTask 执行。model 路径为
`spec/model/phases/smp-runtime/runtime-core/`，实现落点为
`impl/arceos_ex/src/phases/smp_runtime/runtime_core.rs`。

## 生命周期映射

`preset()` 依赖 SmpBringupPhase 精确 Online。依赖接受后立即发出
`RuntimeCorePhase.Started`，因此该事件位于 `Scheduler.enable_smp()` / Linux
`sched_init_smp()` 动作之前；随后驱动全部对象并提交 Prepared。Setup/Enable 只检查
`runtime_core_ready()` 和 RuntimeCoreBoundary，分别提交 Ready/Online；Online 只返回
`KernelInitFlow.setup_after_runtime_core()`。

四个 checkpoint 依次是 Started、Prepared、Ready、Online；Started/Ready 保持既有 ID，
Prepared/Online 追加且默认 unmapped。

#### Entry gate

RuntimeCorePhase must run after SmpBringupPhase.Online, with
secondary CPUs online and SMP concurrency open.

#### Scheduler SMP action

Scheduler.enable_smp() must publish SMP scheduler domains, release
PID 1 boot CPU affinity, clear PF_NO_SETAFFINITY, refresh
granularity and initialize RT/DL SMP post state under the
sched_domains_mutex guard without re-running Scheduler lifecycle
enable.

#### Workqueue topology

RuntimeCorePhase must publish workqueue topology facts for CPU/SMT,
cache and NUMA pod types and rebind unbound pools while keeping the
current object-level Workqueue.Ready historical state stable. The
topology action must reuse the existing wq_pool_mutex and aggregate
workqueue_struct mutex guards.

#### Deferred runtime cores

async_init() and padata_init() must remain explicit deferred
boundaries in this step. The deferred facts must preserve async
workqueue/min_active and padata hotplug/free-list responsibilities.

#### Page allocator late

RuntimeCorePhase must publish page_alloc_init_late() facts,
including memory stats, buffer init, memblock private discard, zone
contiguous, sysctl and current-config trimmed late paths. Deferred
struct page completion/static key, page extension and shuffle late
paths must be recorded with config-trimmed reasons.
