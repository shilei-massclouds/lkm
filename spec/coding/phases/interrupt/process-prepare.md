# ProcessPreparePhase coding

本文件承载 `spec/coding/phases/interrupt/process-prepare.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/phases/interrupt/process-prepare.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`process-prepare.spec`](process-prepare.spec)。

### ArceosExProcessPrepareCodingMust

#### Model path

ProcessPreparePhase is InterruptPhase subphase 4. Its formal model
path is spec/model/phases/interrupt/process-prepare/.

#### Code path

Phase source layout must follow the model phase tree. The target
implementation path for this phase is the interrupt phase subtree,
for example impl/arceos_ex/src/phases/interrupt/process_prepare.rs.

#### Ordering

ProcessPreparePhase must run after IrqOpenPreparePhase.Ready, with
Console.Prepared, SchedClock.Ready, DelayLoop.Ready and the boot CPU
local interrupt gate already established.

#### rest_init boundary

This phase prepares the inputs to rest_init(). It must not create
kernel_init, kthreadd or any PID 1 task, and must not advance the
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
context; entry is not an after-the-fact descriptive flag.

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

<!-- formal-predicate-notes:spec/coding/phases/interrupt/process-prepare.spec END -->
