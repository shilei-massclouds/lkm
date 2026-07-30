# OpenSBI System Charter

`OpenSBI` 是 `Computer` 的直接子系统和固件运行实例，承担 SBI/OpenSBI 规格采纳、固件构造与 Kernel
交接。只读外部标准 `SbiSpec` 和只读启动 ABI 实参 `BootArgs` 的静态 parent 均为 `OpenSBI`。

OpenSBI 初态为 Base：

- `Preset` 采纳 `SbiSpec`、规定 `BootArgs` 的类型和含义并建立 OpenSBI 系统规格，提交 Prepared；它
  不构造或改变初态 Online 的只读输入实例。
- `Setup` 验证规格并构造 OpenSBI 固件，提交 Ready。
- `Enable` 一次性验证平台 Online、固件、SBI/BootArgs、已采纳的 Linux/RISC-V64 kernel boot 规格、
  Config/Lds，以及 ELF 与 boot Image 文件构造事实。它为本次 handoff 选择 `kernel_load_pa`，分别
  建立 boot Image 已装载到该待交接地址、该地址满足 PMD/2 MiB 对齐两个事实；同时选择 SBI HSM
  ordered booting，只释放 primary hart，并精确保证
`BootCpuRegisters.a0 == BootArgs.boot_hartid` 与
`BootCpuRegisters.a1 == BootArgs.dtb_pa`、`BootCpuRegisters.satp == 0`，同时保证 DTB 位于入口可访问
的 RAM 且 blob 完整。提交 OpenSBI Online 后先同步驱动 `CpuGroup.Preset`，由内核入口 adoption
原子发布 `CpuGroup.cpus[0]` 与 CpuGroup Prepared；只有该响应完成后才异步发送 `Kernel.Enable`。
这条边界表达控制权交接；除上述直接
入口契约外，它不表示其它启动相关寄存器已经具有 Kernel 最终值。尤其不要求 OpenSBI 预先关闭
BootCPU 的中断分路门控或完成待决中断清除写：Kernel 在 `BootInitFlow.Preset` 的第一个入口动作中
自行按序完成该防御性写；硬件驱动的待决位可以在写后再次置位。OpenSBI 不拥有
Kernel，也不拥有 Kernel 的内部 phase。

装载事实描述 OpenSBI Enable 交接域必须保证的结果，不虚构 OpenSBI 固件内部执行复制。QEMU/loader
可以完成实际字节放置，但 OpenSBI 只有在观察并保证其结果、选择相同的交接入口地址后才能提交
Online。

OpenSBI.Ready 只承载规格与 firmware 构造完成事实，不提前承载 ordered boot 已选定、primary hart
已进入或入口寄存器已经提交等运行期 handoff 事实；这些事实由 Enable 建立并由 Online invariant 保持。
`kernel_load_pa` 同样只由 Enable 选择，并由 Online 保持；Ready 不含物理装载地址或装载完成事实。
其中 `satp == 0` 的 live CSR 等式只约束 OpenSBI 提交与 `Kernel.Enable` 接受边界；Kernel 随后会切换
页表，OpenSBI.Online 保持的是“交接时 satp 为零”的稳定 handoff 记录，不要求 live satp 永远为零。

OpenSBI.Online 只表示固件实例已经启动并提交 Kernel 入口控制权；下游失败不回滚该状态。
CpuGroup 是 Kernel 的直接子对象；上述同步效果由真实 Kernel 入口在接受 Enable 前采用，不要求修改外部
OpenSBI 固件或让固件直接构造 Rust 对象。

## Mapping

- Model: `spec/model/systems/opensbi.spec`
- Coding: `spec/coding/systems/opensbi.md`
- Implementation: `impl/arceos_ex/src/systems/opensbi.rs`
