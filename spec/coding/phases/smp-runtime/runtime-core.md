# RuntimeCorePhase coding

本文件承载 `spec/coding/phases/smp-runtime/runtime-core.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/phases/smp-runtime/runtime-core.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`runtime-core.spec`](runtime-core.spec)。

### ArceosExRuntimeCoreCodingMust

#### Model path

RuntimeCorePhase is SMP Runtime Phase subphase 3. Its formal model
path is spec/model/phases/smp-runtime/runtime-core/.

#### Code path

Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.

#### Entry gate

RuntimeCorePhase must run after SmpBringupPhase.Ready, with
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

<!-- formal-predicate-notes:spec/coding/phases/smp-runtime/runtime-core.spec END -->
