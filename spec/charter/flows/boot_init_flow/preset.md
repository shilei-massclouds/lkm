# BootInitFlow.Preset

OpenSBI 发出 `Kernel.Enable` 后，Kernel 在 Ready 的同一 handler 中依次完成：

1. `Kernel.Action::AcceptEnable`；
2. `BootInitFlow.Action::AssignCpuRef(BootCPURef)`；
3. `PhysicalDirect.Action::ActivateOnCpu(BootCPURef)` 的 InitialActivation；
4. canonical `BootInitFlow.Transition::Preset`。

Preset 接受时 Flow 必须 Base、parent 必须是 OnCpu/Live BootTask；`Started` 只是在第一个 child 前记录的
checkpoint。入口依次完成 BootCPU interrupt route mask/pending clear、浮点/向量关闭、BSS 清零、hartid
记录、PhysicalDirect 下 boot-only `CurrentTask.BindTaskStack(BootTask, BootTask.stack)`、临时 trap、
Vm.Preset/Setup、正式 TrapType.Setup、EarlyVm 下 `RefreshTaskStack`、Soc.Preset，最后提交 Prepared。
失败不得部分提交 Flow、CurrentTask/CurrentStack 或 translation controller。

## 引用

- [BootInitFlow](README.md)
- [BootInitFlow.Preset model](../../../model/flows/boot_init_flow/main.spec)
- [BootInitFlow.Preset coding](../../../coding/flows/boot_init_flow/preset.md)
