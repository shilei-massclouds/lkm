# Roadmap

本文是项目唯一的活跃任务、优先级和状态入口。已确认但未闭环的实现缺陷记录在
[`DEFECTS.md`](DEFECTS.md)；无剩余责任的完成项从本表移出，由
[`roadmap/completed.md`](roadmap/completed.md) 链接到保存完整证据的专题归档。

## 生命周期规则

- `当前` / `当前计划`：正在推进的最前沿边界。
- `进行中`：已有可复现进展，仍有明确剩余责任。
- `待办`：已确认范围但尚未进入当前执行窗口。
- `延期`：保留责任，等待更高优先级目标或触发证据。
- `长期回归`：首轮能力已建立，剩余责任是持续 ordinary-path 回归、维护或差分。
- 任务没有剩余动作后，从主表移入专题归档，并在 `completed.md` 增加领域、任务名和专题链接。

## 当前焦点

1. 缺陷处理继续遵循证据驱动：先复现并定位 checkpoint/diagnostic 边界，再更新规格与实现。
2. OpenRC getty/login shell 的 bounded pending-child `setpgid` / `TIOCSPGRP` 验收已闭环并归档；
   focused case 保持 opt-in，后续扩大任务图或 job-control 语义必须由新的可复现证据触发。
3. ordinary shell、nightly stress 和 Linux paired difftest 保持长期回归；失败样本按稳定序列和
   source-scoped 事实分类。
4. VFS/pathname、exec lifecycle、vmalloc user trap stack 与文件系统缓存层仍按各自 active row 推进。
5. coding 层的 model-object 文件覆盖、实现证据和 rootfs 测试编排分别由
   [`objects/README.md`](../spec/coding/objects/README.md)、
   [`arceos_ex-implementation.md`](../spec/coding/arceos_ex-implementation.md) 与
   [`testing/rootfs.md`](../spec/testing/rootfs.md) 承载。

## 专题与完成归档

- [完成项中央索引](roadmap/completed.md)
- [当前上下文、验证、构建、工具和文档历史](roadmap/current-context.md)
- [启动阶段审计与同步/上下文历史](roadmap/boot-audit.md)
- [阶段范式四层一致性审计](roadmap/phase-paradigm-audit.md)
- [Linux PLIC object 复用历史](roadmap/linux-plic.md)
- [virtio / block / VFS / Ext2 历史](roadmap/virtio-block-fs.md)
- [用户态 payload、syscall 与 process 历史](roadmap/user-mode.md)
- [IRQ / console / serial8250 / TTY 历史](roadmap/irq-console-tty.md)

## 活跃任务

| 优先级 | 状态 | 领域 | 任务 | 剩余责任 | 细节 |
| --- | --- | --- | --- | --- | --- |
| `P0` | 长期回归 | validation/trace | ordinary-path nightly/压力缺陷复现 | 持续运行 DF-0001/DF-0002/DF-0003 ordinary cases 与 `stress-mem`；出现失败类后按稳定序列和 source-scoped facts 定位，不以 probe 路径替代 ordinary path。 | [缺陷记录](DEFECTS.md)；[测试规格](../spec/testing/rootfs.md) |
| `P0` | 当前 | validation/trace/arceos_ex | 纵向差分定位 DEFECTS 问题 | 用 stress 报告、成功/失败序列、failure diagnostic 和 `stress-mem` 产物定位新失败；没有失败类集合时只记录回归，不增一次性 checkpoint。 | [缺陷记录](DEFECTS.md) |
| `P0` | 进行中 | trace/arceos_ex | 当前 trace 诊断能力评估与缺口补强 | 只补仍影响定位的最小长期观察点；Linux-like trace 由现有证据不足触发。 | [pyveri DEVELOPMENT](../tools/pyveri/DEVELOPMENT.md#step-c1-收口-trace-输出和注释数据流) |
| `P0` | 长期回归 | linux/checkpoint/stress | Linux exact-mapped runtime 插桩与横向差分 | 维护 exact marker 与默认 rc.local hard scope；继续处理 `range`/`unmapped` mapping，并在需要时规格化先纵向、后横向的 multi-run 语义。 | [charter](../spec/charter/main.md#linux-runtime-checkpoint-插桩与横向差分)；[测试规格](../spec/testing/rootfs.md#rclocal-and-paired-difftest) |
| `P0` | 当前 | model/coding/arceos_ex/vfs | VFS pathname walk / rootfs path read 补强 | 推进 slow symlink、通用 nofollow、magic link、RCU walk、权限、mount namespace、fd table 和 errno 边界；保持当前 read-only fast-symlink 骨架。 | [VFS coding](../spec/coding/objects/vfs.md) |
| `P0` | 进行中 | build/rootfs/user/validation | 发行版 rootfs smoke 与 overlay 分层 | 保持 overlay fixture 与 bare distro cases 分层；继续以真实 OpenRC/发行版路径收敛 native init 验收，不用测试专用内核 API。 | [镜像构造](../spec/coding/projects/rootfs-image.md)；[测试规格](../spec/testing/rootfs.md) |
| `P0` | 进行中 | model/coding/arceos_ex/syscall/process | 完整 exec lifecycle 补强 | 补旧 mm/stack/page-table 回收、失败回滚、重复 staging reset、close-on-exec、credentials/signal/binfmt 与 point-of-no-return 语义。 | [user boot coding](../spec/coding/objects/user-boot.md#clone-wait-and-exec) |
| `P0` | 当前 | model/coding/arceos_ex/trap/mm | Linux-aligned 用户 trap 栈与 VMAP guard | 补不会破坏用户寄存器的 early overflow scratch/bit-test；之后再展开 per-task vmalloc stack 与 IRQ hardirq stack。 | [user boot coding](../spec/coding/objects/user-boot.md#trap-and-exception-mapping) |
| `P0` | 进行中 | arceos_ex/task/mm/arch | vmalloc task stack 后续调度语义 | 补完整 kthreadd 请求消费、通用 scheduler class/fairness 与更多 task-return/switch 语义；保持 BootIdle/KernelInit owner 边界。 | [rest_init coding](../spec/coding/phases/up-multitask/rest-init.md) |
| `P0` | 待办 | arceos_ex/fs/vfs/smoke | 目录操作 smoke 补强 | 在 directory-capable openat/getdents64/fd offset/close 规格闭合后，覆盖生产 VFS/Ext2 路径的目录迭代和错误分类。 | [测试规格](../spec/testing/README.md) |
| `P1` | 待办 | model/coding/arceos_ex/rootfs/fs | root switch 后命名空间补强 | 明确 ext2 root 下 `/dev` 挂接、mount namespace 与任务 FsStruct 的运行期规则。 | [RootfsPhase coding](../spec/coding/phases/smp-runtime/rootfs.md) |
| `P1` | 待办 | model/coding/arceos_ex/vfs/fs | Ext2 VFS inode/dentry cache 边界 | 对齐 iget/dentry cache、negative lookup、inode identity、refcount 与 evict 边界。 | [Ext2 coding](../spec/coding/objects/ext2.md)；[VFS coding](../spec/coding/objects/vfs.md) |
| `P1` | 待办 | model/coding/arceos_ex/mm/fs | page cache / address_space / folio read path | 规格化 AddressSpace/page cache/folio/readahead，让 VFS read 从直接 block copy 迁移；writeback/mmap 后置。 | [VFS coding](../spec/coding/objects/vfs.md) |
| `P1` | 待办 | model/coding/arceos_ex/syscall/mm/signal | 动态用户栈 rlimit 与越界信号 | 实现 `prlimit64/setrlimit`、动态 `RLIMIT_STACK`，并把栈越界转换为 Linux-like `SIGSEGV/si_code`。 | [UserStack charter](../spec/charter/objects/user-stack.md) |
| `P1` | 待办 | model/coding/arceos_ex/mm/task | 通用用户栈 VMA、COW 与多线程 | 展开 fork/COW、多个线程栈、`MAP_STACK/MAP_GROWSDOWN`、通用 VMA fault core 与并发锁。 | [UserStack charter](../spec/charter/objects/user-stack.md) |
| `P2` | 延期 | model/coding/arceos_ex/random/hardening | 用户/内核栈随机与 protector 强化 | 在完整 CRNG 基础上补内核 compiler stack protector、per-task canary 与更强栈保护；RISC-V 原生更大 ASLR 窗口随地址布局扩展再评估。 | [UserStack coding](../spec/coding/objects/user-stack.md) |
| `P2` | 延期 | compose/arceos_ex | 对象封装为组件试验 | 选择稳定对象后再明确 crate/component 边界、接口与验收方式。 | [compose 规格](../spec/compose/README.md) |
| `P1` | 待办 | arceos_ex/console | printk TX 异常/压力边界 | 有 nightly/差分证据后再规格化 queue full、drop/truncate、hardirq/reentrant printk 和 handler 内 printk 策略。 | [InitcallPhase coding](../spec/coding/phases/smp-runtime/initcall.md) |
| `P1` | 待办 | arceos_ex/console | serial8250 runtime 与 TTY 分层 | 收敛 RuntimePort/Console/TtyPort/FlipBuffer/XmitFifo 职责与 ordinary I/O runtime 边界。 | [InitcallPhase coding](../spec/coding/phases/smp-runtime/initcall.md) |
| `P2` | 待办 | arceos_ex/console | 用户态标准输入输出机制 | 在 console/TTY runtime 稳定后展开 file backend、N_TTY、stdin/stdout、poll 和 pipes。 | [user boot coding](../spec/coding/objects/user-boot.md) |
| `P1` | 待办 | trace/view | 收口 trace/SVG 输出体验 | 改善 depends_on 长线、图高、标签、事实展示和 action 展开深度。 | [pyveri DEVELOPMENT](../tools/pyveri/DEVELOPMENT.md#step-c1-收口-trace-输出和注释数据流) |
| `P1` | 待办 | trace/view | 优化 trace context 框显示 | 优化 context 高度、文本锚定、跨行标签和视觉层级。 | [pyveri DEVELOPMENT](../tools/pyveri/DEVELOPMENT.md#view) |
| `P1` | 进行中 | validation/stress | stress runner 集合化差分报告 | 将报告升级为成功集合、失败类别集合与跨集合特征汇总；重复序列只计数，完整样本按稳定主键去重。 | [charter](../spec/charter/main.md#nightly压力测试与纵向差分) |
| `P2` | 待办 | validation/stress/arceos_ex | `stress-mem` 共享内存后端 | 先用 host-backed shared memory 与 FDT 描述固定 ABI；doorbell/IRQ/专用设备后置。 | 本文档 |
| `P2` | 待办 | arceos_ex/checkpoint | 清除 `LOG=trace` 兼容入口 | README、脚本、stress case 与历史命令迁完后删除 alias，把 trace 名称留给 Linux-like trace。 | [checkpoint mapping](../spec/coding/mapping.md) |
| `P1` | 待办 | CI | 建立 GitHub Actions 快速 CI | 覆盖工具质量、核心推导、trace smoke、顶层 verify 与最小构建，不跑耗时 QEMU 全量任务。 | [构建规格](../spec/coding/build.md) |
| `P1` | 待办 | pyveri | 默认 target 与 rule-only 检查 | 增加规格默认 target 和只执行 parse/model/rule 的 formal rule-only 模式。 | [pyveri DEVELOPMENT](../tools/pyveri/DEVELOPMENT.md#工具链架构目标) |
| `P1` | 待办 | pyveri | 注释数据流下沉 | 由 parse 保留注释，model/view 建立关联，render 只消费结构化输入。 | [pyveri DEVELOPMENT](../tools/pyveri/DEVELOPMENT.md#step-c1-收口-trace-输出和注释数据流) |
| `P1` | 待办 | model/docs | 补齐 Linux 裁剪配置的 model deferred | 把实现审计清单逐项迁入对应 model/coding 正式入口；展开实现属于后续独立任务。 | [deferred 审计清单](../spec/coding/arceos_ex-implementation.md#待迁移到-model-的-deferred-审计清单) |
| `P1` | 进行中 | model/semantics | 正式规格化上下文和嵌套检查 | 补系统天然独占来源证明、RCU 读侧、handle_level 与更多 guard kind。 | [model semantics](../spec/model/SEMANTICS.md#sem-context-nesting-001-context-effects-compose-monotonically) |
| `P2` | 延期 | model/derive | 多链独立推导语义 | 为 task/interrupt flow 建立独立推导链，并保持跨链状态交互显式。 | [model semantics](../spec/model/SEMANTICS.md#sem-transition-emits-001-completion-events-are-post-commit-events) |
| `P1` | 待办 | model/arceos_ex | 抽取 wake_up_new_task 复用模型 | 统一 task wake-up context、runtime state、runqueue selection 与 nested enqueue 约束。 | [model semantics](../spec/model/SEMANTICS.md#sem-exclusive-context-001-guard-and-resource-exclusive-context-are-distinct) |
| `P1` | 待办 | model/arceos_ex | EventStream per-CPU 归属 | 为每个 live CPU 建立自己的 Event/Interrupt/Exception stream；AP 在 secondary entry 后建立。 | [model semantics](../spec/model/SEMANTICS.md#sem-current-cpu-model-001-currentcpu-is-the-per-cpu-self-identity-entry) |
| `P1` | 进行中 | arceos_ex | 整理对象级源码结构 | 继续按 phase/object mapping 拆分源码，并把资源对象收敛到 Context；目录主题化另列 P2。 | [object coverage](../spec/coding/objects/README.md) |
| `P1` | 待办 | arceos_ex/console | serial8250 RX/TTY/FIFO 补强 | 依据复现/差分证据展开 file/stdin/stdout、line discipline 和更完整并发策略。 | [InitcallPhase coding](../spec/coding/phases/smp-runtime/initcall.md) |
| `P1` | 待办 | arceos_ex/smoke | 类型行为 smoke 双任务场景 | 为 RawSpinLock、Completion 等补最小双任务竞争/等待/唤醒与跨任务可见性。 | [testing 规格](../spec/testing/README.md#测试目标分类) |
| `P1` | 待办 | arceos_ex/codegen | RISC-V64 linker script 生成方案 | 收敛为 `.lds.S` + generated config header，并统一 Rust/linker/codegen 配置来源。 | [构建规格](../spec/coding/build.md) |
| `P1` | 待办 | CI/Homepage | 发布 nightly/manual 结果与主页 | 在本地流水线稳定后发布 trace、测试日志、对象覆盖和推导摘要。 | [构建规格](../spec/coding/build.md) |
| `P2` | 待办 | model | 继续语义扩展 | 在当前闭环稳定后选择 deferred、AP 细节或 Payload/user 扩展。 | [model semantics](../spec/model/SEMANTICS.md) |
| `P2` | 待办 | arceos_ex | `objects/` 目录分层 | 对象语义和命名稳定后再按 model/storage/primitives/主题目录化。 | [object coverage](../spec/coding/objects/README.md) |
| `P2` | 进行中 | pyveri | 工具链拆分与中间协议 | 细化 schema、退出码、缓存/增量策略并保持独立阶段不反向依赖 pyveri 包。 | [pyveri DEVELOPMENT](../tools/pyveri/DEVELOPMENT.md#step-d-工具链拆分) |
| `P2` | 延期 | compose | 组件封装阶段 | 恢复 ArceOS 组件接口、crate、ax-std、workspace、xtask、feature 与 facade。 | [compose 规格](../spec/compose/README.md) |

## 更新要求

行为、接口、对象边界或 Linux differential 语义变化先更新 applicable spec/coding/testing，再实现。
完成项在同一变更中移出本表、把证据写入唯一专题，并更新 `completed.md`；专题不得维护第二套活跃
优先级表。
