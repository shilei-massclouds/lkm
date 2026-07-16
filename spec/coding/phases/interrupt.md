# InterruptPhase 编码指引

InterruptPhase 是 [Kernel 系统编码](../systems/kernel.md)的第三个直接子阶段。model 来源为
[`spec/model/phases/interrupt/phase.spec`](../../model/phases/interrupt/phase.spec)，实现落点为
`impl/arceos_ex/src/phases/interrupt/mod.rs`。本阶段拥有四个子阶段的顺序、context 和
continuation；子阶段之间不存在直接调用边。

## Transition 映射

| Transition | Source -> target | Start / continuation | depends / drives / ensures | 提交、checkpoint 与后续 |
| --- | --- | --- | --- | --- |
| Preset | Base -> Prepared | `setup()` 接受；四个 `preset_after_*()` 依次恢复 | 精确检查 Base 与 `BootPhase.Online`；按 model context 依次驱动四个子阶段 Preset；每次恢复检查刚完成子阶段 Online | ProcessPrepare Online 后提交 Prepared，发出 `InterruptPhase.Prepared`，调用 Interrupt.Setup |
| Setup | Prepared -> Ready | `setup_after_children()` | 精确检查 Prepared 和四个子阶段 Online | 提交 Ready，发出 `InterruptPhase.Ready`，调用 Interrupt.Enable |
| Enable | Ready -> Online | `enable()` | 精确检查 Ready 和四个子阶段 Online | 提交 Online，发出 `InterruptPhase.Online`，返回 `kernel::setup_after_interrupt()` |

## Preset continuation

```text
Interrupt.Preset
  -> IrqTimeInit.Preset [SingleTaskContext]
  -> preset_after_irq_time_init()
  -> LocalIrqEnable.Preset [no outer context]
  -> preset_after_local_irq_enable()
  -> IrqOpenPrepare.Preset [SingleTaskInterruptStreamContext]
  -> preset_after_irq_open_prepare()
  -> ProcessPrepare.Preset [SingleTaskInterruptStreamContext]
  -> preset_after_process_prepare()
  -> Interrupt.Prepared -> Interrupt.Setup -> Interrupt.Ready
  -> Interrupt.Enable -> Interrupt.Online
  -> Kernel.setup_after_interrupt()
```

上述四个 `preset_after_*()` 都属于 Interrupt.Preset；叶子阶段 Online 后只返回对应父
continuation。正常路径不得保留泛化 `handoff()`，也不得让叶子阶段直接启动 sibling。

## Context lowering

- IrqTimeInit 的 Base -> Online 全链运行在 boot CPU 的 `SingleTaskContext` 事实内，且 SIE 必须
  保持关闭。
- LocalIrqEnable 不加外层 context；其 Preset 自身先清除 early flag，再通过
  `InterruptStream.Enable` 打开 SIE。
- IrqOpenPrepare 和 ProcessPrepare 在 boot CPU 已开放总入口、任务与 SMP 并发仍关闭的
  `SingleTaskInterruptStreamContext` 中执行。

当前 Rust 启动路径不构造仅用于标记的 context guard；context 通过各 transition 的前置/后置
事实和状态检查 lowering，不能因此省略 model 中的 `within` 所有权。

## 状态与 checkpoint

`INTERRUPT_PHASE_STATE` 持久记录 Base、Prepared、Ready、Online。每个 transition start 检查
精确 source state；提交后读回 target 并检查四个子阶段 Online，再发出同名 checkpoint。
`is_online()` 只在父阶段与四个直接子阶段都精确 Online 时返回 true。

| 观察点 | owner state | 落点 |
| --- | --- | --- |
| `InterruptPhase.Started` | Base | `setup()` 接受 Preset 后 |
| `InterruptPhase.Prepared` | Prepared | `preset_after_process_prepare()` |
| `InterruptPhase.Ready` | Ready | `setup_after_children()` |
| `InterruptPhase.Online` | Online | `enable()` |

子阶段专题为：

- [interrupt/irq-time-init.md](interrupt/irq-time-init.md)
- [interrupt/local-irq-enable.md](interrupt/local-irq-enable.md)
- [interrupt/irq-open-prepare.md](interrupt/irq-open-prepare.md)
- [interrupt/process-prepare.md](interrupt/process-prepare.md)
