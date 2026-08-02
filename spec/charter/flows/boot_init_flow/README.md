# BootInitFlow

`BootInitFlow` 是静态 `BootTask.flow` 指向的终身 TaskFlow。它从 `_start` 编排 boot execution，提交
Online 后仍承载同一 PID 0 的 idle setup、首次 Schedule 返回、`BootIdleEntryPhase` 和 idle loop。
不存在第二个 boot idle Flow；`BootTask` 首次真实切出时才保存 context 并进入 Online，未来通过统一
Continue 返回同一个 BootInitFlow continuation。

## 生命周期与不变量

```text
Base --Preset--> Prepared --Setup--> Ready --Enable--> Online
```

每个 transition/action 都重新校验固定 parent、FlowRef/generation、CpuRef 与 effective-flow guard。
BootInitFlow 不退出，终身属于 BootTask。Preset、Setup、Enable/Online 的完整职责分别由本目录对应
专题约束；完整 SMP arbitration、迁移与 replay 保持 P2 延期。

## 导航

- [`Preset`](preset.md)：Kernel.Enable 接受后的入口编排与 Prepared 提交。
- [`Setup`](setup.md)：`start_kernel()`、直接对象与叶阶段编排，以及 Ready 提交。
- [`Enable / Online`](enable.md)：首次调度预检、Online 提交、恢复 continuation 与 idle loop。

## 引用

- [阶段范式](../../phase-paradigm.md)
- [BootInitFlow model](../../../model/flows/boot_init_flow/main.spec)
- [BootInitFlow coding](../../../coding/flows/boot_init_flow/README.md)
- [Kernel 系统](../../systems/kernel.md)
