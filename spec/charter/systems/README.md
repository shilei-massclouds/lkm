# Charter Systems

完整模型只有一棵顶层系统树。固定根和直接子节点是：

```text
Computer
|- Riscv64Platform
|- OpenSBI
`- Kernel
   |- LinuxRiscv64KernelBootSpec
   |- KernelAddrSpace
   |- Vm
   |- Soc
   `- CpuGroup
      `- cpus[0] (BootCPU role)
         `- BootCpuRegisters
```

主题文件：

- [`computer.md`](computer.md)：唯一顶层根、assembly fact 和启动链入口。
- [`riscv64-platform.md`](riscv64-platform.md)：平台规格、构造与运行交接。
- [`opensbi.md`](opensbi.md)：固件规格、构造与 Kernel 交接。
- [`kernel.md`](kernel.md)：Linux/RISC-V64 kernel boot 规格采纳、kernel image 构造、入口交接与内部阶段树。
- [`../objects/kernel-address-space.md`](../objects/kernel-address-space.md)：唯一内核地址空间与区域归属。
- [`../objects/vm.md`](../objects/vm.md)：地址转换控制面；其四个 controller 由各自独立 Charter 文件定义。
- [`../objects/cpu.md`](../objects/cpu.md) 与
  [`../objects/cpu-group.md`](../objects/cpu-group.md)：CPU 实例、CurrentCPU selector 与唯一 owned 集合。
