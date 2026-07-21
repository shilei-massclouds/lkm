# PreSmpInitPhase coding

PreSmpInitPhase 是 SmpRuntimePhase 的第 1 个直接子阶段，由 KernelInitTask 在自己的
vmalloc task stack 上执行。model 路径为 `spec/model/phases/smp-runtime/pre-smp-init/`，
实现落点为 `impl/arceos_ex/src/phases/smp_runtime/pre_smp_init.rs`。

## 生命周期映射

`preset()` 精确检查 Base、BootInitFlow Online、KernelInitTask entry/release/dispatch 与栈事实，
发出 Started，然后按 model 顺序驱动本阶段全部对象动作并提交 Prepared。`setup()` 只检查对象
完成事实并提交 Ready；`enable()` 再检查相同 invariant、提交 Online，并且只返回
`smp_runtime::preset_after_pre_smp_init()`。

| Checkpoint | owner state | 位置 |
| --- | --- | --- |
| `PreSmpInitPhase.Started` | Base | Preset 依赖被接受后、首个对象动作前 |
| `PreSmpInitPhase.Prepared` | Prepared | 全部对象动作完成后 |
| `PreSmpInitPhase.Ready` | Ready | Setup 检查完成后 |
| `PreSmpInitPhase.Online` | Online | Enable 检查完成后、父 continuation 前 |

#### Entry facts

This phase must run after BootInitFlow.Online from the
KernelInitTask entry on its verified task stack. It must additionally
consume the KernelInitTask release/dispatch facts and Scheduler
first-schedule fact, not infer readiness from BootIdleEntryPhase.Ready
or a RestInitPhase wrapper.

#### kthreadd_done wait side

PreSmpInitPhase begins on the KernelInitTask execution line by
observing kthreadd_done through the Completion wait side. The release
fact must be produced by KernelInitTask observing KthreaddReadyGate,
not by BootTask's complete side.

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
