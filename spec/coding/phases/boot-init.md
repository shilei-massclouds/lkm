# BootInitFlow 编码指引

`BootInitFlow` 是静态 Online `BootTask` 的 `PhaseObject` 子对象，不是 `TaskFlow`。model 来源为
[`spec/model/phases/boot-init/phase.spec`](../../model/phases/boot-init/phase.spec)，实现落点为
`impl/arceos_ex/src/phases/boot_init/mod.rs`。它从 `_start` 开始覆盖入口、Boot、Interrupt、rest_init
和首次 PID 1 切换的 pre-commit 边界。

## Transition 映射

| Transition | Source -> target | depends / drives / ensures | continuation、checkpoint 与后续 |
| --- | --- | --- | --- |
| Preset | Base -> Prepared | `BootTask` 必须已是 Online；驱动 `EntryPreludePhase.Preset` 并要求其 Online | `preset_after_entry_prelude()` 提交 Prepared，发出 `BootInitFlow.Prepared`，返回 Kernel.Preset completion |
| Setup | Prepared -> Ready | 驱动 `BootPhase.Preset`、随后驱动 `InterruptPhase.Preset` | `setup_after_interrupt()` 检查两棵子树 Online，提交 Ready，发出 `BootInitFlow.Ready`，返回 Kernel.Setup completion |
| Enable | Ready -> Online | 驱动 `BootInitRestInitPhase.Preset`、`BootInitScheduleHandoffPhase.Preset`；后者只建立 `BootIdleFlow` Ready/owner/active binding 和首次切换预检 | `enable_after_boot_init_schedule_handoff()` 提交 Online，发出 `BootInitFlow.Online`，随后 Kernel.Enable 才调用真实 `Scheduler.schedule()` |

```text
_start: Kernel.Started -> BootTask.Online -> BootInitFlow.Started
  -> EntryPreludePhase -> BootInitFlow.Prepared -> Kernel.Prepared
  -> BootPhase -> InterruptPhase -> BootInitFlow.Ready -> Kernel.Ready
  -> BootInitRestInitPhase -> BootInitScheduleHandoffPhase
  -> BootInitFlow.Online
  -> real BootTask-to-KernelInitTask schedule switch
  -> kernel_init_entry() [verify actual SP]
  -> kernel::enable_after_boot_init()
  -> SmpRuntimePhase
```

每个叶子 Online 只能调用其所属父 transition 的具名 continuation。`BootInitFlow.Online` checkpoint
必须紧邻真实 `Scheduler.schedule()` 之前；该 checkpoint 之前的失败仍走既有 fail-stop，之后不得伪造
rollback。

`BootIdleEntryPhase` 不是 `BootInitFlow` 子阶段，也不能在 PID 1 入口执行。首次 schedule 将来恢复
`BootTask` 时，原 schedule 调用的返回 continuation 才在 `BootIdleStartupContext` 中驱动
`BootIdleEntryPhase`，随后进入不返回的 `schedule_idle()` 循环。

## Kernel continuation

`kernel_init_entry()` 在 `KernelInitTask` 的初始化 switch context 上第一次执行时读取实际 `sp`，
通过 `KernelInitTask.mark_entry_started()` 验证它位于该任务的 16 KiB vmalloc stack，随后调用
`systems::kernel::enable_after_boot_init()`。该 continuation 必须检查：

- Kernel 精确处于 Ready；
- `BootInitFlow`、两个 boot-init 叶子阶段精确 Online；
- `BootIdleFlow` 已是 Ready，且 owner/active binding 指向 Online `BootTask`；
- `KernelInitTask` 精确 Online；
- 首次 schedule/switch、entry count 均恰为 1，且实际 SP 已验证。

只有这些检查通过后才能调用 `smp_runtime::setup()`。CPU-local current `TaskRef` 事实不能替代真实
stack switch 与 SP 验证。

## 状态与 checkpoint

`BOOT_INIT_FLOW_STATE` 持久记录四个 Phase 状态。`BootTask` 不由该状态机推进；所有边界都必须静默
验证 `BootTask` 仍为 Online 且 canonical storage/PID/ref 未变化。

| 观察点 | owner state | 落点 |
| --- | --- | --- |
| `BootInitFlow.Started` | Base | `_start` 中紧随 `BootTask.Online` |
| `BootInitFlow.Prepared` | Prepared | `preset_after_entry_prelude()` |
| `BootInitFlow.Ready` | Ready | `setup_after_interrupt()` |
| `BootInitFlow.Online` | Online | `enable_after_boot_init_schedule_handoff()`、真实 schedule commit 前 |

叶子阶段的对象动作、context、状态与 checkpoint 映射见
[boot-init/rest-init.md](boot-init/rest-init.md)。
