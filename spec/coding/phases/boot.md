# BootPhase 编码指引

BootPhase 是[Kernel 系统编码](../systems/kernel.md)的首个直接子阶段。其实现遵循
[阶段范式代码映射](../phase-paradigm.md)：Boot 的 transition 拥有子阶段 `drives` 顺序，
子阶段完成后返回 Boot continuation；子阶段之间不存在直接 sibling 调用。

## 父 Transition 驱动顺序

- `Boot.Preset` 驱动 `EntryPrelude.Preset`；子阶段 Online 后提交 Boot.Prepared，并由 Boot
  自身 `emits Setup`。
- `Boot.Setup` 驱动 `EntrySuccessor.Preset`；子阶段 Online 后提交 Boot.Ready，并由 Boot
  自身 `emits Enable`。
- `Boot.Enable` 依次驱动 `CorePrepare.Preset`、`MmCoreInit.Preset`、`SchedInit.Preset`；三个
  continuation 完成后提交 Boot.Online，再返回 `Kernel.Preset` continuation。

状态设置、checkpoint 和实际 continuation 名称在 Boot 子树审计中逐项确认。子阶段专题为：

- [boot/entry-prelude.md](boot/entry-prelude.md)
- [boot/entry-successor.md](boot/entry-successor.md)
- [boot/core-prepare.md](boot/core-prepare.md)
- [boot/mm-core-init.md](boot/mm-core-init.md)
- [boot/sched-init.md](boot/sched-init.md)
