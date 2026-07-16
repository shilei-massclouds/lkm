# arceos_ex 实现说明

本文记录 `arceos_ex` 的源码落点、运行命令、已经观察到的实现事实和阶段性取舍。它不是
coding 规则来源，也不覆盖 model、project/system/phase/object/testing 专题中的约束。

统一任务优先级和状态见 [`docs/ROADMAP.md`](../../docs/ROADMAP.md)。coding 阅读从
[`arceos_ex.md`](arceos_ex.md) 开始；对象文件级归属见
[`objects/README.md`](objects/README.md)。

## 权威规格入口

- 通用映射：[`mapping.md`](mapping.md)、[`phase-paradigm.md`](phase-paradigm.md)。
- Kernel project/system：[`projects/kernel.md`](projects/kernel.md)、
  [`systems/kernel.md`](systems/kernel.md)。
- 阶段映射：[`phases/README.md`](phases/README.md)。
- 普通对象：[`objects/README.md`](objects/README.md)。
- Rootfs 镜像构造：[`projects/rootfs-image.md`](projects/rootfs-image.md)。
- Rootfs/user 验收编排：[`../testing/rootfs.md`](../testing/rootfs.md)。
- 构建门禁：[`build.md`](build.md)。

本文件后文的“实现证据”只说明当前代码怎样落实这些入口。若证据和权威规格不一致，先把差异
作为审计问题处理，不从本文反向解释规则。

## 当前工程边界

源码位于 `impl/arceos_ex/`，由仓库根 `Makefile` 构建和运行。当前实验直接完成对象级内核路径，
尚未恢复 tgoskits/ArceOS crate 兼容、overlay workspace、feature 传递、`axlog` 或 `ax-alloc`
facade；这些属于 roadmap 中延期的 Composition Phase。

当前实现已经贯通 Boot、Interrupt、UpMultitask、SmpRuntime 与 Payload 的模型阶段树。默认 smoke
payload 执行对象级用例后通过 SBI 关机；`APP=hello` 观察最终 KernelInitTask payload handoff；
`APP=user-boot` 观察 ext2/VFS/ELF/user-entry/syscall 路径。

## 常用命令

```bash
make build
make build APP=smoke
make build APP=hello
make build APP=user-boot
make run
make run APP=smoke
make run APP=hello
make run APP=user-boot
make run PROBE=announce
make verify
make verify REPORT=graph
make coding-spec-check
make test
make test-kunit
make test-smoke
make clean
```

`KERNEL ?= arceos_ex` 选择默认内核，`APP ?= smoke` 选择 payload。`PROBE=announce` 打开 checkpoint
自声明 consumer；历史 `LOG=trace` 仍是兼容 alias。`make test` 的实际组成和顺序以
[`build.md`](build.md) 及根 Makefile 为准。

## 阶段实现证据

### ProcessPreparePhase

权威映射：[`phases/interrupt/process-prepare.md`](phases/interrupt/process-prepare.md)。

实现位于 `impl/arceos_ex/src/phases/interrupt/process_prepare.rs`。当前路径在本地 IRQ 打开之后建立
ProcessPrepare 的 runtime service facts、VFS 初始 ramfs root、FsStruct/FilesStruct 前置对象和
UpMultitask handoff；测试读取 checkpoint 时已有的事实，不在 handler 中推进对象。

### Completion

权威映射：[`objects/completion.md`](objects/completion.md)。

`impl/arceos_ex/src/objects/completion.rs` 提供共享 Completion 实现；KthreaddReadyGate 等实例复用
同一对象协议。现有 smoke 覆盖 setup/enable、token 产生、观察和消费，并观察启动路径中的 live
instance。

### rest_init 与首轮 task handoff

权威映射：[`phases/up-multitask/rest-init.md`](phases/up-multitask/rest-init.md)、
[`objects/effective-context.md`](objects/effective-context.md) 和 [`riscv64.md`](riscv64.md)。

实现分布在 `impl/arceos_ex/src/phases/up_multitask/rest_init.rs`、scheduler/task/context 对象与 RISC-V
switch lowering。BootIdleTask 保留静态 boot stack；KernelInitTask/KthreaddTask 使用新分配的 vmalloc
stack。最终线性启动 handoff 保存 BootIdle context、恢复 KernelInitTask stack，由
`kernel_init_entry()` 继续 SmpRuntime/Payload。BootIdle continuation 进入 `schedule_idle()`，
KthreaddTask 当前进入简化调度循环。完整 kthreadd 请求消费、通用 scheduler class/fairness 和更完整
返回语义仍是后续工作。

### PreSmpInit、SMP bringup 与 RuntimeCore

权威映射：[`phases/smp-runtime/pre-smp-init.md`](phases/smp-runtime/pre-smp-init.md)、
[`phases/smp-runtime/smp-bringup.md`](phases/smp-runtime/smp-bringup.md)、
[`phases/smp-runtime/runtime-core.md`](phases/smp-runtime/runtime-core.md)。

对应源码位于 `impl/arceos_ex/src/phases/smp_runtime/`。当前证据包括 kthreadd completion wait、
KernelInitTask entry stack、BP/AP topology、SBI HSM boot data、AP online completion、scheduler SMP
action、workqueue topology，以及按配置保留的 deferred/trimmed runtime cores。

### InitcallPhase

权威映射：[`phases/smp-runtime/initcall.md`](phases/smp-runtime/initcall.md)。

`impl/arceos_ex/src/phases/smp_runtime/initcall.rs` 与 `objects/initcall.rs` 收集 linker section entry，
按 level 执行 constructor。当前生产路径经 OF platform population 建立 PlatformDevice/Bus/Driver，
完成 ns16550a、console handoff、virtio-mmio、virtio RNG/block 和 devfs 首片。driver/core/console 的
职责边界、probe window 和 checkpoint 位置都由上述 phase 文档承载。

### RootfsPhase

权威映射：[`phases/smp-runtime/rootfs.md`](phases/smp-runtime/rootfs.md)、
[`objects/ext2.md`](objects/ext2.md) 与 [`objects/vfs.md`](objects/vfs.md)。

`impl/arceos_ex/src/phases/smp_runtime/rootfs.rs` 从 Online 的 default block device 建立
Ext2Driver/Volume/FileSystem，经 `/root` staging mount、mount move 与 ChrootDot 把 FsStruct root/pwd
切到 ext2 root。当前实现保留 initramfs、device-probe wait、rootwait/initrd/md/NFS/CIFS/devtmpfs 与
integrity-key 的配置分类事实；未展开路径仍以 model/phase 文档为准。

### FinalizePhase 与 PayloadPhase

权威映射：[`phases/smp-runtime/finalize.md`](phases/smp-runtime/finalize.md) 与
[`phases/payload.md`](phases/payload.md)。

实现记录 async cleanup boundary、system state、RCU boot-end 和裁剪事实，随后选择 smoke/hello/user
payload。当前 hello 已在 KernelInitTask vmalloc stack 上完成最终输出；user payload 继续进入下述
对象链。

## 普通对象实现证据

### DeviceTree

权威映射：[`objects/device-tree.md`](objects/device-tree.md) 与
[`phases/boot/core-prepare.md`](phases/boot/core-prepare.md)。

当前 unflatten 使用 MemBlock-backed storage 和既有 linear mapping；两遍扫描后建立 root、parent/child、
path/property 查询事实。unsafe pointer 写集中在内部构造边界，公开查询保持借用范围明确。

### Allocator、vmalloc 与 ioremap

权威映射：[`phases/boot/mm-core-init.md`](phases/boot/mm-core-init.md)。

`impl/arceos_ex/src/objects/` 下的 memblock/page allocator/slub/vmalloc/ioremap 实现已经支持启动期 handoff、
普通 kmalloc/GlobalAlloc、动态容器和当前 MMIO mapping。vmalloc 记录 area/mapping metadata 并能在完整
VMALLOC window 内按需安装页表；unmap/free、空洞复用、更完整树查找与更多 memory attribute 仍未闭合。

### Virtio、block、Ext2 与 VFS

权威映射：[`objects/virtio.md`](objects/virtio.md)、[`objects/virtio-blk.md`](objects/virtio-blk.md)、
[`objects/block-device.md`](objects/block-device.md)、[`objects/bio.md`](objects/bio.md)、
[`objects/ext2.md`](objects/ext2.md) 和 [`objects/vfs.md`](objects/vfs.md)。

源码按 `objects/virtio*`、`block_device.rs`、`bio.rs`、`ext2.rs`、`vfs.rs` 和 `devfs.rs` 分布。
QEMU virtio devices 经 platform probe 注册 generic VirtioDevice；RNG 经 HwRngCore 暴露 current device；
block read 经 Bio/BufferHead、同步 token completion 和 registry provider；Ext2 解析 1/2/4KiB block、
direct 与 single-indirect regular-file read；VFS 提供 mount crossing、fast symlink 和当前 pathname API。

### User boot、files、syscall 与 process

权威映射：[`objects/user-boot.md`](objects/user-boot.md) 与 [`objects/vfs.md`](objects/vfs.md)。

源码集中在 `impl/arceos_ex/src/objects/user_boot.rs` 及相关 VFS/files/task/trap 模块。当前实现可从 ext2
root 读取 dynamic musl ELF 与 interpreter，建立 auxv/address space/trap frame，进入 U-mode，并经
FilesStruct/fd/OFD/backend 调度当前 syscall slice。BusyBox shell 外部命令路径使用单 observed-child
handoff；exec replacement address space 放在 Context-owned staging，避免 4KiB trap stack 上的大对象。

现有 focused 证据已经越过 clone/wait/exec 主线、OpenRC login 认证、基础 credential drop 和登录 shell
内一次 `/bin/ls` child，并闭合单 pending grandchild 在 handoff/exec 前的父侧 setpgid 与后续 TIOCSPGRP。
完整 task graph、多 pending child、post-exec parent setpgid、COW/mm、signals、networking、完整 exec
rollback/reclamation 等仍是 deferred 行为。

### Console、TTY 与 IRQ

权威映射：[`phases/smp-runtime/initcall.md`](phases/smp-runtime/initcall.md)、
[`phases/interrupt/irq-time-init.md`](phases/interrupt/irq-time-init.md) 和
[`phases/interrupt/irq-open-prepare.md`](phases/interrupt/irq-open-prepare.md)。

实现已经完成 boot console 到 serial8250 console route/cursor handoff、PLIC source/domain/handler 链、
interrupt-driven printk TX，以及受控 ordinary TTY TX/RX loopback probes。checkpoint/KUnit 读取真实链路
facts；普通 payload 只通过 printk 或 files/TTY 前端产生输出。

## Checkpoint 与差分证据

权威 mapping 和工具边界位于 [`mapping.md`](mapping.md) 与
[`phases/interrupt/irq-time-init.md`](phases/interrupt/irq-time-init.md)；rootfs case 编排位于
[`../testing/rootfs.md`](../testing/rootfs.md)。

`impl/arceos_ex/src/checkpoint/mod.rs` 是 checkpoint enum/name/early-byte 的实现源。工具产物位于
`tools/out/checkpoints/`，覆盖 inventory、Linux source mapping、coverage review 和 exact-marker plan。
当前默认 paired difftest 是 direct-inittab rc.local case；historical shell baseline 仍可显式选择。
failure diagnostics 使用稳定 phase/step/object/check/first-failed 字段并保留原 EventError 行，便于 stress
分类，而不改变成功事件序列。

## 构建与测试证据

rootfs 构造规则见 [`projects/rootfs-image.md`](projects/rootfs-image.md)，测试编排见
[`../testing/rootfs.md`](../testing/rootfs.md)。当前 Makefile 支持 Alpine minirootfs、compiled fixture map、
static file overlay、case-local disk、显式 rebuild 和 QEMU command line passthrough。

smoke 负责 payload/端到端可观察行为；checkpoint KUnit 负责真实 checkpoint 时刻的只读事实；stress runner
负责 ordinary-path 重复执行、序列分类和代表样本归档。具体门禁集合和 clean-build/Clippy 矩阵由
[`build.md`](build.md) 维护。

## `make verify` 历史观察

历史提交曾暴露 `13 obligation / 2 deferred`。Lds、OpenSBI DTB handoff 与 BootCPU 前序事实随后补入
推导，当前报告已收敛为零 unresolved obligation。该历史用于解释为何入口布局、firmware handoff 和对象
前序事实都保留显式 source/check 边界；obligation 的正式处理规则位于 coding 根入口与 model semantics。

## Deferred 审计迁移状态

原候选清单已经完成逐项审计：已实现内容回到正式 facts；真实剩余责任和配置/架构/输入裁剪进入
结构化 model inventory。唯一 disposition 记录见
[`deferred-trimmed-audit.md`](../../docs/roadmap/deferred-trimmed-audit.md)，本文件不再复制可独立演化的
backlog。实现和评审必须引用 model boundary ID。

## 源码结构

- `impl/arceos_ex/src/main.rs`：startup timeline 和顶层入口。
- `impl/arceos_ex/src/phases/`：按 model phase 树组织的过程实现。
- `impl/arceos_ex/src/objects/`：普通对象、共享 primitive 与 storage/facade。
- `impl/arceos_ex/src/checkpoint/`：checkpoint identity、consumer 与 sink glue。
- `impl/arceos_ex/src/arch/riscv64/`：entry、trap、switch、page-table 与 SBI/FDT 边界。
- `impl/arceos_ex/tests/`：smoke、KUnit handlers、user fixtures、stress/difftest cases。

对象目录当前仍以语义拆分为先；主题子目录化是 roadmap 中的后续结构工作。

## 阶段性待确认事项

- vmalloc stack / IRQ stack 泛化：`process_prepare.011`、`irq_time.007`。
- 通用 task graph、job control 与未观察组合：`user_clone.014`–`user_clone.016`。
- fork/wait/reap、COW/mm、signals 和 exec synchronization：`user_clone.002`–`.014`、
  `exec_sync.001`–`.003`。
- component/crate 封装恢复时与现有对象 API 的适配层位置。

这些事项只描述当前实现观察；是否推进、优先级和剩余责任只在
[`docs/ROADMAP.md`](../../docs/ROADMAP.md) 维护。
