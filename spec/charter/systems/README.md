# Charter Systems

运行系统树独立于工程树。固定根和直接子节点是：

```text
Computer
|- Riscv64Platform
|- OpenSBI
`- Kernel
   `- BootCurrentCPU
      `- BootCPU
         `- BootCpuRegisters
```

主题文件：

- [`computer.md`](computer.md)：运行系统根和启动链入口。
- [`riscv64-platform.md`](riscv64-platform.md)：由 Ready 经 Enable 启动的平台系统。
- [`opensbi.md`](opensbi.md)：固件运行实例和 Kernel 交接。
- [`kernel.md`](kernel.md)：现有 Kernel 生命周期与内部阶段树。
