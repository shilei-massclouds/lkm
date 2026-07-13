# UpMultitaskPhase 编码指引

UpMultitaskPhase 是[Kernel 系统编码](../systems/kernel.md)的第三个直接子阶段。其实现遵循
[阶段范式代码映射](../phase-paradigm.md)；跨 BootInitTask、BootIdleTask 和 KernelInitTask 的
物理控制权交接必须仍能定位到 UpMultitask/Kernel 的父 continuation。

## 父 Transition 驱动顺序

- `UpMultitask.Preset` 按 model 顺序驱动 `BootInitRestInit.Setup`、
  `BootInitScheduleHandoff.Setup`、`BootIdleEntry.Setup`；完成后提交 UpMultitask.Prepared，再由
  UpMultitask 自身 `emits Setup`。
- `UpMultitask.Setup` 检查三个子阶段 Ready，提交 UpMultitask.Ready，再由本阶段
  `emits Enable`。
- `UpMultitask.Enable` 提交 UpMultitask.Online，再返回 `Kernel.Enable` 的下一 drive
  continuation。BootIdle 无限循环不拥有 SmpRuntime 的 sibling 调用权。

任务所有权、非标准子阶段入口、状态/checkpoint 和跨栈 continuation 在本子树审计中确认。
叶子阶段专题为 [up-multitask/rest-init.md](up-multitask/rest-init.md)。
