# Testing 规格

本目录记录由规格生成、指导或审查测试用例时必须遵守的约束。

`spec/model` 仍是对象、状态、事件、action、依赖和事实的权威来源；`spec/coding`
仍是对象级实现的权威来源。`spec/testing` 不重新定义模型和编码语义，只规定测试代码如何从这些语义中选择测试目标、生成场景、隔离副作用并选择执行载体。

正式规格入口是 [`main.spec`](main.spec)。当前首先规格化 smoke 测试生成规则，见 [`smoke.spec`](smoke.spec)。
Rootfs/user fixture、发行版 smoke、BusyBox-init scripted interaction 和 paired difftest 的权威编排规则见
[`rootfs.md`](rootfs.md)。

统一 Task carrier、独立 TaskFlow 生命周期、exec handoff 与 fresh child identity 的测试规则见
[`task-taskflow.md`](task-taskflow.md)。

运行期 `declare` 的 parser/checker/derive/JSON/view、128 次声明点压力与差分门禁见
[`dynamic-instance-declaration.md`](dynamic-instance-declaration.md)。

Stress/difftest 复合测试的 v2-only 配置、basic-test 编排和历史报告比较规则见
[`composite-tests.md`](composite-tests.md)。

单轮 kernel/QEMU 基本测试的配置、七阶段流水线、脚本隔离、退出和结构化结果规则见
[`basic-tests.md`](basic-tests.md)。

## tools2 Signal 工具链测试

`tools2` 本轮以完整 `spec/model/main.spec` 和既有最小 fixture 共同验收。独立入口是
`make -C tools2 test`，不得加入根目录默认 `make test`。测试必须覆盖：

- 每类 schema/version/producer 校验，含 version 4、拒绝 tools2 v1/v2/v3/老工具/旧 snapshot 和老工具不被
  tools2 产物误用的边界；
- Transition/Action 调用规范化、命名 payload 绑定、受控值/系统引用类型错误和带 span unsupported；
- self、向下、向上、同级、跨分支坐标，默认 `3/3`、整数、`all`、分支预算独立和 frontier truncation；
- drives 同步 source order、emits post-commit enqueue 与全局 FIFO 调度记录；
- drives 有序备选只发送首个当前可接受候选，未选候选不分配 Signal ID，全部不可接受时保留逐候选诊断；
- 所有 rejected 或 handler failure 都导致 failed，异步失败同样传播，条件不成立永不产生 pending；
- failed/bounded/until_signal_not_reached 不创建 snapshot-out，complete/reached snapshot 可作为下一 scenario 输入；
- 同一输入重复运行的 Signal ID、事件序号和 canonical JSON 完全稳定；
- view 不重新推导，text 能从根 Signal 还原 rejected/failed/truncated 的完整因果链。
- 主模型 parse/model 零诊断；effective parent 覆盖显式优先、BootTask→Kernel、普通非 Project 默认
  Kernel、ProjectObject 例外、无 Kernel fixture、unknown/self/cycle。
- tools2 结构事实 fixture 必须覆盖实例属性类型声明的 `slots`：已声明成员可证明
  `has_slot(instance.field, SlotKind::Member)` 并记录 `model_structure` proof source，未声明成员保持 rejected；主模型默认推导中的
  `FixMap.Preset` 不得再因 `Config.fixmap` 已声明的 FDT 槽位而拒绝。
- `tools2/bin/pyveri -h` usage 显示 `[-t SIGNAL] [-u SIGNAL]` 和 `--until`，旧 `tools2/pyveri2` 不存在；
  无参数入口使用 `Human -> ComputerProject.Preset`，新入口能从仓库根和其它 cwd 启动，scenario 与高级
  参数完整透传，显式 `--source` 覆盖默认 Human。
- shortcut、driver、derive API/CLI 都验证 `Target.Startup` 与 `Target.Preset` 规范化后的 request、Signal
  ID、事件序列和 canonical JSON 完全相同；正式 model transition 仍只有 `Preset`。
- until fixture 覆盖根发送前、同步 drives 发送前、emits 入队前、有序候选选择后、动态 receiver、已有
  FIFO、未提交祖先和重复目标；目标不得拥有 Signal ID、`signal_sent`、enqueue/receive/handler 事件，
  boundary snapshot 必须精确等于发送前稳定状态。
- reached 返回 0并生成带 provenance 的 v4 snapshot；既有 completed response 保持 completed，未提交
  ancestor 和未处理 FIFO 变为 stopped，summary 不产生 pending；failed/bounded/unreached 返回 1且不
  生成 snapshot。
- 不带 snapshot/scenario 的 `tools2/bin/pyveri -t Kernel.Preset` 必须因前期状态/事实缺失而 failed，且
  不隐式回溯 emitter 或合成快照；`tools2/bin/pyveri -t ComputerProject.Preset` 不带 scenario 必须能从
  模型初态开始真实推导。本轮不要求完整闭包达到 Online；若到达当前尚不满足的规格边界，必须 failed
  并报告缺项和完整因果链。重复运行已有可达部分的 Signal ID、顺序和 canonical JSON 必须稳定。显式
  提供真实到达边界 scenario 后也可从后续 Signal 继续。
- 显式 `--max-depth 0` 返回 bounded/1、frontier 可见且不生成 snapshot；完整运行 snapshot 可由 `-s`
  续跑。
- 主模型的 `tools2/bin/pyveri -u Kernel.Startup` 必须从默认 `Human -> ComputerProject.Preset` 开始并在
  `Kernel.Preset` 发出前 reached；其 snapshot 可用于后续 `-t Kernel.Startup -s SNAPSHOT`，不带 until
  时仍执行完整可达推导。发送前 snapshot 中三个直接子 Project 均为 `Ready`，
  `BootArgs/Config/Lds`、`Computer/Riscv64Platform/OpenSBI` 均为 `Online`，初态即 Online 的
  `BootCpuRegisters` 仍可访问，`Kernel` 仍为 `Base`；snapshot 还必须包含精确的 a0/a1 交接事实。
  Signal 创建顺序必须保留三次 Project Preset、三次 Project Setup 和
  `Computer -> Riscv64Platform -> OpenSBI -> Kernel` 的异步 FIFO 交接，且不得出现
  `BootArgs` 或 `BootCpuRegisters` 生命周期 Signal。
- 默认文本覆盖 Transition/Action 两种行、`Preset` 到 `Startup` 的显示别名、每层两空格的 hierarchy
  depth 缩进、负 depth 整体平移以及 Signal 创建顺序不被 depth 重排；Transition 状态必须来自 target
  的 before/after snapshot，stopped 未提交时显示相同状态，未解析 handler 不得猜类型或显示状态。
- stopped/rejected/truncated/failed 必须在各自 Signal 同行显示 outcome/reason；reached 的
  before-send boundary、failed 的 failure chain/reason 和空 Signal trace 都必须保持简洁且可辨识。
- 未设置 `VERBOSE`、`VERBOSE=0` 和其它非 `1` 值均选择 compact；只有 `VERBOSE=1` 逐字恢复既有详细
  文本和 canonical `Preset`。直接 renderer、render CLI、driver、shortcut、stdout 与 `-o` 都使用同一
  选择，并验证两种文本模式不改变 derive/view JSON、snapshot、verdict、Signal 顺序或退出码。

focused test 可用于开发，但最终必须依次运行 parser/model/Kernel focused、`make -C tools2 test`、老
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

`ObjectApiBehavior` 用于 `TaskCreationCore.copy_process()`、`CurrentRunQueueRef`/`RunQueue.EnqueueTask`/`RunQueue.PickNextTask` 这类正式对象 API 或 action 边界。测试目标是对象 API 契约本身，可以本地构造 subject 对象，并只读 live context 作为依赖前置条件。测试不得为了方便增加 `test_*` 被测入口；若实现缺少可调用边界，应补正式对象 API，使生产路径和测试路径共享同一语义入口。

`KernelEnvironmentBound` 用于 `MemBlock`、`CpuGroup`、`Scheduler` 这类启动路径中的真实对象或事实集合。它们通常只出现一次，或脱离内核环境后测试意义不足。生成的 smoke case 可以直接读取 `Context` 中的生产对象，并验证阶段事实、对象事实和关键派生行为。

`CpuGroup` 相关 smoke 必须归类为 `KernelEnvironmentBound`。测试目标是启动路径建立的真实 CPU 拓扑事实，而不是本地构造一个假的 CPU 组。生成测试应直接观察 `Context` 中的生产 `CpuGroup`、CPU 实例事实和 checkpoint facts，并至少断言：logical id `0` 通过 `BootCPURef` 指向 `BootCPU`；boot CPU 处于 possible/present/online；secondary CPU 在 bringup 前处于 possible/present/not-online；logical id 与 hartid 唯一；possible/present/online 集合元素是 CPU 引用而不是额外 CPU 本体对象。测试不得为了这些断言增加 `test_*` CPU API。

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
