# ProcessPreparePhase coding

model 来源为 `spec/model/phases/interrupt/process-prepare/phase.spec`，实现落点为
`impl/arceos_ex/src/phases/interrupt/process_prepare.rs`。本文件同时承载原 legacy formal rule
的有效实现约束；coding 层不再保留同主题 `.spec`。

## 生命周期映射

| Transition | depends / drives / ensures | checkpoint 与 continuation |
| --- | --- | --- |
| Preset: Base -> Prepared | 在 `SingleTaskInterruptStreamContext` 下精确检查 Base、IrqOpenPrepare Online 及对象依赖；按 model 顺序执行现有 PID/task/cred/VMA/namespace/key/security/VFS/trimmed 对象动作 | 接受后发出 Started；提交后发出 Prepared，调用 Setup |
| Setup: Prepared -> Ready | 精确检查 Prepared 和完整 `process_prepare_phase_ready()` | 提交后发出 Ready，调用 Enable |
| Enable: Ready -> Online | 精确检查 Ready，重新确认未创建 PID 1/kthreadd 且 task/SMP 门关闭 | 提交后发出 Online，返回 `interrupt::preset_after_process_prepare()` |

`is_online()` 只接受精确 Online。ProcessPrepare 不得提交 Interrupt 状态；父 continuation 在该叶子
Online 后提交 Interrupt.Prepared。

## 已迁移实现约束

#### Model path

ProcessPreparePhase is a direct BootInitFlow.Setup phase. Its formal model
path is spec/model/phases/interrupt/process-prepare/.

#### Code path

Phase source layout must follow the model phase tree. The target
implementation path for this phase is the interrupt phase subtree,
for example impl/arceos_ex/src/phases/interrupt/process_prepare.rs.

#### Ordering

ProcessPreparePhase must run after IrqOpenPreparePhase.Online, with
Console.Prepared, SchedClock.Ready, DelayLoop.Ready and the boot CPU
local interrupt gate already established.

#### rest_init boundary

This phase prepares the inputs to rest_init(). It must not create
kernel_init, kthreadd, `KernelInitTask` (PID 1), or any equivalent task, and must not advance the
system into the scheduling-running state.

#### Runtime services

This phase must keep task concurrency and SMP concurrency closed and
must not implicitly start workqueue workers, RCU GP kthreads, full
softirq execution, network namespace runtime or proc visible
services. The only VFS service allowed here is the Linux-like
vfs_caches_init()/mnt_init() slice that creates the initial
ramfs-backed rootfs mount.

#### Object coverage

The implementation must provide explicit object carriers for the
formal PID namespace, anonymous VMA, task creation, credential,
vector context, uprobe, signal, task file context, VMA, namespace,
keyring and security readiness/preparedness facts.

#### Task entry creation contract

TaskCreationCore must expose copy_process()/kernel_clone() as the
shared creation boundary for later rest_init tasks. That boundary
must bind the caller-provided TaskEntry into the new task's startup
context; entry is not an after-the-fact descriptive flag. The source
must be the current OnCpu/Live Task, and its validated TaskRef must equal
the read-only CurrentTask capability. Online, Reserved, non-current,
or mismatched-ref sources are rejected before any mutation; BootTask has
no name-based exception.

#### TaskCreationCore API smoke

TaskCreationCore ObjectApiBehavior smoke must exercise the formal
copy_process() contract directly. It may build a local
TaskCreationCore subject and read live prerequisite objects, but it
must not add a test-only copy helper or register this API case as a
checkpoint KUnit smoke case by default.

#### Deferred paths

Trimmed and deferred Linux start_kernel() calls in this interval
must remain visible as checkpoints or deferred facts rather than
silently disappearing from the implementation boundary.

#### Trimmed/deferred path carrier

ProcessPreparePhase must use a ProcessPrepareTrimmedPaths-style
object to record config-trimmed calls such as x86 EFI runtime
switch, SCS, lockdep_init_task(), cpuset/cgroup/taskstats/
delayacct/ACPI/KCSAN, and enabled-but-deferred calls such as
net_ns_init(), pagecache_init(), seq_file_init(), proc_root_init(),
nsfs_init() and pidfs_init(). rcu_init_tasks_generic() is not in
this subphase and must be recorded as out-of-scope rather than
silently pulled into ProcessPreparePhase.
