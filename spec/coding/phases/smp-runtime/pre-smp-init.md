# PreSmpInitPhase coding

本文件承载 `spec/coding/phases/smp-runtime/pre-smp-init.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/phases/smp-runtime/pre-smp-init.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`pre-smp-init.spec`](pre-smp-init.spec)。

### ArceosExPreSmpInitCodingMust

#### Model path

PreSmpInitPhase is SmpRuntimePhase subphase 1. Its formal model
path is spec/model/phases/smp-runtime/pre-smp-init/.

#### Code path

Implementation must live under impl/arceos_ex/src/phases/smp_runtime/.

#### Entry facts

This phase must run after UpMultitaskPhase.Online from the
KernelInitTask entry on its verified task stack. It must additionally
consume the KernelInitTask release/dispatch facts and Scheduler
first-schedule fact, not infer readiness from BootIdleEntryPhase.Ready
or a RestInitPhase wrapper.

#### kthreadd_done wait side

PreSmpInitPhase begins on the KernelInitTask execution line by
observing kthreadd_done through the Completion wait side. The release
fact must be produced by KernelInitTask observing KthreaddReadyGate,
not by BootInitTask's complete side.

#### KernelInitTask entry

This phase must also consume the TaskCreationCore entry contract:
KernelInitTask was created with TaskEntry::KernelInit and that entry
points at the SmpRuntimePhase execution line whose first child is
PreSmpInitPhase.

#### Allocation and CPU topology

This phase must open PageAllocator full GFP mask and record pre-SMP
CPU topology/present facts without making secondary CPUs online.

#### Runtime support setup

This phase must setup Workqueue, VmstatCore, TasksRcu and early
pre-SMP initcall boundary facts.

#### workqueue_init() synchronization

Workqueue.setup() corresponds to Linux workqueue_init(), which takes
wq_pool_mutex while fixing pool node hints and creating rescuers.
The implementation must expose this KernelInitTask-side mutex guard
and the model must use within WorkqueuePoolMutexContext { ... }.

#### Stop before SMP

smp_init() is the next top-level phase boundary and must not be
executed or modeled as complete here.

<!-- formal-predicate-notes:spec/coding/phases/smp-runtime/pre-smp-init.spec END -->
