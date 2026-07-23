# Firmware Project Charter

`FirmwareProject` 负责 SBI/OpenSBI 规格和固件构造产物，不承担 OpenSBI 运行时启动响应。

## Lifecycle and products

- Preset 采纳外部 `SbiSpec`，建立 `OpenSBI` 系统规格，然后停在 Prepared。
- Setup 定义启动 ABI，顺序驱动 `BootArgs.Setup` 与 `BootArgs.Enable`，构造 OpenSBI 固件，然后停在
  Ready。

`SbiSpec` 是只读外部标准，parent 是 `FirmwareProject`。`BootArgs` 是 FirmwareProject 定义并构造的
只读静态 ABI 产物，parent 同样是 `FirmwareProject`；`boot_hartid` 与 `dtb_pa` 在工程 Setup 时决定，
不由 `Riscv64.a0/a1` 或 Kernel 入口反向定义。

运行时 `OpenSBI.Enable` 负责使 `BootCpuRegisters.a0/a1` 与已 Online 的 `BootArgs` 一致，再向
Kernel 发送启动信号；它不承诺其它启动相关寄存器已经准备完成。

## Mapping

- Model: `spec/model/projects/firmware.spec`
- Coding: `spec/coding/projects/firmware.md`
- Implementation: `impl/arceos_ex/src/projects/firmware.rs`
