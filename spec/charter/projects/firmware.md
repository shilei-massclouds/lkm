# Firmware Project Charter

`FirmwareProject` 负责 SBI/OpenSBI 规格和固件构造产物，不承担 OpenSBI 运行时启动响应。

## Lifecycle and products

- Preset 采纳外部 `SbiSpec`，建立 `OpenSBI` 系统规格，然后停在 Prepared。
- Setup 构造 OpenSBI 固件，然后停在 Ready；Ready invariant 读取初态已为 Online 的 `BootArgs`，
  但 Setup 不驱动其生命周期，也不在此时构造或决定其实参。

`SbiSpec` 是只读外部标准，parent 是 `FirmwareProject`。`BootArgs` 是从模型观察起点就已经存在的
只读启动 ABI 实参对象，parent 同样是 `FirmwareProject`，保存本次启动已给定的 `boot_hartid` 与
`dtb_pa`。FirmwareProject 规定或采纳这些参数的类型和含义，但不通过生命周期构造其实例；参数也
不由 `Riscv64.a0/a1` 或 Kernel 入口反向定义。`BootArgs.Online` 只表示这些不可变参数事实可访问，
不表示 OpenSBI 已完成向 Kernel 的控制权交接。

运行时 `OpenSBI.Enable` 负责使 `BootCpuRegisters.a0/a1` 与已 Online 的 `BootArgs` 一致，再向
Kernel 发送启动信号；它不承诺其它启动相关寄存器已经准备完成。

## Mapping

- Model: `spec/model/projects/firmware.spec`
- Coding: `spec/coding/projects/firmware.md`
- Implementation: `impl/arceos_ex/src/projects/firmware.rs`
