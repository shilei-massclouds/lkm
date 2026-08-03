# Roadmap 完成项索引

本文件只记录领域、完成任务名和保存完整证据的唯一专题，不复制验收日志、命令或实现细节。
当前状态、优先级和剩余责任只在 [`../ROADMAP.md`](../ROADMAP.md) 维护。带“首轮”字样的条目只表示
该里程碑已归档；若仍有维护或补强责任，主 roadmap 另有 `长期回归` / `进行中` 条目。

| 领域 | 已完成任务 | 专题证据 |
| --- | --- | --- |
| charter/model/coding/arceos_ex/mm/task | 统一用户 fault core 与普通 fork 独立 mm/eager dup_mm | [用户缺页、独立 mm、COW 与 SIGSEGV 执行边界](user-memory.md#已完成基线第一与第二阶段) |
| charter/model/coding/arceos_ex/trap/scheduler | TaskFlow 正式 trap overlay 与 page-fault leaf 内 A->B->A 返回首轮 | [确定性 SMP TaskFlow lanes 专题](deterministic-smp-lanes.md#已闭合taskflow-trap-overlay-与-leaf-内切换返回) |
| model/tools2/animate | tools2 Signal 交互式离线 HTML 动画 v1 | [交互式 model trace HTML 动画](interactive-model-animation.md) |
| model/tools2/validation | tools2 Deferred / Trimmed / Obligation v9 语义闭环 | [Signal-driven tools2 完成证据](signal-driven-tools2.md#deferred--trimmed--obligation-v9-完成证据) |
| tools2/testing/performance | tools2 主模型集成测试去重与阶段复用 | [Signal-driven tools2 完成证据](signal-driven-tools2.md#主模型集成测试时长优化完成证据) |
| model/tools/docs | Deferred / Trimmed 结构化治理与 legacy inventory 全量审计 | [Deferred / Trimmed 结构化治理审计](deferred-trimmed-audit.md) |
| validation/arceos_ex | 全量 Clippy 与 clean-build warning 收口 | [当前上下文归档](current-context.md) |
| validation/trace | nightly/压力缺陷复现流水线首轮建立 | [当前上下文归档](current-context.md) |
| checkpoint/arceos_ex | `LOG=trace` 迁移为 `PROBE=announce` | [当前上下文归档](current-context.md) |
| trace/user | `wait4 child handoff` 默认输出收口 | [用户态 payload 历史](user-mode.md) |
| docs/roadmap | `ROADMAP.md` 索引化首轮拆分 | [当前上下文归档](current-context.md) |
| linux/checkpoint/stress | Linux exact-mapped runtime 插桩与横向差分首轮 | [当前上下文归档](current-context.md) |
| model/coding/arceos_ex/virtio | virtio-rng 与 virtio 基础对象首轮闭环 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/virtio | virtio-blk 最小 read request 闭环 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/block | block device registry / major-minor 发现路径 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/block | block registry 公开读路径与 smoke 读验收 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/vfs | VFS / ramfs / 初始 rootfs mount 最小闭环 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/vfs | devfs 与 rootfs enable 前置闭环 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/block | bio / buffer_head 同步读路径 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/block/fs | read-only ext2 首轮路径 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/block/fs | 4K Buffer / 4K ext2 block 支持 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/block/fs | Ext2 对象建模与规格/实现收敛 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/block/fs | Ext2 read path 泛化 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/block/fs | Ext2 真实目录 path lookup | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/vfs/fs | Ext2 最小 VFS read-only mount/read | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/rootfs/fs | RootfsPhase 真实 ext2 root mount/root switch | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| model/coding/arceos_ex/payload/user | 用户态 payload 对象边界 | [用户态 payload 历史](user-mode.md) |
| build/rootfs/user | 用户 init ELF 与 ext2 镜像输入 | [用户态 payload 历史](user-mode.md) |
| build/rootfs/user | rootfs 构造期 overlay 机制 | [用户态 payload 历史](user-mode.md) |
| model/coding/arceos_ex/exec/mm/trap | 切换前用户地址空间与 trap frame 准备 | [用户态 payload 历史](user-mode.md) |
| model/coding/arceos_ex/exec/mm/trap | UserAddressSpace 真实页表与 satp-ready | [用户态 payload 历史](user-mode.md) |
| model/coding/arceos_ex/trap/syscall | U-mode entry 与最小 syscall | [用户态 payload 历史](user-mode.md) |
| validation/user | user-boot 验收闭环 | [用户态 payload 历史](user-mode.md) |
| model/coding/arceos_ex/payload/user | B 阶段对象边界收口 | [用户态 payload 历史](user-mode.md) |
| model/coding/arceos_ex/task/mm | PID 1 固定 Task/KernelInitFlow/UserAppRuntime 身份语义 | [用户态 payload 历史](user-mode.md) |
| model/coding/arceos_ex/syscall/vfs | 最小 fd/VFS syscall 层首片 | [用户态 payload 历史](user-mode.md) |
| model/coding/arceos_ex/syscall/vfs | read-only open/read/close/stat syscall 首片 | [用户态 payload 历史](user-mode.md) |
| model/coding/arceos_ex/elf/mm | 动态 libc 用户 init 前置条件 | [用户态 payload 历史](user-mode.md) |
| validation/user/tty | 交互式 `/bin/sh` 外部命令后 prompt-return 诊断 | [用户态 payload 历史](user-mode.md) |
| model/coding/arceos_ex/syscall/process | `clone(220)` / fork 首片 | [用户态 payload 历史](user-mode.md) |
| validation/user/smoke | `sh_probe` 综合 smoke 拆分 | [用户态 payload 历史](user-mode.md) |
| validation/user/process | `clone(220)` 首片 KUnit/smoke 回归 | [用户态 payload 历史](user-mode.md) |
| validation/user/init | BusyBox init getty/login shell pending-child job-control 验收 | [用户态 payload 历史](user-mode.md) |
| model/coding/arceos_ex/syscall/process | `/bin/sh` child `execve(221)` 首轮闭包 | [用户态 payload 历史](user-mode.md) |
| model/coding/arceos_ex/exec/mm/user | 用户栈 initial ABI、`AT_EXECFN` 与页粒度 ASLR 首片 | [用户态 payload 历史](user-mode.md) |
| arceos_ex/task/mm/arch | Per-task vmalloc 内核栈首轮 | [启动阶段审计历史](boot-audit.md) |
| model/coding/arceos_ex/trap/mm | RISC-V VMAP trap 栈 early overflow 首片 | [启动阶段审计历史](boot-audit.md#已完成专题risc-v-vmap-trap-栈-early-overflow) |
| charter/model/coding/arceos_ex | Kernel 阶段范式四层一致性审计与 coding 文档化 | [阶段范式审计](phase-paradigm-audit.md) |
| charter/model/coding/arceos_ex/phases/task | TaskFlow 直接子阶段归属重构 | [TaskFlow 直接子阶段归属重构](phase-paradigm-audit.md#taskflow-直接子阶段归属重构) |
| model/coding/arceos_ex/block/fs | Ext2 single-indirect read 支持 | [virtio / block / VFS / Ext2 历史](virtio-block-fs.md) |
| arceos_ex/irq | PLIC 驱动与 UART 外部中断链首轮 | [IRQ / console / TTY 历史](irq-console-tty.md) |
| arceos_ex/console | serial8250 interrupt-driven console TX | [IRQ / console / TTY 历史](irq-console-tty.md) |
| validation/stress | stress runner 集合化差分与内存输出首轮 | [当前上下文归档](current-context.md) |
| validation/stress/difftest | Composite basic-test v2 编排与集合化结果分析验收 | [Composite basic-test 独立验收归档](composite-basic-test-validation.md) |
| docs | 开发文档与真实进展同步 | [当前上下文归档](current-context.md) |
| model/arceos_ex | CurrentCPU 与 RawSpinLock 建模收敛 | [阶段范式审计](phase-paradigm-audit.md) |
| arceos_ex/allocator | KernelHeap/Allocator facade 动态容器前置 | [当前上下文归档](current-context.md) |
| arceos_ex/smoke | 内存分配 API 运行期 smoke | [当前上下文归档](current-context.md) |
| arceos_ex/platform | OF platform device population | [IRQ / console / TTY 历史](irq-console-tty.md) |
| arceos_ex/platform | PlatformBus driver/probe 与 ns16550a driver | [IRQ / console / TTY 历史](irq-console-tty.md) |
| arceos_ex/console | console/earlycon handoff | [IRQ / console / TTY 历史](irq-console-tty.md) |
| arceos_ex/console | handoff 后重复输出修复与 announce | [IRQ / console / TTY 历史](irq-console-tty.md) |
| arceos_ex/console | console 测试边界收口 | [IRQ / console / TTY 历史](irq-console-tty.md) |
| spec/console | EarlyCon / BootConsole / ConsoleRegistry 分层 | [IRQ / console / TTY 历史](irq-console-tty.md) |
| arceos_ex/mm | ioremap/vmalloc 运行时映射首轮 | [当前上下文归档](current-context.md) |
| arceos_ex/mm | MMIO 属性模型首轮 | [当前上下文归档](current-context.md) |
| arceos_ex/allocator | initcall 动态分配压力失败根因收口 | [当前上下文归档](current-context.md) |
