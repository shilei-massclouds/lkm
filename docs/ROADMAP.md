# Roadmap

本文是项目唯一的优先级和任务状态入口。其它文档可以保留设计背景、约束、命令和专题方案，但不再维护独立的 P0/P1/P2 计划表。

## 优先级规则

- `P0`：当前最应优先推进，直接影响日常审阅或当前闭环质量。
- `P1`：近期应推进，通常依赖 `P0` 的结果或与其紧密相邻。
- `P2`：后续扩展项，应在当前闭环稳定后推进。

## 当前焦点

1. 收口 trace/SVG 输出体验，使推导过程图适合日常审阅。
2. 建立快速 CI，保护推导工具、核心规格和 `impl/arceos_ex` 最小构建。
3. 同步开发文档，把当前真实状态收敛到本 roadmap 和专题说明中。

## 统一计划

| 优先级 | 状态 | 领域 | 任务 | 目标与说明 | 细节 |
| --- | --- | --- | --- | --- | --- |
| `P0` | 待办 | trace/view | 收口 trace/SVG 输出体验 | 系统检查基础图、state 注释图、event 注释图、state+event 注释图四种输出；优先处理 `depends_on` 长线、完整图过高、标签简化/分行、关键非状态谓词事实展示，以及布局常量是否暴露为 render 参数等问题，使 trace 图适合日常审阅。 | [pyveri DEVELOPMENT](../tools/pyveri/DEVELOPMENT.md#step-c1-收口-trace-输出和注释数据流) |
| `P1` | 待办 | CI | 建立 GitHub Actions 快速 CI | 覆盖推导工具质量、核心规格推导、trace 生成 smoke、顶层 `make verify` 和 `impl/arceos_ex` 最小构建；不发布 Pages，不跑耗时 QEMU 全量任务。 | [arceos_ex 说明](../spec/coding/arceos_ex.md#ci-与项目主页) |
| `P1` | 待办 | pyveri | 规格内默认 target 与 rule-only 检查模式 | 支持在规格元信息中声明默认推导目标；命令行 `--target` 优先，其次规格元信息，缺失时 derive/check/trace 要求显式 target。为 `spec/coding/main.spec`、`spec/compose/main.spec` 等 rule-only 规格提供只执行 parse/model/rule 检查的模式。 | [pyveri DEVELOPMENT](../tools/pyveri/DEVELOPMENT.md#工具链架构目标) |
| `P1` | 待办 | pyveri | 注释数据流下沉 | 让 `parse` 保留注释 span/内容，由 `model` 或 `view` 建立 state/event 关联，`render` 只消费 `view.json` 或明确的 annotation 输入。 | [pyveri DEVELOPMENT](../tools/pyveri/DEVELOPMENT.md#step-c1-收口-trace-输出和注释数据流) |
| `P1` | 待办 | docs | 同步开发文档与当前真实进展 | 清理过期描述，例如历史 obligation 状态；明确当前主入口、已覆盖阶段、剩余 `deferred` 和统一计划入口。 | 本文档 |
| `P1` | 待办 | model/docs | 补齐 Linux 裁剪配置下的 model deferred 标记 | 对照 `linux-6.12.37/default_config`，先把当前模型省略但仍属于参考启动路径的流程补到 `spec/model` 的显式 `deferred` 中；`spec/coding/arceos_ex.md` 只保留待迁移审计清单。首批候选包括 RISC-V boot image/EFI stub 入口、SATP mode 探测、early alternatives、stack end magic、build id、page address、cmdline 保存、DT unflatten、memblock 细节、final page table 权限、hwcap/ISA/alternatives 等。展开为正式对象或实现属于后续任务。 | [待迁移到 model 的 deferred 标记](../spec/coding/arceos_ex.md#待迁移到-model-的-deferred-标记) |
| `P1` | 进行中 | arceos_ex | 整理对象级源码结构 | 按 Object Coding Phase 映射规则整理源码结构：`main.rs` 承载 `startup-timeline`，`phases/` 按 Phase 包含层次拆分过程文件，`objects/` 按对象类别逐步拆成一对象一文件，并把资源对象统一收敛到全局 `Context`。目录分层另列为 P2。 | [arceos_ex 说明](../spec/coding/arceos_ex.md#新增核心-crate) |
| `P1` | 待办 | arceos_ex/smoke | 补齐内存分配 API 运行期 smoke | 当前可执行覆盖限于 `MemBlock::alloc_phys()` 的 checkpoint probe；`PageAllocator`、`kmalloc`/SLUB 和 `vmalloc` 目前只发布 Ready 状态与元数据事实，尚无公开 alloc/free API。后续实现对应 API 后，应补充 QEMU smoke：页分配/释放、kmalloc 小对象分配/释放与读写、vmalloc/vfree 区间分配/释放与读写，并保持不为 smoke 暴露无必要内部状态。 | [smoke 测试边界](../spec/coding/arceos_ex.md#smoke-测试边界) |
| `P1` | 待办 | arceos_ex/codegen | 收敛 RISC-V64 linker script 生成方案 | 将当前直接生成完整 `riscv64.lds` 的实验收敛为 Linux 风格的 `riscv64.lds.S`：`codegen` 基于 `Config` 生成 `generated/config.lds.h` 等配置头，`.lds.S` 保留链接布局结构并经预处理生成最终 `.lds`；同时规划 Rust 侧配置生成，避免 `.lds`、Rust 常量和 codegen profile 各自维护同一配置值。 | [arceos_ex 说明](../spec/coding/arceos_ex.md#目标边界) |
| `P1` | 待办 | CI/Homepage | 建立 nightly/manual 测试流水线和项目主页展示 | 生成 trace、系统测试日志、对象覆盖表、推导摘要和主页展示产物；主页作为测试流水线结果发布视图，优先展示最近一次 nightly 或手动 workflow 成功生成的结果。 | [arceos_ex 说明](../spec/coding/arceos_ex.md#ci-与项目主页) |
| `P2` | 待办 | model | 继续语义扩展 | 在 trace/文档闭环稳定后，再决定是优先消化已标记的 `deferred`，还是沿 Linux 启动流程继续推进到 `paging_init()` 之后的下一个阶段边界。 | [model SEMANTICS](../spec/model/SEMANTICS.md) |
| `P2` | 待办 | arceos_ex | `impl/arceos_ex/src/objects/` 目录分层 | 当前先在平铺 `objects/` 目录内拆清对象语义、静态存储、通用原语和 facade/API 边界；等主要大文件和命名稳定后，再考虑按 `model/`、`storage/`、`primitives/` 或主题子目录整理。这是结构收敛后的目录化工作，不紧急，避免当前阶段产生过多路径 churn。 | [arceos_ex 说明](../spec/coding/arceos_ex.md#新增核心-crate) |
| `P2` | 进行中 | pyveri | 工具链拆分与中间文件协议完善 | 独立阶段工具已经落地，后续继续细化 schema、退出码、缓存/增量重建策略，并保持独立阶段不反向依赖 `pyveri` 包。 | [pyveri DEVELOPMENT](../tools/pyveri/DEVELOPMENT.md#step-d-工具链拆分) |
| `P2` | 延期 | compose | 组件封装阶段 | 恢复 ArceOS 组件接口、crate 边界、`ax-std` 接入、overlay workspace、`xtask`、feature 传递、`axlog` 和 `ax-alloc` facade 等问题。 | [compose 规格](../spec/compose/README.md) |

## 已完成里程碑

| 领域 | 结果 | 说明 |
| --- | --- | --- |
| arceos_ex | 独立对象级实验目录落地 | `impl/arceos_ex/` 已包含 Makefile、RISC-V64 linker script、入口汇编和 no-alloc Rust 源码骨架。 |
| arceos_ex | 对象级公共基础落地 | 已定义状态集合、事件集合、`EventResult`、生命周期事件检查和 checkpoint hook。 |
| model/derive | 推导义务阶段性收口 | 当前 `make verify` 为 `0 obligation / 23 deferred`。 |
| arceos_ex | `EntryPreludePhase` 最小闭环 | 已覆盖 head prefix、BootArgs、RootStream、KernelImage、BootCPU/CpuGroup、InitTask/InitStack、RawDtb、FixMap、TrampolineVm、EarlyVm 和 VM 切换。 |
| arceos_ex | `EntrySuccessorPhase` 最小闭环 | 已覆盖 EarlyDtb、PlatformCpuInfo、PhysicalMemory、CpuIdMap、InterruptStream、BootCPU setup/enable、PrintkBuffer、CommandLine/KernelCmdline、EarlyParam、SBI、EarlyCon、MemBlock、InitMM、EarlyIoremap 和 SwapperVm。 |
| arceos_ex | `CorePreparePhase` 最小骨架 | 已按规格插入 PayloadPhase 前，覆盖 DeviceTree、Zones、ResourceTree、CpuGroup.setup_smp、CacheBlockInfo、CpuCapabilities、CommandLine saved/static 视图、PerCpuStorage、CpuHotplugState、Params、BootParam、PayloadParam、Randomness、PrintkBuffer.setup、ExceptionTable 和 ExceptionStream.setup。 |
| arceos_ex | `MmCoreInitPhase` 最小闭环 | 已覆盖 MemoryTopology、BootMemoryNode/BootZoneSet/BootZonelistSet、PageAllocator、MemoryDebugHardening、StackDepot、Swiotlb、SlubAllocator、PageTableCaches、VmallocAllocator 和 MmStructCache。 |
| arceos_ex | `SchedInitPhase` 最小闭环 | 已正式落到 `spec/model/boot/sched-init/` 和 `impl/arceos_ex/src/phases/boot/sched_init.rs`，覆盖 Scheduler、BootRunQueue、BootIdleTask、RadixTree、MapleTree、Workqueue.Prepared、Softirq.Prepared、RcuCore 和 `Scheduler.schedule_preempt_disabled()` smoke。 |
| arceos_ex | `IrqTimeInitPhase` 最小闭环 | 已正式落到 `spec/model/interrupt/irq-time-init/` 和 `impl/arceos_ex/src/phases/interrupt/irq_time_init.rs`，覆盖 IrqController、IrqDispatchTree、Tick/TimerWheel/HrtimerCore、Timekeeper、RiscvTimerProvider、Softirq.Ready、Randomness.Ready、SbiIpi、SmpCallFunction，并在阶段末尾打开 boot CPU 本地中断，新增 timer interrupt smoke。 |
| arceos_ex | `IrqOpenPreparePhase` 最小闭环 | 已正式落到 `spec/model/interrupt/irq-open-prepare/` 和 `impl/arceos_ex/src/phases/interrupt/irq_open_prepare.rs`，覆盖 SLUB late flush workqueue、Console.Prepared、SchedClock.Ready、DelayLoop.Ready 和中断打开后的 trimmed/deferred 路径。 |
| arceos_ex | `ProcessPreparePhase` 最小闭环 | 已正式落到 `spec/model/interrupt/process-prepare/` 和 `impl/arceos_ex/src/phases/interrupt/process_prepare.rs`，覆盖 RootPidNamespace、TaskCreationCore、CredentialCore、VMA/task context、namespace、keyring/security 等 rest_init 输入事实，并新增 process_prepare smoke。 |
| arceos_ex | `PayloadPhase` 最小闭环 | 已把启动链末尾的 selected payload 交接建模为不返回阶段，默认 `APP=smoke` 执行批量 smoke 用例后通过 SBI 关机。 |
| arceos_ex | no-alloc 输出路径 | 启动期内部输出前端和应用侧最小 `println!` 前端都写入 `PrintkBuffer`，再由 `EarlyCon(SBI)` drain。 |
| arceos_ex | 最小 FDT 解析 | 不引入外部 crate，不使用 `Vec`、`String`、`Box`，只解析当前闭环必要节点。 |
| arceos_ex | 顶层 Makefile 入口 | 顶层 `make build`、`make run`、`make run LOG=trace`、`make verify`、`make clean` 已可用，并支持 `APP=smoke` / `APP=hello` payload 选择。 |
| arceos_ex | trace/checkpoint 一致性复查 | `make verify REPORT=graph`、`make run LOG=trace` 和实现阶段顺序一致，checkpoint 单字符映射无重复。 |
| spec/coding | 规则强度分层 | 已为 `MUST`、`SHOULD`、`MAY`、`NOTE` 建立统一标注与解释规则，并把全局 `Context` 映射记录为 `SHOULD`。 |
| arceos_ex | 第一轮源码结构清理 | 已拆分 `entry_successor`、`entry_prelude`、FDT、静态页表、VM setup bridge 等聚合文件，保留后续目录分层为 P2。 |

## 当前交接标记

- 截止提交 41ce0b9 (impl: add pre smp init phase)，UP Multitask Phase 已形成两个子阶段最小闭环：`RestInitPhase` 和 `PreSmpInitPhase`。`PreSmpInitPhase` 从 `KernelInitDispatchGate.Ready` 分叉进入，不硬依赖 `RestInitPhase.Ready`。
- 当前正在推进 `SMP Runtime Phase` 子阶段 1 `SmpBringupPhase`：正式规格位于 `spec/model/smp-runtime/smp-bringup/`，当前实现策略只展开 BP 侧主线和 BP/AP 同步量，AP 内部 entry/callback 细节保持 deferred。
- 本阶段完成后需验收：`make verify`；`make build APP=smoke`；`make run APP=smoke`；`make test`；`make run LOG=trace APP=smoke`；`make verify REPORT=graph`。
- 上下文清理后恢复：先执行 git status --short --branch；预期工作树干净。

## 细节文档索引

- [tools/pyveri/DEVELOPMENT.md](../tools/pyveri/DEVELOPMENT.md)：pyveri 工具链架构、阶段工具职责、trace 输出和注释数据流设计。
- [spec/coding/arceos_ex.md](../spec/coding/arceos_ex.md)：`arceos_ex` 对象级实现边界、命令、CI/主页方案、平台任务和实现约束。
- [spec/model/SEMANTICS.md](../spec/model/SEMANTICS.md)：模型生命周期语义硬规则。
- [spec/coding/README.md](../spec/coding/README.md)：coding 规格阅读入口和规格优先级。
