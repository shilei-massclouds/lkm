# Charter Systems

完整模型只有一棵顶层系统树。固定根和直接子节点是：

```text
Computer
|- Riscv64Platform
|- OpenSBI
`- Kernel
   |- LinuxRiscv64KernelBootSpec
   `- BootCurrentCPU
      `- BootCPU
         `- BootCpuRegisters
```

主题文件：

- [`computer.md`](computer.md)：唯一顶层根、assembly fact 和启动链入口。
- [`riscv64-platform.md`](riscv64-platform.md)：平台规格、构造与运行交接。
- [`opensbi.md`](opensbi.md)：固件规格、构造与 Kernel 交接。
- [`kernel.md`](kernel.md)：Linux/RISC-V64 kernel boot 规格采纳、kernel image 构造、入口交接与内部阶段树。
