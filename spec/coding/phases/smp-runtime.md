# SmpRuntimePhase 编码指引

SmpRuntimePhase 是 [Kernel 系统编码](../systems/kernel.md)中 `Kernel.Enable` 的第二个
drive 目标。model 来源为
[`spec/model/phases/smp-runtime/phase.spec`](../../model/phases/smp-runtime/phase.spec)，
实现落点为 `impl/arceos_ex/src/phases/smp_runtime/mod.rs`。父阶段和六个直接子阶段的 BP
主线都由 KernelInitTask 执行。

## Transition 映射

| Transition | Source -> target | depends / drives / ensures | continuation、checkpoint 与后续 |
| --- | --- | --- | --- |
| Preset | Base -> Prepared | 精确检查 Base、BootInitFlow Online 和 KernelInitTask 主线事实；驱动 PreSmpInit.Preset 并要求其 Online | `preset_after_pre_smp_init()` 提交 Prepared，发出 `SmpRuntimePhase.Prepared`，进入父 Setup |
| Setup | Prepared -> Ready | 精确检查 Prepared、PreSmpInit Online 和当前 task stack；驱动 SmpBringup.Preset 并要求其 Online | `setup_after_smp_bringup()` 提交既有 Ready，进入父 Enable |
| Enable | Ready -> Online | 精确检查 Ready、SmpBringup Online；依次驱动 RuntimeCore、Initcall、Rootfs、Finalize 的 Preset，并要求各自 Online | 四个具名 continuation 串联 sibling；`enable_after_finalize()` 提交 Online 后调用 `kernel::enable_after_smp_runtime()` |

```text
Kernel.Enable on KernelInitTask
  -> SmpRuntime.Preset -> PreSmpInit.Preset -> Online
  -> preset_after_pre_smp_init() -> SmpRuntime.Prepared
  -> SmpRuntime.Setup -> SmpBringup.Preset -> Online
  -> setup_after_smp_bringup() -> SmpRuntime.Ready
  -> SmpRuntime.Enable -> RuntimeCore.Preset -> Online
  -> enable_after_runtime_core() -> Initcall.Preset -> Online
  -> enable_after_initcall() -> Rootfs.Preset -> Online
  -> enable_after_rootfs() -> Finalize.Preset -> Online
  -> enable_after_finalize() -> SmpRuntime.Online
  -> kernel::enable_after_smp_runtime() -> PayloadPhase
```

每个叶子 Online 只能返回上述父 continuation。不得保留 child-to-sibling 直调、通用
`setup_after_children()` 或用 Ready 代表完成的查询。

## KernelInitTask 主线检查

`KernelInitTask` 提供可复用的当前 SP 栈范围查询。`SmpRuntimePhase` 入口和六个父 continuation
每次都必须验证：

- `KernelInitTask.state == Online` 且当前任务引用是 KernelInitTask；
- `Scheduler.kernel_init_stack_switch_started_count() == 1`；
- `KernelInitTask.entry_started_count() == 1` 且既有 entry SP verification 为真；
- 此刻读取的实际 `sp` 仍位于 KernelInitTask 的 16 KiB vmalloc stack。

这些检查覆盖 SmpRuntime 父阶段及六个 BP 主线叶子的 Started/Prepared/Ready/Online checkpoint。
SmpBringup 内的 AP checkpoint 由 AP idle task 执行并继续使用 AP owner，不得调用该 BP helper。

## Checkpoints

`SMP_RUNTIME_PHASE_STATE` 持久记录四个 model 状态。Started 表示 Preset 已接受；Prepared、Ready、
Online 都必须在状态提交后发出。

| Checkpoint | owner state | 落点 |
| --- | --- | --- |
| `SmpRuntimePhase.Started` | Base | `setup()` 接受 Preset 后、PreSmpInit 前 |
| `SmpRuntimePhase.Prepared` | Prepared | `preset_after_pre_smp_init()` |
| `SmpRuntimePhase.Ready` | Ready | `setup_after_smp_bringup()` |
| `SmpRuntimePhase.Online` | Online | `enable_after_finalize()`、Kernel continuation 前 |

六个叶子的对象动作、状态、checkpoint、context 和 continuation 分别见本目录对应文档。
