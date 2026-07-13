# UpMultitaskPhase 编码指引

UpMultitaskPhase 是 [Kernel 系统编码](../systems/kernel.md)的第三个直接子阶段。model 来源为
[`spec/model/phases/up-multitask/phase.spec`](../../model/phases/up-multitask/phase.spec)，实现
落点为 `impl/arceos_ex/src/phases/up_multitask/mod.rs`。本阶段拥有三个子阶段的父迁移、
completion continuation 和离开 BootIdle 栈的真实 handoff。

## Transition 映射

| Transition | Source -> target | depends / drives / ensures | continuation、checkpoint 与后续 |
| --- | --- | --- | --- |
| Preset | Base -> Prepared | 精确检查 Base 与 `InterruptPhase.Online`；驱动 `BootInitRestInitPhase.Preset` 并要求其 Online | `preset_after_boot_init_rest_init()` 提交 Prepared，发出 `UpMultitaskPhase.Prepared`，进入 UpMultitask.Setup |
| Setup | Prepared -> Ready | 精确检查 Prepared 与 RestInit Online；驱动 `BootInitScheduleHandoffPhase.Preset` 并要求其 Online | `setup_after_boot_init_schedule_handoff()` 提交 Ready，发出既有 `UpMultitaskPhase.Ready`，进入 UpMultitask.Enable |
| Enable | Ready -> Online | 精确检查 Ready 与 ScheduleHandoff Online；驱动 `BootIdleEntryPhase.Preset` 并要求其 Online | `enable_after_boot_idle_entry()` 提交 Online，发出 `UpMultitaskPhase.Online`，执行真实 BootIdle -> KernelInit handoff |

```text
Kernel.Enable
  -> UpMultitask.Preset -> BootInitRestInit.Preset -> Online
  -> preset_after_boot_init_rest_init() -> UpMultitask.Prepared
  -> UpMultitask.Setup -> BootInitScheduleHandoff.Preset -> Online
  -> setup_after_boot_init_schedule_handoff() -> UpMultitask.Ready
  -> UpMultitask.Enable -> BootIdleEntry.Preset -> Online
  -> enable_after_boot_idle_entry() -> UpMultitask.Online
  -> handoff_boot_idle_to_kernel_init()
  -> kernel_init_entry() [verify actual SP]
  -> kernel::enable_after_up_multitask()
  -> SmpRuntimePhase
```

三个叶子 Online 后只能调用表中的父 continuation。不得保留 child-to-sibling 调用或通用
`handoff()` 链。BootIdle continuation 若被恢复，只能进入无限 `schedule_idle()` 循环。

## Kernel continuation

`kernel_init_entry()` 在 KernelInitTask 的初始化 switch context 上第一次执行时读取实际 `sp`，
通过 `KernelInitTask.mark_entry_started()` 验证它位于该任务的 16 KiB vmalloc stack，随后调用
`systems::kernel::enable_after_up_multitask()`。该 continuation 必须检查：

- Kernel 精确处于 Ready；
- UpMultitaskPhase 和三个直接子阶段精确 Online；
- KernelInitTask 精确 Online；
- 真实 KernelInit stack switch start count 和 KernelInitTask entry start count 均恰为 1，且实际
  SP 已验证。

只有检查通过后才能调用既有 `smp_runtime::setup()`。`CurrentTaskRef::KernelInit` 是调度模型事实，
不能替代真实 stack switch 与 SP 验证。

## 状态与 checkpoint

`UP_MULTITASK_PHASE_STATE` 持久记录四个 model 状态。每次 transition start 精确检查 source，
父 continuation 精确检查对应叶子 Online 后提交 target。`is_online()` 仅在父阶段和三个直接
子阶段全部为 Online 时返回 true。

| 观察点 | owner state | 落点 |
| --- | --- | --- |
| `UpMultitaskPhase.Started` | Base | `setup()` 接受 Preset 后 |
| `UpMultitaskPhase.Prepared` | Prepared | `preset_after_boot_init_rest_init()` |
| `UpMultitaskPhase.Ready` | Ready | `setup_after_boot_init_schedule_handoff()` |
| `UpMultitaskPhase.Online` | Online | `enable_after_boot_idle_entry()`、真实 handoff 前 |

叶子阶段的对象动作、context、状态与 checkpoint 映射见
[up-multitask/rest-init.md](up-multitask/rest-init.md)。
