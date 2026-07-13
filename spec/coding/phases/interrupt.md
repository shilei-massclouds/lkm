# InterruptPhase 编码指引

InterruptPhase 是[Kernel 系统编码](../systems/kernel.md)的第二个直接子阶段。其实现遵循
[阶段范式代码映射](../phase-paradigm.md)，由 Interrupt transition 保留子阶段驱动和 context
所有权，不生成 child-to-sibling 调用。

## 父 Transition 驱动顺序

- `Interrupt.Preset` 按 model 源码顺序驱动 `IrqTimeInit.Preset`、
  `LocalIrqEnable.Setup`、`IrqOpenPrepare.Setup`、`ProcessPrepare.Setup`，并保留各自 `within`
  边界；完成后提交 Interrupt.Prepared，再由 Interrupt 自身 `emits Setup`。
- `Interrupt.Setup` 检查四个子阶段完成事实，提交 Interrupt.Ready，再由 Interrupt 自身
  `emits Enable`。
- `Interrupt.Enable` 提交 Interrupt.Online，再返回 `Kernel.Setup` continuation。

非标准子阶段入口、状态设置、checkpoint 和 continuation 在 Interrupt 子树审计中逐项确认。
子阶段专题为：

- [interrupt/irq-time-init.md](interrupt/irq-time-init.md)
- [interrupt/local-irq-enable.md](interrupt/local-irq-enable.md)
- [interrupt/irq-open-prepare.md](interrupt/irq-open-prepare.md)
- [interrupt/process-prepare.md](interrupt/process-prepare.md)
