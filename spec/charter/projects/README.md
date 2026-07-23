# Charter Projects

工程树与运行系统树彼此独立。工程树的固定根和直接子节点是：

```text
ComputerProject
|- HardwareProject
|- FirmwareProject
`- KernelProject
```

主题文件：

- [`computer.md`](computer.md)：总工程的编排、组装事实与启动交接。
- [`hardware.md`](hardware.md)：RISC-V 平台规格与 boot-hart context 构造。
- [`firmware.md`](firmware.md)：SBI/OpenSBI 规格与 BootArgs 构造。
- [`kernel.md`](kernel.md)：Config、Lds 和 kernel image 构造。
