# Testing 规格

本目录记录由规格生成、指导或审查测试用例时必须遵守的约束。

`spec/model` 仍是对象、状态、事件、action、依赖和事实的权威来源；`spec/coding`
仍是对象级实现的权威来源。`spec/testing` 不重新定义模型和编码语义，只规定测试代码如何从这些语义中选择测试目标、生成场景、隔离副作用并选择执行载体。

正式规格入口是 [`main.spec`](main.spec)。当前首先规格化 smoke 测试生成规则，见 [`smoke.spec`](smoke.spec)。

## 测试目标分类

smoke 测试生成时必须先判定测试目标类别，再决定是否依赖真实内核环境。

`TypeBehavior` 用于 `RawSpinLock`、`Completion` 这类可复用抽象类型。测试目标是类型语义，而不是某个生产实例。生成的 smoke case 默认应本地构造 subject 实例，只引入最小依赖对象，不绑定 `Context` 中的某个具体生产单例。若必须读取 live context 以满足前置条件，读取行为应保持为支撑条件，不得把该生产对象变成测试主体。

`KernelEnvironmentBound` 用于 `MemBlock`、`CpuGroup`、`Scheduler` 这类启动路径中的真实对象或事实集合。它们通常只出现一次，或脱离内核环境后测试意义不足。生成的 smoke case 可以直接读取 `Context` 中的生产对象，并验证阶段事实、对象事实和关键派生行为。

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

- 正常路径：满足前置条件时，event/action 成功并建立后置事实。
- 非法状态路径：subject 或依赖对象处于不允许的状态时，调用失败且不得提交成功事实。
- 重复操作路径：重复 setup、重复 acquire、重复 release、重复 wait/consume 等语义必须按规格接受或拒绝。
- 资源边界路径：计数、深度、队列、token、容量和空/满边界必须覆盖。
- 顺序敏感路径：`drives` 或 `within` 中指定的顺序必须能被测试直接或间接观察。
- 依赖缺失路径：缺少必要依赖对象、依赖状态不满足或前置事实缺失时，调用必须失败或保持 deferred 语义。

同一 smoke case 可以包含多个 scenario，但 scenario 之间不得依赖前一 scenario 留下的副作用。若一个 scenario 本身要测试连续动作序列，连续性必须局限在该 scenario 的 `run` 段内，并由该 scenario 的 `teardown` 完成恢复。

## KUnit 与 Smoke 的边界

`TypeBehavior` smoke case 默认不注册为 checkpoint KUnit case。KUnit checkpoint 更适合验证启动路径中的真实对象、checkpoint 时刻和内核环境绑定事实。若某个类型测试必须进入 KUnit，必须记录原因，说明它验证的不是普通类型行为，而是某个 checkpoint 下不可脱离环境的事实。

`KernelEnvironmentBound` smoke case 可以在 app smoke 和 checkpoint KUnit 中复用，但应明确它测试的是生产对象或阶段事实，不应伪装成通用类型测试。
