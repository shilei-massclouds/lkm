# RISC-V64 Coding 规格

本文记录把当前模型规格落到 RISC-V64 内核实现时需要遵守的补充约束。

## 适用范围

本文件补充 `spec/model` 中与 RISC-V64 启动、CSR、页表、SBI、FDT 和中断异常入口相关的编码约束。它不重新定义对象生命周期，也不改变 `Preset`、`Setup`、`Enable`、`Cleanup` 的状态迁移语义。

本文是 RISC-V64 coding 层的权威约束来源；对象与生命周期语义仍以 `spec/model` 为准。

第一轮实现只支持 RISC-V64，不要求提供其它架构的兼容层或空实现。

## 启动入口

- 内核入口应显式接收并保存启动 ABI 传入的 `a0` 和 `a1`；可变寄存器在模型中属于
  `BootCPU.BootCpuRegisters`，其寄存器文件由硬件天然提供而不是由工程或平台生命周期构造。
- OpenSBI 交接时 `BootCpuRegisters.a0` 必须等于初态已存在的 `BootArgs.boot_hartid`。
- 入口汇编必须把 `a0` 的入口值写入后续阶段仍可读的持久存储位置或变量。实现可先
  把 `a0` 复制到未被后续入口代码改写的临时寄存器，再保存该副本。这一存储位置只是交接载体，
  不要求汇编入口寻址或直接写入 `Cpu` 对象中的 `hartid`。
- OpenSBI 交接时 `BootCpuRegisters.a1` 必须等于初态已存在的 `BootArgs.dtb_pa`。
- OpenSBI 只确定上述两个交接寄存器；`sp/tp/gp` 和 supervisor CSR 继续由真实入口汇编按阶段更新。
- BootCPU 的 FS/VS 执行状态及其内核态受控使用策略归 CPU 规格所有；入口 lowering 见
  [`objects/cpu.md`](objects/cpu.md)。`BootInitFlow` 只负责编排，`CpuGroup` 只负责 CPU identity 关系。
- 入口代码不得跳过 `BootArgs` 抽象直接让后续对象长期依赖裸寄存器值。
- `Riscv64` 只表示外部 ISA 能力，不作为 live GPR/CSR 容器。
- 链接脚本必须显式提供 `__global_pointer$`，并保证 `_start` 同时是内核 text 起点和 ELF entry。

## 入口前导期地址访问纪律

RISC-V64 入口前导期实现必须按地址空间阶段区分可执行代码位置和可访问数据范围。

1. MMU 关闭阶段：
   - 入口最前段代码应集中放在内核映像起始处的 boot/head text 区域。实现可采用 `.head.text`、`.text.boot` 或其它等价段名，但链接脚本必须保证该区域紧接 `_start`，且位于固件跳入后可直接取指的物理映像范围内。
   - 该阶段访问内核符号必须使用 PC-relative 或等价 position-independent 方式。若使用 Rust/C 函数，必须确保对应编译选项和代码形态不会生成依赖最终虚拟地址的绝对寻址。
   - 该阶段不得生成或执行依赖最终虚拟地址的间接控制流。普通对象方法、公共状态机、match/switch 分发、trait/vtable 调用、函数指针表、跳转表、panic 路径和编译器自动生成的分发表都必须满足当前物理地址可执行性；无法证明时，必须改用显式分支、head-safe 汇编或其它可验证的物理地址安全实现。
   - 该阶段不得访问普通 `.data/.bss/.init.data`，除非该访问已经被证明使用当前物理地址语义且不依赖尚未启用的虚拟映射。
   - 该阶段函数不得引入 ftrace、sanitizer、coverage、stack protector 或其它可能访问普通数据段或运行时设施的 instrumentation，除非逐项证明其访问路径满足本阶段约束。
   - 实现应把 pre-MMU 路径纳入反汇编或等价构建检查范围，重点检查是否出现跳转到高虚拟地址、从普通 `.rodata` 读取代码目标、或其它无法在 MMU 关闭阶段执行的间接分支。
2. trampoline 临时映射阶段：
   - 切换到 trampoline 页表后，只允许执行 trampoline 映射覆盖的代码，且代码应尽快切换到 `EarlyVm` 页表。
   - 该阶段不得访问普通 `.data/.bss/.init.data`，不得调用可能访问普通数据段、锁、日志、allocator 或未映射静态对象的函数。
   - checkpoint hook 若位于该阶段，必须是极小的 head/trampoline-safe 实现。
3. EarlyVm 阶段：
   - `EarlyVm` 必须至少映射整个 `KernelImage`，包括入口后续需要访问的 `.text/.rodata/.data/.bss/.init.data` 等内核映像范围。
   - 地址转换切换不得破坏 `gp_relative_addressing_ready(KernelImage)`；若原有寻址基准会失效，必须按
     [`objects/kernel-image.md`](objects/kernel-image.md) 在下一次 `gp` 相对访问前重新建立。
   - 只有在该阶段之后，入口前导期代码才可以按普通早期虚拟地址访问内核静态数据。

该约束参考 Linux RISC-V64 的 `__HEAD`/`HEAD_TEXT_SECTION`、`setup_vm()` 的 `medany` 要求，以及 trampoline 到 early page table 的两次 `satp` 切换流程；`arceos_ex` 不要求复制 Linux 宏名，但必须提供等价的链接布局和访问纪律。

## 链接脚本参考建议

- 生成或维护的 RISC-V64 `.lds` 文件不得依赖固定的内核物理加载地址，也不得定义 `KERNEL_PHYS_ADDR` 这类 Config 常量。内核链接虚拟地址来自 `Config.kernel_link_addr`；实际物理装载起点由入口前导期 `KernelImage.Preset` 根据运行时映像位置确认，并作为 `KernelImage.phys_start` 一类对象事实供后续地址转换使用。
- `.lds` 仍可依赖 `Lds` 规格和其它 `Config` 值，例如页大小、PMD 大小、段对齐、head text 布局、boot stack size 和相关符号位置。
- `__global_pointer$` 的存在、初始化方式和实际引用覆盖证明属于
  [`objects/kernel-image.md`](objects/kernel-image.md) 的强制约束；本文件不规定它必须位于两个特定
  section 之间。

## CSR 与屏障

- CSR 访问应集中封装，调用点表达具体语义，例如关闭中断、设置 `stvec`、切换 `satp`。
- 写入 `stvec` 的入口地址必须满足 RISC-V64 `stvec` 对齐和模式编码要求。直接模式下入口 base 至少 4 字节对齐，低位不得被误用为 mode；trampoline 或 early trap 入口标签必须在汇编或链接布局中显式保证对齐，并按当前地址空间阶段写入物理地址或虚拟地址。
- `TrapType.Preset` 写入 `stvec` 的目标必须是汇编定义的临时保护入口；该入口只以无条件分支回跳
  自身，形成没有其它指令或副作用的空无限循环。
- `TrapType.Setup` 必须在 `ExceptionType` 及四个子类型的 Preset fallback 已经建立后，把 `stvec` 写为
  对齐的正式响应汇编函数入口地址；该入口承载 fresh `TrapFlowType` 的建立和绑定。不得把 `satp` 值、
  页表地址、Rust wrapper 或数据对象写入 `stvec`。紧接该 `stvec` 写入必须以 `csrw sscratch, zero`
  标记当前正在内核态执行；不得把 CPU 的 `TrapEntryContext` 地址常驻在 `sscratch` 中。
- 页表切换相关代码必须显式处理 `sfence.vma` 要求。模型规格可以不逐条展开该细节，但实现规格要求保留该边界。
- `SwapperVm.Action::ActivateOnCpu(cpu_ref)` 对应 Linux/RISC-V `setup_vm_final()` 中写入 `swapper_pg_dir`
  SATP 后的 `local_flush_tlb_all()` 边界。实现必须在写入 swapper SATP 后执行本地 TLB/地址转换同步，
  并把完成结果暴露为 `swapper_vm_translation_sync_complete(SwapperVm, cpu)`；不得只把屏障指令散落在
  代码中而不纳入 per-CPU activation 事实。
- 早期入口对中断 pending/enable 状态的防御性清理应对应 `InterruptType.Preset` 或 `InterruptType.Setup` 的实现边界。

## 当前任务引用

模型层的 `CurrentTask` 是 CPU 执行上下文已提交的 task binding，`CurrentTaskRef` 从该 Task 的唯一
generation-checked 引用派生；两者都没有 object、槽或 lifecycle。RISC-V64 lowering 参考 Linux，用本
hart 的 `tp/x4` 承载这一 binding。通用 `CurrentTask.Action::BindTask(task)` 只允许由调度切换 commit
调用，并且必须由汇编把 task 当时可用的 canonical 地址写入 `tp`；该 Action 不写 `sp`，也不提交
CurrentStack。架构 switch 在同一个外层 commit 中独立从 next TaskThreadContext 恢复 `sp`，再以
`next.stack` 校验并发布 CurrentStack。内核入口、AP HSM entry、真实及模拟调度提交都必须在任何 Rust
Context、scheduler 或 CurrentTask 路径前建立正确的 `tp`；identity switch 保持原值，普通/terminal
switch 在正式 switch commit 指向 next Task。入口调用位置对应 Linux `la tp, init_task`，调度路径对应
`__switch_to` 的 `move tp, a1`。集中解析器必须把该地址验证为 canonical Task/TaskRef，
未知地址、错误 storage class 或 stale generation 都显式失败，不回退到 BootTask、runqueue curr 或缓存。

BootTask 入口不调用通用 `BindTask`。`BindTaskStack(BootTask, BootTask.stack)` 和
`RefreshTaskStack(BootTask, BootTask.stack)` 必须各自 lowering 为一个连续的 RISC-V 汇编提交块：前者在
PhysicalDirect 下使用 Linux 对应的 `la tp, init_task`、`la sp, init_thread_union + THREAD_SIZE` 与
`addi sp, sp, -PT_SIZE_ON_STACK` 建立物理 task/stack pair；后者在 EarlyVm 接管后以相同符号重新装载
虚拟 `tp/sp`。两个块都直接内联在 Linux `_start_kernel` 风格的入口汇编路径中，完成后顺序执行后续
入口代码；不得为任一 Action 发明专用 continuation 或额外 tail。Linux 中最终的 `tail start_kernel`
位于 `soc_early_init` 与 BootInitFlow Prepared 边界之后，不属于 RefreshTaskStack。外围
`SingleTaskContext` / `SystemExclusive` 保证 Action 的语义原子性，不要求硬件多寄存器原子写。所有可
失败校验必须在进入提交块前完成；两次寄存器写之间不得调用会观察 current pair 的 C/Rust、不得插入
可失败分支或暴露 checkpoint。Action 成功后才发布 CPU-local task/stack bindings；失败保持 bindings
与 `tp/sp` 不变。两种 Action 不初始化或修改 preempt count，也不得导出独立 `BindStack` API。

Linux 6.12 的参考路径是：`kernel/sched/core.c::__schedule()` 调用 `switch_to(prev, next, prev)`，RISC-V 宏 `arch/riscv/include/asm/switch_to.h::switch_to` 最终调用 `arch/riscv/kernel/entry.S::__switch_to`；`__switch_to` 保存 `prev->thread`、恢复 `next->thread` 后执行 `move tp, a1`，其中 `a1` 是 next `task_struct`。`arch/riscv/include/asm/current.h` 将 `current` 绑定为 `tp` 上的 `struct task_struct *`。

per-cpu 存储可以作为 contextual binding 的验证快照承载方式，但不得把它变成 CurrentTaskSlot、
CurrentStackSlot、生命周期对象或全局单例；CurrentTask/CurrentStack 的体系结构执行事实仍分别由本
hart 的 live `tp/sp` 与 Task.stack 约束。
Linux PLIC shim 对 `tp` 的临时占用只属于 foreign ABI；正常、错误和嵌套返回路径都必须恢复调用前的
真实 task `tp`，且在恢复前不得进入 Rust Context、调度或 CurrentTask 解析。

## 地址空间与页表

各系统的权威 lowering 分别位于 [`Vm`](objects/vm.md)、
[`PhysicalDirect`](objects/physical-direct.md)、[`TrampolineVm`](objects/trampoline-vm.md)、
[`EarlyVm`](objects/early-vm.md)、[`SwapperVm`](objects/swapper-vm.md)、
[`KernelAddrSpace`](objects/kernel-address-space.md)、[`FixMap`](objects/fix-map.md)、
[`LinearMap`](objects/linear-map.md)、[`UserSpaceReserve`](objects/user-space-reserve.md) 与
[`RawDtb`](objects/raw-dtb.md)。本文件只保留跨系统的 RISC-V 地址访问纪律与 CSR 规则，不把这些系统
重新合并成一个 Coding 边界。

## FDT 与物理内存

- `RawDtb` 的物理地址、头部范围和完整范围应在进入后续解析前被记录并检查。
- OpenSBI 固件提供 RawDtb 已完整加载到 S-mode 可访问内存中的交付保证；实现仍应在 `RawDtb` 推进过程中逐步确认 header、magic、totalsize 和完整范围。
- `PhysicalMemory` 和 `PlatformCpuInfo` 不是准备期事实。它们来自 `RawDtb`，由入口后继期 `EarlyDtb.Preset` 解析 FDT `/memory` 与 `/cpus` 后建立并发布。
- `EarlyDtb` 只表示入口后继期短暂存在的早期解析服务，不能被实现为后续正式 DeviceTree 对象的无边界延续。实现中应区分 `EarlyDtb.Preset` 的基础平台事实抽取和 `EarlyDtb.Setup` 的命令行、MemBlock 候选区段等后续解析用途。
- `PhysicalMemory`、`PlatformCpuInfo`、`MemBlock` 第一轮应优先由实际 FDT 解析建立。若解析能力不足，应停止并报告缺口，不得静默回退到 QEMU virt 固定内存范围。

## 多核与内存模型

- 当前 RISC-V64 generic 平台按 UMA 架构建模，所有 CPU/hart 共享同一物理内存地址空间。
- 将来扩展多核时，应基于 SMP 架构推进 `CPUGroup`、secondary CPU、IPI、timer 和调度相关对象；不得引入 NUMA
  节点、本地内存距离或 per-node allocator 语义，除非模型规格先显式扩展。
- FDT 中解析出的多个 hart 只表示 SMP CPU 拓扑事实，不改变 `PhysicalMemory` 的 UMA 语义。

## SBI

- SBI 能力探测应集中形成 `SBI` 能力视图。
- `SBI` 对象负责记录能力事实，不把每一次具体 SBI 调用都建模为自身生命周期 transition。
- Early console、timer、IPI、rfence 等对象应依赖 SBI 能力事实，而不是各自重复探测固件能力。
- SMP AP 启动必须使用 OpenSBI ordered booting + SBI HSM hart_start 语义。BP 侧为每个 secondary CPU 发布
  Linux-like `sbi_hart_boot_data { task_ptr, stack_ptr }`，其中 `task_ptr` 指向该 AP 自己的 idle task，
  `stack_ptr` 指向该 AP task 的 pt_regs/栈顶边界；随后以 `secondary_start_sbi` 作为 AP 汇编入口调用
  HSM `hart_start(hartid, entry_pa, boot_data_pa)`。实现不得把 BP `_start` 复用于 AP，也不得在当前规格下
  新增 spinwait booting fallback。
- AP 入口路径应对齐 Linux RISC-V `secondary_start_sbi -> smp_callin -> cpu_startup_entry` 的阶段划分：
  AP 先消费 HSM boot data 并建立自己的 current task/stack，再在 `smp_callin()` 产生 `cpu_running`，
  最后进入 `CPUHP_AP_ONLINE_IDLE` 产生 `done_up`。这些边界必须能通过 checkpoint 定位。
- 第一轮 `EarlyCon` 使用 SBI early console 后端，不继承 ArceOS RISC-V64 QEMU virt 当前的 NS16550 UART console 路径。

## Checkpoint Announce

- 状态一致点在实现中应映射为 checkpoint hook，默认实现为空。
- checkpoint announce 是独立路径，不属于 `EarlyCon` 或正式 `Console`，不得依赖 allocator、锁、字符串缓冲区、FixMap 或线性映射已经可用。
- RISC-V64 第一轮可提供 SBI legacy putchar 单字符后端，用于 `EarlyVm` 切换生效前的极早期定位；该后端只输出稳定 checkpoint id 对应的一个字节。
- `EarlyVm` 切换生效、完整 `KernelImage` 映射可访问后，checkpoint announce 应切换为稳定 checkpoint 名称字符串，不继续扩展单字符 id 空间。
- 单字符 announce 只用于调试和状态差分采集的最低层观测，不改变对象状态，也不能作为规格事件或状态迁移的组成部分。
- 页表切换前后的 checkpoint hook 必须显式考虑当前代码地址是否已被正在使用的页表覆盖；必要时使用极小的 inline/boot text 实现。

## RISC-V64 Generic 平台

`arceos_ex` 使用新的 RISC-V64 generic SBI/FDT 平台实现，而不是复用现有 `riscv64-qemu-virt` 平台实现。

该平台的定位是：

- 当前以 QEMU virt 作为前期测试目标。
- 长期面向遵守启动 ABI、SBI 和 FDT 描述约束的 RISC-V64 平台。
- boot hart、DTB 地址来自启动 ABI。
- CPU 描述、物理内存、reserved-memory、chosen bootargs 等事实来自 FDT。
- timer、IPI、HSM、shutdown、early console 等能力来自 SBI 能力视图。
- 不把 QEMU virt 的 UART、PLIC、固定 RAM range 或 MMIO range 作为默认语义硬编码；需要使用时应来自 FDT 或明确的配置事实。

## 待补充

- 具体页表模式和地址布局常量。
- RISC-V64 链接脚本符号列表。
- QEMU virt 与真实硬件之间的差异处理。

## Rule catalog

以下规则是本文件的权威约束。迁移自旧索引的稳定 rule ID、原 type 分组和 MUST/SHOULD 层级在此保留；这些 ID 用于评审和追踪，不是 pyveri predicate。

### Riscv64LinkerScriptMust

#### No fixed physical kernel load address

Rule ID: `riscv64_must_linker_script_ignore_fixed_kernel_phys_addr` (MUST).

RISC-V64 linker script generation must not define, require or
consume a fixed kernel physical load address such as
KERNEL_PHYS_ADDR. The physical load address is not a Config fact.

#### Link address source

Rule ID: `riscv64_must_linker_script_use_config_kernel_link_addr` (MUST).

The kernel link virtual address used by the linker script must come
from Config.kernel_link_addr. The linker script may also depend on
model Lds facts and other Config facts such as page size, PMD size,
boot-stack size, section alignment and head-text layout.

#### Runtime physical start

Rule ID: `riscv64_must_kernel_phys_start_from_kernel_image` (MUST).

The kernel physical image start is a runtime fact established by
KernelImage.Preset from the actual entry/image position observed in
the pre-MMU stage. It must be represented as KernelImage.phys_start
or an equivalent model-backed resource fact.

#### Address translation source

Rule ID: `riscv64_must_address_translation_use_kernel_image_offset` (MUST).

Runtime physical/link/virtual address conversion must derive its
offset from KernelImage.phys_start and Config.kernel_link_addr. It
must not use a fixed Config.kernel_phys_addr-style constant.

### Riscv64AddressTranslationMust

地址转换的稳定 RISC-V rule ID 和逐 controller 提交顺序由上述各对象 Coding 文件独立拥有；跨系统
只要求 BP/AP activation journal 属于同一 CPU，且任一 Rust 完成接口不得重写汇编已经提交的 receipt。

### Riscv64SchedulerCodingShould

#### Current task reference

Rule ID: `riscv64_should_current_task_ref_follow_linux_tp` (SHOULD).

RISC-V64 code should realize CurrentTask's stable implementation identity by
following the Linux-style use of the tp register as the current-task
view. There is no object-level current-task slot; the implementation
boundary is a centralized, generation-checking tp-to-TaskRef resolver.
Every successful `BindTask` must write the task's current usable address to
`tp/x4`; an identical target refreshes that address without allocating another
task or binding identity. This follows Linux 6.12
arch/riscv/kernel/entry.S::__switch_to, which moves next
task_struct from a1 into tp. This is an implementation reference for
this target; the model semantics remain a CPU-local contextual binding and do
not create an object-level current-task slot.

For a user-origin trap, the architecture entry context at the safe kernel
stack boundary must preserve user `tp`, establish the receiving Task pointer
in kernel `tp` before calling any Rust event/CurrentTask path, and restore the
saved user `tp` only in the final return epilogue. The epilogue must publish
the post-dispatch kernel `tp` back to that entry context before restoring user
state; this makes terminal and non-terminal dispatch select the Task committed
by the scheduler rather than the Task that originally trapped.
