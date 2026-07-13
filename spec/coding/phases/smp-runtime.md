# SmpRuntimePhase 编码指引

SmpRuntimePhase 是[Kernel 系统编码](../systems/kernel.md)中 `Kernel.Enable` 的第二个 drive 目标，
由 KernelInitTask 执行。其实现遵循[阶段范式代码映射](../phase-paradigm.md)，子阶段完成后返回
SmpRuntime continuation，而不是直接推进下一 sibling。

## 父 Transition 驱动顺序

- `SmpRuntime.Preset` 按 model 顺序驱动 `PreSmpInit.Setup`、`SmpBringup.Setup`、
  `RuntimeCore.Setup`、`Initcall.Setup`、`Rootfs.Setup`、`Finalize.Setup`；完成后提交
  SmpRuntime.Prepared，再由本阶段 `emits Setup`。
- `SmpRuntime.Setup` 检查六个子阶段 Ready，提交 SmpRuntime.Ready，再由本阶段
  `emits Enable`。
- `SmpRuntime.Enable` 提交 SmpRuntime.Online，再返回 `Kernel.Enable` continuation；由 Kernel
  选择下一项 `Payload.Preset`。

非标准子阶段入口、状态/checkpoint 和实际 continuation 在本子树审计中确认。子阶段专题为：

- [smp-runtime/pre-smp-init.md](smp-runtime/pre-smp-init.md)
- [smp-runtime/smp-bringup.md](smp-runtime/smp-bringup.md)
- [smp-runtime/runtime-core.md](smp-runtime/runtime-core.md)
- [smp-runtime/initcall.md](smp-runtime/initcall.md)
- [smp-runtime/rootfs.md](smp-runtime/rootfs.md)
- [smp-runtime/finalize.md](smp-runtime/finalize.md)
