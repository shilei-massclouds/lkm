# Testing 规格

本目录记录由规格生成、指导或审查测试用例时必须遵守的约束。

`spec/model` 仍是对象、状态、事件、action、依赖和事实的权威来源；`spec/coding`
仍是对象级实现的权威来源。`spec/testing` 不重新定义模型和编码语义，只规定测试代码如何从这些语义中选择测试目标、生成场景、隔离副作用并选择执行载体。

正式规格入口是 [`main.spec`](main.spec)。当前首先规格化 smoke 测试生成规则，见 [`smoke.spec`](smoke.spec)。
Rootfs/user fixture、发行版 smoke、BusyBox-init scripted interaction 和 paired difftest 的权威编排规则见
[`rootfs.md`](rootfs.md)。

统一 Task carrier、独立 TaskFlow 生命周期、exec handoff 与 fresh child identity 的测试规则见
[`task-taskflow.md`](task-taskflow.md)。

CPU/CpuGroup 唯一所有权、CpuRef、CurrentCPU selector 与迁移/Flow handoff 的测试规则见
[`cpu.md`](cpu.md)。

per-CPU Scheduler inventory、PreparePrev、class handoff、identity/nonidentity continuation 与 Linux
差分观测规则见 [`scheduler.md`](scheduler.md)。

运行期 `declare` 的 parser/checker/derive/JSON/view、128 次声明点压力与差分门禁见
[`dynamic-instance-declaration.md`](dynamic-instance-declaration.md)。

Stress/difftest 复合测试的 v2-only 配置、basic-test 编排和历史报告比较规则见
[`composite-tests.md`](composite-tests.md)。

具体实现之间的 checkpoint inventory、Linux 静态对照、coverage、marker patch 和 paired difftest 归属见
[`checkpoint-cross-reference.md`](checkpoint-cross-reference.md)；这些是 Testing/cross-reference 责任，不是 Coding 核心语义。

单轮 kernel/QEMU 基本测试的配置、七阶段流水线、脚本隔离、退出和结构化结果规则见
[`basic-tests.md`](basic-tests.md)。

## tools2 Signal 工具链测试

`tools2` 本轮以完整 `spec/model/main.spec` 和既有最小 fixture 共同验收。独立入口是
`make -C tools2 test`，不得加入根目录默认 `make test`。Python 全量和 focused 调用必须通过
`tools2/bin/test-python` 或由它支撑的 Make target；仓库根的 dotted unittest 调用也必须由
`tools2.tests` 自动装载所有并列源码包。上述入口不得要求调用者组装 `PYTHONPATH`，并必须有
清除外部 `PYTHONPATH` 后仍能导入 `pyveri` 及全部阶段包的回归测试。测试必须覆盖：

- 每类 schema/version/producer 校验，含 version 9、拒绝 tools2 v1 至 v8、老工具/旧 snapshot 和老工具不被
  tools2 产物误用的边界；
- Transition/Action 调用规范化、命名 payload 绑定、受控值/系统引用类型错误和带 span unsupported；
- self、向下、向上、同级、跨分支坐标，默认 `3/3`、整数、`all`、分支预算独立和 frontier truncation；
- drives 同步 source order、emits post-commit enqueue 与全局 FIFO 调度记录；
- `external Human` 解析、重复/非法声明、同步失败短路，以及两次 drives 成功后才异步入队 Enable；
  显式单 Signal 不得隐式补齐生命周期；
- drives 有序备选只发送首个当前可接受候选，未选候选不分配 Signal ID，全部不可接受时保留逐候选诊断；
- 所有 rejected 或 handler failure 都导致 failed，异步失败同样传播，条件不成立永不产生 pending；
- failed/bounded/until_signal_not_reached 以及 complete/reached 加 unresolved obligation 都不创建或覆盖
  snapshot-out；只有 `check.allowed == true` 的 v9 snapshot 可作为下一 scenario 输入；
- boundary 结构负例覆盖非法 category/ID、重复 ID、缺失或空 summary/evidence/resolution、错误
  close_when/revisit_when、重复属性/evidence 块及 legacy syntax；主模型 inventory 必须精确为 138
  deferred、53 trimmed，且 Type process composition/实例展开不得复制 inventory ID；
- occurrence 正反例覆盖 declaration、state、Transition、Action、`within`、Type process 多实例与同一
  boundary 的重复 Signal 到达。每次到达必须保留独立 proof/obligation；obligation 不改变 Signal 数量、
  顺序、state、outcome 或 animation causal moment，完整主模型 unresolved obligation 必须为零；
- 同一输入重复运行的 Signal ID、事件序号和 canonical JSON 完全稳定；
- view 不重新推导，text 能从根 Signal 还原 rejected/failed/truncated 的完整因果链。
- 主模型 parse/model 零诊断；effective parent 覆盖显式优先、BootTask→Kernel、普通对象默认
  Kernel、无 Kernel fixture、unknown/self/cycle。
- tools2 结构事实 fixture 必须覆盖实例属性类型声明的 `slots`：已声明成员可证明
  `has_slot(instance.field, SlotKind::Member)` 并记录 `model_structure` proof source，未声明成员保持 rejected；主模型默认推导中的
  `FixMap.Preset` 不得再因 `Config.fixmap` 已声明的 FDT 槽位而拒绝。
- `tools2/bin/pyveri -h` usage 显示 `[-t SIGNAL] [-u SIGNAL]` 和 `--until`，旧 `tools2/pyveri2` 不存在；
  无参数入口执行唯一 Human 外部编排且第一个真实 Signal 是 `Human -> Computer.Preset`，新入口能从仓库根和其它 cwd 启动，scenario 与高级
  参数完整透传，显式 `--source` 覆盖自动 snapshot provenance 或默认 Human。
- shortcut 只有在显式 `-t` 且没有 `-s` 时按 canonical signal 加载
  `tools2/scenarios/<CanonicalSignal>.snapshot.json`；覆盖 `Startup`/`Preset` 同文件、显式 scenario 优先、
  省略 `--source` 时从自动 snapshot 的 matching boundary provenance 恢复真实 sender、显式
  `--source` 覆盖 provenance，而显式 `-s` 不隐式恢复 sender；
  缺失文件在 derive 前返回 2 并报告 canonical signal/预期路径，以及 target/name、绝对路径、`..`、
  symlink 均不能越出 scenarios 目录，自动 snapshot 的协议、JSON/I/O、boundary identity 或
  source 错误也必须在 derive 前返回 2。省略 `-t` 的默认入口和 `-u Kernel.Enable` 必须继续从模型初态
  推导；driver/derive 无 scenario 时也继续使用模型初态。
- shortcut、driver、derive API/CLI 都验证 `Target.Startup` 与 `Target.Preset` 规范化后的 request、Signal
  ID、事件序列和 canonical JSON 完全相同；正式 model transition 仍只有 `Preset`。
- until fixture 覆盖根发送前、同步 drives 发送前、emits 入队前、有序候选选择后、动态 receiver、已有
  FIFO、未提交祖先和重复目标；目标不得拥有 Signal ID、`signal_sent`、enqueue/receive/handler 事件，
  boundary snapshot 必须精确等于发送前稳定状态。
- reached 且 obligation 为零返回 0并生成带 provenance 的 v9 snapshot；既有 completed response 保持 completed，未提交
  ancestor 和未处理 FIFO 变为 stopped，summary 不产生 pending；failed/bounded/unreached 返回 1且不
  生成 snapshot。
- 提交的 `tools2/scenarios/Kernel.Enable.snapshot.json` 必须与从仓库根运行
  `tools2/bin/pyveri -u Kernel.Enable --snapshot-out /tmp/kernel-enable-presend.snapshot.json` 得到的 canonical
  bytes 逐字节一致，并验证 v9、稳定 model fingerprint、发送前 boundary provenance、关键状态、
  Linux RV64 boot spec 采纳及其 a0/a1/satp/物理 PMD 要求、kernel ELF/boot artifact 构造事实、精确 a0/a1/satp
  交接、ordered boot/DTB 和 BootTaskRef 事实。该边界必须有 `task_concurrency_closed()`，但尚未有
  `interrupt_concurrency_closed()`、`context_is(SystemExclusive)` 或旧的 firmware SIE 事实。
  `tools2/bin/pyveri -t Kernel.Enable` 必须自动使用该 snapshot，derive `initial_snapshot` 与 golden
  完全一致且 Kernel.Enable 被 handler 接受；其后第一个 `InterruptType.Preset` 必须建立
  `interrupt_class_gates_closed`、`interrupt_pending_clear_write_completed` 及分路门控先于待决信号清理写完成的顺序事实，
  但不得建立 total-gate、fallback 或 `interrupt_concurrency_closed()`；完整闭包再成功到达 payload
  handoff/application 分支。不同 cwd 输出必须一致，陈旧或其它模型 fingerprint 必须拒绝。显式
  `-s` 仍可覆盖默认 golden。
- `tools2/bin/pyveri -t Computer.Preset` 在没有对应默认文件时必须返回 2；只有省略 `-t` 的默认
  Human 外部编排从模型初态真实推导。底层 driver/derive 直接请求 `Kernel.Enable` 且不带
  snapshot/scenario 时仍必须因前期状态/事实缺失而 failed，并且不得隐式回溯 emitter 或合成快照。
- 显式 `--max-depth 0` 返回 bounded/1、frontier 可见且不生成 snapshot；完整运行 snapshot 可由 `-s`
  续跑。
- 主模型的 `tools2/bin/pyveri -u Kernel.Enable` 必须从默认 Human 外部编排开始并在
  `Kernel.Enable` 发出前 reached；其 snapshot 可用于后续 `-t Kernel.Enable -s SNAPSHOT`，不带 until
  时仍执行完整可达推导。发送前 snapshot 中 `Computer/Riscv64Platform/OpenSBI` 均为 `Online`，
  `Kernel` 为 `Ready`，`Config/Lds` 为 `Online`，`BootInitFlow` 仍为 `Base`；初态即 Online 的
  `BootArgs/BootCpuRegisters` 与 `LinuxRiscv64KernelBootSpec` 仍可访问，snapshot 还必须包含精确的
  a0/a1/satp 交接、物理 PMD 对齐及 image 构造叶事实；不得提前包含 Kernel 入口才建立的中断关闭事实。
  前 15 个 Signal 的创建顺序必须精确保留三个 Human 创建的 Computer Signal、三个子 System 的三次
  Preset、三次 Setup、Config 先于 Lds Enable，以及 `Computer -> Riscv64Platform -> OpenSBI` 的异步
  FIFO 交接，以及 OpenSBI 同步驱动 CpuGroup 原子发布 `CpuGroup.cpus[0]`；15 个 Signal 的 identity、
  delivery、cause、handler 和 target before/after state 必须逐项固定。三个 Computer Signal 的
  source/delivery 必须分别为 Human/drives、Human/drives、Human/emits，均无共同 cause_id；三个 emits
  必须逐个 position=1 入队并以 remaining=0 出队。边界处 CpuGroup 与 CPU0 均为 Prepared，并恰有一个
  `indexed_instance_declared` 事件；Kernel.Enable 不得已有 Signal ID 或收发/handler 事件，且不得出现
  `BootArgs` 或 `BootCpuRegisters` 生命周期 Signal。
  同一次 derive 的 model/derive/view fingerprint 必须一致；compact、verbose 和 animation v3 都只能消费
  该推导，不得各自重建或重排 Signal。
- 主模型的 `tools2/bin/pyveri -u BootInitFlow.Preset` 必须同样从默认 Human 外部编排开始，并证明实际
  sender 是 OpenSBI：前 15 个 Signal 后，`sig-0016 OpenSBI -> Kernel.Enable` 必须以 emits/cause 0013
  入队、出队和 receive，全部 Kernel 入口 guard 成功并开始 `Kernel.Transition::Enable@Ready`；随后
  `sig-0017 Kernel -> Kernel.AcceptEnable`、`sig-0018 Kernel -> BootInitFlow.AssignCpuRef` 和
  `sig-0019 Kernel -> PhysicalDirect.ActivateOnCpu` 必须以 drives/cause 0016 依次完成 Action，分别提交
  `kernel_enable_accepted(Kernel)`、`BootInitFlow.cpu_ref = BootCPURef` 和 BootCPU 的 PhysicalDirect
  InitialActivation。在下一个 `BootInitFlow.Preset` 创建前 reached 时，0016 因未提交 ancestor 显示
  stopped 而不是 rejected/failed，Kernel 仍为 Ready、BootInitFlow 仍为 Base；boundary snapshot 必须
  等于 0019 after snapshot，且不得存在 BootInitFlow lifecycle Signal identity、send/receive/handler
  事件。该验收不得用显式 `-t Kernel.Enable` 的 Human 根 source 证明真实 sender。
- 提交的 `tools2/scenarios/BootInitFlow.Setup.snapshot.json` 必须与从模型初态执行
  `tools2/bin/pyveri -u BootInitFlow.Setup --snapshot-out /tmp/boot-init-flow-setup-presend.snapshot.json`
  得到的 canonical bytes 逐字节一致，并从仓库根与其它 cwd 重建出相同结果。当前正式模型下其
  SHA-256 固定为 `1f0327b33e1de1a493ccb78c89a3ee2781b8799a07229ca6341864072d8c6906`，model
  fingerprint 固定为 `sha256:36108bcafb76b847a0da2a05abc5fe6214033dbbbb9c0346f7906f798d196a05`。
  `tools2/bin/pyveri -t BootInitFlow.Setup` 必须自动采用该第 3 组入口 scenario；显式 `-s` 仍优先，
  其它模型必须因 stale fingerprint 拒绝，缺失 canonical scenario 必须在 derive 前返回 2。
- 提交的 `tools2/scenarios/Cpu0Scheduler.Schedule.snapshot.json` 必须与从模型初态执行
  `tools2/bin/pyveri -u Cpu0Scheduler.Schedule` 得到的 canonical bytes 逐字节一致；SHA-256 固定为
  `acf73a92956b48045009d3c4334b702ed515506daab5b31c8b482b85efa131f7`，model fingerprint 与上述
  Setup scenario 相同。该 before-send 边界的真实 sender 必须是 `BootIdleFlow`，上一个已完成 Signal
  必须是 `BootInitFlow -> BootIdleFlow.RequestSchedule`；边界处 BootInitFlow=Online、BootTask=OnCpu、
  BootIdleFlow=Ready，Cpu0Scheduler=Online、Cpu1Scheduler..Cpu7Scheduler=Ready，且 8 个
  `CpuGroup.cpus[i].scheduler` reference 分别指向唯一的 `Cpu{i}Scheduler` lifecycle identity。推导中
  不得已经存在 Cpu0Scheduler.Schedule Signal、PreparePrev 或 context switch occurrence。
- `-u BootInitFlow.Setup` 的真实上游推导必须精确包含 52 个 Signal，并逐项固定
  `sig-0016..sig-0052` 在 charter 因果账本中的 source、target、canonical name、delivery、cause、
  handler kind/id、target before/after state 和创建顺序。0016 必须是唯一 stopped Signal；0017–0052
  必须合计为 36 个 completed drives，完整 summary 必须为 51 completed、1 stopped、
  0 failed/rejected/truncated/pending。0020 只允许一对 `SingleTaskContext` enter/exit，enter 位于首个
  child send 前、exit 位于最后一个 child response 后和 0020 commit 前；第 1 组只有 CPU0 的一个 indexed declaration，
  第 2 组不得出现 Lock 或其它 fresh-instance event。
- `sig-0026 BindTaskStack` 必须先建立 PhysicalDirect 地址表示的 BootTask/stack pair；其后
  `sig-0027 CurrentCPU.Setup`、`sig-0028 InterruptType.Setup` 与 `sig-0029 TrapType.Preset` 必须严格
  按序发生。`sig-0029 TrapType.Preset` 前必须已经存在异常/中断共用 `TrapFlowType` 总入口事实；完成后
  `TrapType` 从 Base 进入 Prepared，并新增所属 CPU 临时保护入口、意外事件处理以及测试和缺陷定位
  用途事实。该 Signal 不直接产生 child Signal，动画 request 必须展示对应 Model 注释。
- `sig-0030 Vm.Preset` 必须首先以 `sig-0031 Vm -> KernelAddrSpace.Preset` 驱动地址空间 Preset，再准备
  Trampoline/Early 页表、RawDtb/FixMap 并完成 KernelAddrSpace.Setup；BootInitFlow 不得直接发送
  KernelAddrSpace lifecycle Signal。Vm.Preset 完成后
  BootCPU 仍由 PhysicalDirect 承载且 live SATP 为零；`sig-0036 RawDtb.Preset` 只建立固定头部可读边界，
  magic 事实必须到 `sig-0037 RawDtb.Setup` 才出现。`sig-0041 Vm.Setup` 才依次完成 Trampoline 与
  Early activation，并在结束时证明 BootCPU 由 EarlyVm 承载及 `stvec` 已恢复到 TrapType 保护入口。
  两个 Vm Signal 的动画 request 必须展示各自 Model 注释。
- `sig-0045 TrapType.Setup` 必须在 `Vm.Setup` 后把所属 CPU 的响应入口从临时保护入口重置为正式
  `TrapFlowType` 入口；其嵌套 `sig-0046 ExceptionType.Preset` 必须先驱动四个异常子类型的 Preset，
  再由父 Setup 提交 TrapType Ready。Setup 前不得存在正式入口/新鲜响应流/初始异常兜底事实，Setup
  后必须同时存在 `trap_response_entry_reset_to_formal_trap_flow`、`trap_formal_entry_creates_fresh_trap_flow`
  与 `exception_initial_fallbacks_prepared`；动画 request 必须展示正式入口 Model 注释。
- `make test-trap-setup-order` 必须检查实际 RV64 ELF：`TrapType::setup` 仅调用一次
  `ExceptionType::preset`，随后先写一次 `stvec`、再以 `csrw sscratch, zero` 清零一次 `sscratch`；`stvec`
  的 PC-relative 目标精确等于四字节对齐的汇编符号 `formal_event_entry`。该函数不得写 `satp`，也不得把
  非零寄存器值写入 `sscratch`。该检查纳入仓库根 `make test`。
- `make test-boot-init-flow-entry` 必须检查实际 RV64 ELF/source：MMU-off head 的 early marker 唯一包含
  `PhysicalDirect.ActivatedOnCpu(D) -> BootInitFlow.Started(O) -> InterruptType.Prepared(I)`，入口 receipt
  依次表示 CpuGroup、AcceptEnable、AssignCpuRef、PhysicalDirect 与 Preset acceptance；Rust 必须先整体
  校验再 adoption，不能晚发 Started。该检查还必须固定 Bind/CurrentCPU/Interrupt/Trap/Vm 顺序、
  Vm-owned KernelAddrSpace.Preset、`start_kernel` 的 Kernel Ready/Enable acceptance 门禁，以及既有
  sie→sip、sie-only adoption、Setup 首次清 SIE 和非链接 tail ABI。
- `BootInitFlow.Setup` 发送前 boundary 必须来自 `spec/model/phases/boot-init/phase.spec` 中
  BootInitFlow.Preset 成功提交后的真实 completion event，source/target 为
  `BootInitFlow -> BootInitFlow`、delivery 为 emits、cause 为 0020、FIFO position 为 1，snapshot
  精确等于 0020 after snapshot。Kernel 必须只创建 BootInitFlow.Preset 并保持 Ready，BootInitFlow 必须为 Prepared；入口关键对象
  状态必须与 charter 账本一致，`task_flow_started(BootInitFlow)`、中断关闭、BootCPU 浮点/向量默认关闭及受控使用策略、BSS 清零完成且作为普通可写内存使用、`boot_cpu_hartid_recorded_for_later_use(BootCpuRegisters.a0)`、物理/虚拟 `tp/sp/gp`、
  临时保护入口、正式 TrapFlowType 入口、初始异常兜底、DTB/fixmap、VM translation 和 Soc
  Preset deferred 事实必须存在。结构化 Signal 必须把
  `selector_resolutions` 数组必须保留 `CurrentCPU` 的 source flow `BootInitFlow`、source ref `BootCPURef`
  与 canonical target `CpuGroup.cpus[0]`。`BindTaskStack` 与 `RefreshTaskStack` Signal 必须分别展示
  “首次原子建立”和“同一 pair 原子刷新”的 Model 注释；边界 snapshot 必须保存 CPU0 的
  CurrentTask -> BootTask/BootTaskRef/BootInitFlow 与 CurrentStack -> BootTask.stack/BootInitFlow 两种
  contextual binding、EarlyVm 地址表示与 revision 2。边界不得包含 `BootCurrentCPU`、
  `BootCpuCurrentTask`、`CurrentTaskSlot`、`CurrentStackSlot` 或 Stack object 的 state/instance，也不得
  创建、发送、接收或处理公开 BindStack Signal；同一 Boot Action 的中间快照不得出现半绑定状态，也不得
  创建、发送、接收或处理
  BootInitFlow.Setup；该边界不得提前包含 `cpu_hartid_ready(CpuGroup.cpus[0], ...)`。compact text 必须显示 reached/before-send 与 0016 stopped，verbose text 必须保留
  真实 Human root、0020 identity、handler 和 reached boundary，结构化 event 必须保留 context，且两种
  text render 都不得改变 derive/view/snapshot。
- 同一 `BootInitFlow.Setup` completion-event 边界的 animation v3 必须精确包含 52 request、48 feedback、3 settle、
  1 terminal，共 104 moments。0016 terminal 表示已 receive 的 stopped emits ancestor；0020 feedback
  必须晚于 0021–0052 的全部嵌套 feedback，且边界必须保留 BootInitFlow 自发 emits、cause 0020 与
  FIFO position 1 的因果元数据；
  0011–0013 仍各自 settle，边界 Signal 不得产生 moment。
- 第 2 组负例必须从 canonical 快照删除单个真实前提并验证首个失败边界：缺失 Kernel acceptance、
  BootTask `OnCpu/Live/Invalid` 或 initial-flow binding 时不得创建第一个入口 child；缺失入口
  a0/a1/satp 或 VM 条件时不得越过消费它的 handler；缺失任一所选早期对象 guard 时，失败 child 后
  不得创建后续 sibling。所有这些路径必须保持 Kernel Ready、BootInitFlow Base，不提交
  `task_flow_started(BootInitFlow)` 或 BootInitFlow.Prepared，也不得生成 BootInitFlow.Setup。
  绕过真实上游、重复 BootInitFlow.Preset、stale fingerprint 与缺失默认 scenario 继续严格拒绝。
- 主模型负例必须继续覆盖缺失 Computer assembly、OpenSBI/Linux boot 前提和 Kernel a0/a1/satp/Image
  guard、绕过 Kernel.Enable、重复 Enable、`AcceptEnable` 同步子失败，以及 emits 异步失败传播。失败
  不得创建 BootInitFlow Signal，并须保持 cause chain、未提交 ancestor 和最后稳定 snapshot。
- 默认文本覆盖 Transition/Action 两种行、`Preset` 到 `Startup` 的显示别名、每层两空格的 hierarchy
  depth 缩进、负 depth 整体平移以及 Signal 创建顺序不被 depth 重排；Transition 状态必须来自 target
  的 before/after snapshot，stopped 未提交时显示相同状态，未解析 handler 不得猜类型或显示状态。
- stopped/rejected/truncated/failed 必须在各自 Signal 同行显示 outcome/reason；reached 的
  before-send boundary、failed 的 failure chain/reason 和空 Signal trace 都必须保持简洁且可辨识。
- 未设置 `VERBOSE`、`VERBOSE=0` 和其它非 `1` 值均选择 compact；只有 `VERBOSE=1` 逐字恢复既有详细
  文本和 canonical `Preset`。直接 renderer、render CLI、driver、shortcut、stdout 与 `-o` 都使用同一
  选择，并验证两种文本模式不改变 derive/view JSON、snapshot、verdict、Signal 顺序或退出码。
- animate 必须拒绝错误 model/view schema、producer、version、source/fingerprint 组合、缺失端点、
  parent cycle、未知 handler 结构和损坏 snapshot；输出使用安全 JSON 内嵌与原子写入，失败不留半成品。
- animation v3 fixture 必须覆盖 drives、同步根请求、emits、同步嵌套、emits 内嵌 drives、异步 FIFO、
  Transition/Action 和 completed/rejected/failed/truncated/stopped。`signal_sent` 只作证据，receive 生成
  request；drives/root 终止生成 feedback，emits 终止生成 settle，truncated/stopped 生成 terminal。未
  receive 的 Signal 只有 terminal，不 reveal target。每个 moment 按 event sequence 排列，同步父 feedback
  必须晚于全部同步子 feedback。request 的 stable before、feedback/settle 的真实 after、异常 snapshot、
  reason、缺失/重复/乱序事件、delivery 分类与 outcome 不匹配都必须验证。初始 frame 为空，节点只在
  request reveal；虚拟 `$root` 和任意 parent 的 sibling order 稳定按 `first_seen` 升序，feedback/settle/
  terminal 不得改变首次出现或同级顺序。至少四层 parent 树不得因层级方向或当前 Signal 重排；重复生成、
  sibling 稳定性与任意前后往返必须恢复完全相同的 frame 和 sibling order。
- 主模型 `-u Kernel.Enable` 的 animation v3 必须精确包含 15 个 Signal、30 个 moment：15 request、
  12 feedback、3 settle。request 顺序服从实际 receive/FIFO，Preset/Setup/CpuGroup Preset 的父 feedback 位于同步子
  feedback 之后；Computer、Riscv64Platform、OpenSBI 的 Enable 分别在 emits settle 提交且不得伪装成
  feedback。Preset 三个
  子系统依次 Prepared 后 Computer 才变为 Prepared；Setup 的平台、OpenSBI、Kernel（含 Config/Lds）
  全部 Ready 后 Computer 才变为 Ready；Computer 在自身 Enable 完成时变为 Online，随后才发送平台
  Enable，全程不得状态回退。Kernel.Enable 仍没有 Signal/moment，最终说明显示 before-send boundary。
- frontend 单元测试必须覆盖 parent 包含、一级/三级 row、二级/四级 bottom-to-top column、children 位于
  identity 上方、偶数层左对齐与奇数层底部对齐；普通 system 不显示 `SYSTEM`，名称和裸状态名或
  `Stateless` 使用完全相同字号并正确换行、不重叠，节点及底部响应均不显示 `State::`，External/
  Structure 标签和紧凑宽度保持。箭头几何测试覆盖四方向单一直线路径、source 边起点、target 边精确
  终点和线段中点 label；自环必须位于节点上方、按节点宽高缩放、label 位于上弧外侧且不越出预留
  顶部净空。组件测试覆盖直接和跨多层 ancestor 到 descendant 不生成 SVG，并确认 self、普通同级、
  跨分支和 descendant 到 ancestor 仍生成；异常线型/reason、request/feedback/settle 状态样式、说明句/footer 不渲染和
  reduced-motion 继续覆盖。
- frontend 效果测试必须覆盖 request 箭头和 target 抖动、feedback 不画反向箭头且提交状态并闪烁、
  Action feedback 闪烁但不伪造状态、settle 不产生反馈闪烁且显示内部完成、异常 settle 的红色效果与
  reason、祖先箭头隐藏时 target 仍抖动、reduced-motion 取消动画但保留确定高亮，以及普通/错误 SVG
  marker 都精确缩小到原宽高的 50%。
- 浏览器端到端测试必须覆盖 request 箭头、feedback/settle 不重复箭头、异常终止保持状态、按钮、
  ArrowLeft/ArrowRight、离线加载和本次主模型 derive 产物的完整时刻往返；交替层级 fixture
  必须验证 Human 固定 stage 左下，后续一级节点只向右，二级向上、三级向右、四级向上。无碰撞时
  已有内容坐标不变；碰撞时只有较晚分支向右或向上，较早锚点不动；父框只随可见子树向上/向右
  膨胀且不覆盖文字。箭头必须跟随四向布局、膨胀、滚动与 resize；Human 到 Root 显示直线，Root 到
  Child 隐藏箭头但仍产生 Action 响应效果，异常 self Signal 走节点上弧。前后确定性、自动滚入视区和
  reduced-motion 保持可用，不得观察到网络请求或浏览器 derive 路径。
- 浏览器布局验收必须覆盖 1024×768、1280×900 与 1920×1080：桌面页面无外层纵向滚动，stage 至少
  占 viewport 高度约 72%，header/transport 不随宽屏无界增长；390×844 允许换行但外层页面仍固定，
  不得文字覆盖或页面溢出，纵向和必要的横向 overflow 只能出现在 stage 内。浏览器测量必须确认名称与状态字号在
  桌面、移动端完全相等；桌面和移动端都维护稳定截图基线。
- `lkm-animate MODEL VIEW -o HTML` 与 driver/shortcut `--html-out` 必须覆盖成功、协议/I/O 错误、check
  退出码 0/1 保留，以及与 text `-o`、stdout、scenario、snapshot-out 和 work-dir 的组合。编译 bundle
  rebuild 后必须通过 stale check，固定 fixture 生成 `tools2/out/pipeline-animation.html`。

focused test 可用于开发；标准命令为
`make -C tools2 test-focused TEST=tools2.tests.<module>.<Class>.<method>`，不得在命令行重述包源码路径。
最终必须依次运行 parser/model/Kernel focused、`make -C tools2 test`、老
model/view/render、静态 trace/SVG 重新生成与布局评审、`git diff --check`，再从仓库根目录以不包装、
不重定向的直接 `make test` 完成回归门禁。

## 测试生成元规则

测试用例生成还受 [`../guidance/main.spec`](../guidance/main.spec) 的上层元规则约束。AI 或其它代码生成器生成测试用例时必须按三步执行：

1. 阅读测试生成原则和具体规格要求。先阅读 `spec/guidance` 的生成工作流、本目录的测试实现前提原则、目标分类、场景结构、断言策略和 smoke/KUnit 边界，再阅读目标对象在 `spec/model`、`spec/coding` 中对应的正式规格。
2. 实现测试用例。实现时只使用正式对象 API 和测试执行载体允许的入口，不得跳过第一步直接写测试。
3. 检查生成结果。实现后必须回查生成的测试是否满足原则和具体要求，包括是否新增了测试专用 API、是否改变了功能 API、是否错误地交叉注册 smoke/KUnit，以及断言、隔离和 teardown 是否符合规格。

## 测试实现前提原则

测试用例不得为了测试便利新增 `test_*` 或其它测试专用被测 API。若规格动作缺少可调用实现边界，应回到 model/coding 规格补正式对象 API，再由生产路径和测试路径共同使用。

测试用例不得为了测试便利改变既有功能 API 的签名、语义、可见性或错误语义。若现有功能 API 与正式规格不一致，应作为规格/实现修正处理，而不是作为测试适配处理。

checkpoint/KUnit 是基本测试中的 checkpoint callback profile，不是复合测试。handler 默认是只读
observer；它只能读取真实 checkpoint 时刻已经存在的对象 facts、counter、trace 或 sink 输出，不得为了
测试便利调用 driver/probe/ring/IRQ action，不得 reset/snapshot/restore 普通对象状态，不得伪造
completion 或设备事件，也不得要求被测对象新增测试专用后门。

checkpoint/KUnit handler 原型必须是只读 `Context` 加受限 `Sink`，即等价于 `fn(Checkpoint, &Context, &mut dyn Sink) -> CheckpointOutcome`。不得恢复 `Write` handler、`&mut Context` 参数，或任何能修改普通 context 对象的等价入口。

若某个 checkpoint KUnit 确实必须执行会改变对象内部数据的 action，必须先在 testing/coding 规格中明确它是 action-level probe，写清允许的最小 capability、状态清理边界和为什么不能通过 smoke 或只读 observer 覆盖。没有明确记录时，一律按只读 observer 处理。

新增 smoke 测试默认只注册到 smoke 执行入口，不自动加入 checkpoint KUnit。若确需复用到 KUnit，必须记录它验证的 checkpoint 事实和不能仅由 smoke 覆盖的理由。

新增 KUnit 测试默认只注册到 KUnit/checkpoint 执行入口，不自动加入 smoke。若确需复用到 smoke，必须记录它验证的 payload/端到端可观测行为和不能仅由 KUnit 覆盖的理由。

## 测试目标分类

smoke 测试生成时必须先判定测试目标类别，再决定是否依赖真实内核环境。

`TypeBehavior` 用于 `RawSpinLock`、`Completion` 这类可复用抽象类型。测试目标是类型语义，而不是某个生产实例。生成的 smoke case 默认应本地构造 subject 实例，只引入最小依赖对象，不绑定 `Context` 中的某个具体生产单例。若必须读取 live context 以满足前置条件，读取行为应保持为支撑条件，不得把该生产对象变成测试主体。

`ObjectApiBehavior` 用于 `TaskCreationCore.copy_process()`、`Scheduler.EnqueueTask`/`Scheduler.PickNextTask` 这类正式对象 API 或 action 边界。测试目标是对象 API 契约本身，可以本地构造 subject 对象，并只读 live context 作为依赖前置条件。测试不得为了方便增加 `test_*` 被测入口；若实现缺少可调用边界，应补正式对象 API，使生产路径和测试路径共享同一语义入口。

`KernelEnvironmentBound` 用于 `MemBlock`、`CpuGroup`、`Scheduler` 这类启动路径中的真实对象或事实集合。它们通常只出现一次，或脱离内核环境后测试意义不足。生成的 smoke case 可以直接读取 `Context` 中的生产对象，并验证阶段事实、对象事实和关键派生行为。

`CpuGroup` 相关 smoke 必须归类为 `KernelEnvironmentBound`。测试目标是启动路径建立的真实 CPU 拓扑事实，而不是本地构造一个假的 CPU 组。生成测试应直接观察 `Context` 中的生产 `CpuGroup`、CPU 实例事实和 checkpoint facts，并至少断言：logical id `0` 通过 `BootCPURef` 指向 canonical `CpuGroup.cpus[0]`，`BootCPU` 只作为该元素的角色别名；boot CPU 处于 possible/present/online；secondary CPU 在 bringup 前处于 possible/present/not-online；logical id 与 hartid 唯一且可双向定位；possible/present/online 集合元素是 CpuRef 而不是额外 CPU 本体或 view。测试不得为了这些断言增加 `test_*` CPU API。

## 场景三段体

每个生成的测试场景必须使用三段体：

- `setup`：建立该场景的最小前置状态，创建 subject 和依赖对象，必要时记录外部环境快照。
- `run`：执行同一场景意图下的一个或多个动作，检查返回结果、状态迁移、事实、依赖对象效果和顺序约束。
- `teardown`：释放该场景持有的资源，恢复外部环境快照，并确认后续场景不会被污染。

`teardown` 是硬约束。测试中途发现失败也不得让全局中断状态、抢占状态、锁持有状态、队列内容、完成量 token 或其它可影响后续场景的状态泄漏到下一场景。局部对象可以在场景结束后丢弃，但外部环境必须恢复到 `setup` 记录的边界。

## 断言与失败策略

测试目的必须通过断言完成，不能依赖人阅读日志判断。输出信息只作为诊断补充，不得作为 pass/fail 的判定依据。生成的 `run` 段必须把每个测试目标转化为明确断言，例如：

- `ASSERT(condition)`：期望条件成立。
- `ASSERT_FAIL(operation)`：期望操作失败，用于非法状态、重复 acquire、未持有 release 等异常路径。
- `ASSERT_TIMEOUT(condition, bound)`：期望条件在有限边界内成立，超时是测试失败，不是无限等待。

后续可以引入 `ASSERT_EQ`、`ASSERT_STATE`、`ASSERT_EFFECT`、`ASSERT_ORDER` 等派生形式，但它们都必须归约为明确的测试断言，而不是日志输出。

断言不满足时，默认托底行为不得是停机、panic 或无限等待。测试框架应记录失败，标记当前 scenario 或 case 失败，并继续执行该 scenario 的 `teardown`。是否继续后续 scenario/case 由测试执行策略决定；只有显式 strict/debug 策略可以把失败升级为停机。

断言失败不得跳过 `teardown`。对于会修改外部环境的测试，生成代码应使用统一 scenario runner、guard 或等价结构，保证失败路径和成功路径都执行环境恢复。

## 场景覆盖

生成测试不应只覆盖正常路径。对 `TypeBehavior`，测试生成至少应从正式规格中枚举以下场景类别：

- 正常路径：满足前置条件时，transition/action 成功并建立后置事实。
- 非法状态路径：subject 或依赖对象处于不允许的状态时，调用失败且不得提交成功事实。
- 重复操作路径：重复 setup、重复 acquire、重复 release、重复 wait/consume 等语义必须按规格接受或拒绝。
- 资源边界路径：计数、深度、队列、token、容量和空/满边界必须覆盖。
- 顺序敏感路径：`drives` 或 `within` 中指定的顺序必须能被测试直接或间接观察。
- 依赖缺失路径：缺少必要依赖对象、依赖状态不满足或前置事实缺失时，调用必须失败或保持 deferred 语义。

同一 smoke case 可以包含多个 scenario，但 scenario 之间不得依赖前一 scenario 留下的副作用。若一个 scenario 本身要测试连续动作序列，连续性必须局限在该 scenario 的 `run` 段内，并由该 scenario 的 `teardown` 完成恢复。

## KUnit 与 Smoke 的边界

`TypeBehavior` smoke case 默认不注册为 checkpoint KUnit case。KUnit checkpoint 更适合验证启动路径中的真实对象、checkpoint 时刻和内核环境绑定事实。若某个类型测试必须进入 KUnit，必须记录原因，说明它验证的不是普通类型行为，而是某个 checkpoint 下不可脱离环境的事实。

`ObjectApiBehavior` smoke case 默认也不注册为 checkpoint KUnit case。它可以覆盖正式对象 API 的成功/失败边界，但不代表某个 checkpoint 时刻必须出现的全局事实。若要纳入 KUnit，必须说明该 API 行为为什么只能在 checkpoint 环境下验证。

`KernelEnvironmentBound` smoke case 可以在 app smoke 和 checkpoint KUnit 中复用，但应明确它测试的是生产对象或阶段事实，不应伪装成通用类型测试。
