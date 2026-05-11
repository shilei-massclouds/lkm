# pyveri Development Plan

本文档记录 `pyveri` 第一版推导验证器的实现计划。当前目标是验证：

```text
../../spec/entry-prelude-object-model.spec
```

## 定位

`pyveri` 是面向组件化内核规格的静态推导验证器。它不读取运行时轨迹，也不依赖模拟器、硬件采集或参考内核差分数据。

第一版验证器只做基于 `.spec` 文件本身的纸上推导验证：从规格声明的顶层启动时间轴出发，检查对象、阶段、状态和事件能否按模型规则逐级推出目标状态。

当前推导入口是：

```text
StartupTimeline.Event::Setup
```

当前推导目标是：

```text
StartupTimeline.state == State::Ready
```

## 规格语义

- `object` 是推导中的实体单位。
- `parent` 建立对象之间的静态父子层级。
- `initial_state` 给出对象进入推导模型时的初始状态。
- `state` 定义对象可处于的状态。
- `on Event::X -> State::Y` 定义状态迁移规则。
- `depends_on` 是事件执行前必须满足的前提。
- `drives` 是事件触发的子推导队列，必须按声明顺序处理。
- `invariant` 是对象进入某状态后必须验证的状态约束。
- `deferred` 是规格明确保留的延期证明义务，不等同于注释。
- 注释只供人阅读，解析时忽略。

## 实现阶段

### 1. Parser

输入 `.spec` 文件，输出语法级 AST。

第一版需要支持：

- 去除 `/* ... */` 和 `// ...` 注释。
- 解析 `enum`、`function`、`predicate`、`type`。
- 解析 `object`、`initial_state`、`parent`、`attrs`。
- 解析 `state`。
- 解析 `events` 和 `on Event::... -> State::...`。
- 解析 `depends_on`、`drives`、`may_change`、`invariant`、`deferred`。

第一版可以把复杂表达式保留为原始字符串，不急于完整求值。

### 2. Model Builder

把 AST 装配成可推导模型。

需要建立：

- 对象表。
- 类型表。
- 父子层级。
- 属性表。
- 每个对象的状态表。
- 每个对象的事件表。
- 事件的目标状态、依赖、驱动队列、延期义务。
- 状态的不变量集合。

这一阶段负责把解析得到的扁平声明反扁平化为对象层级和事件层级；它仍然不执行正式推导。

需要检查：

- 对象名重复。
- 状态名重复。
- 事件名重复。
- `parent` 指向的对象存在。
- 事件目标状态存在。
- `Object.Event` 引用存在。
- `Object.state == State::X` 引用存在。

语法错误和模型结构错误应作为 error 报告并停止；不影响建模但可能影响后续推导质量的问题可以作为 warning 报告。

### 3. Derivation Engine

从推导入口开始执行静态推导。

基本规则：

1. 每个对象从 `initial_state` 开始。
2. 推进事件前，当前对象必须处于包含该事件的源状态。
3. 推进事件前，`depends_on` 必须可证明或形成明确证明义务。
4. `drives` 中的子事件按声明顺序递归推进。
5. 子事件全部完成后，当前对象进入目标状态。
6. 进入目标状态后验证该状态的 `invariant`。

第一版不做复杂谓词求值；未定义或不可计算的谓词进入证明义务列表。

### 4. Diagnostics

推导结果至少分为：

- `proved`：由模型规则成功推出。
- `assumed`：作为当前规格范围的输入事实接受。
- `obligation`：无法在第一版中求值，但必须保留的证明义务。
- `deferred`：规格明确标记的延期证明义务。
- `blocked`：缺少必要前提，导致事件无法推进。
- `contradiction`：模型内部出现状态或约束矛盾。
- `syntax_error`：语法错误，停止处理。

### 5. CLI

第一版命令形式：

```text
python -m pyveri ../../spec/entry-prelude-object-model.spec
```

默认目标：

```text
StartupTimeline.Event::Setup
```

后续可以增加参数：

```text
--target StartupTimeline.Event::Setup
--format text|json
--strict
```

### 6. Tests

第一版测试应覆盖：

- 注释剥离。
- 基础语法解析。
- 对象父子层级。
- 状态和事件引用检查。
- `drives` 顺序保持。
- `deferred` 收集。
- 从 `StartupTimeline.Event::Setup` 推导到目标状态的最小路径。

## 第一版非目标

- 不解析或执行真实内核代码。
- 不读取运行时日志、插桩、模拟器或硬件采集数据。
- 不做参考内核差分。
- 不完整求解所有谓词语义。
- 不引入 SMT/SAT 求解器。
