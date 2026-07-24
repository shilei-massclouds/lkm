# 阶段范式

本文定义 charter 层如何把阶段设计意图生成到 model。它只规定阶段对象、生命周期、父子驱动
和迁移完成事件的语义，不规定函数、状态变量或 checkpoint 的实现方式；model 到 impl 的映射见
[`spec/coding/phase-paradigm.md`](../coding/phase-paradigm.md)。

阶段（Phase）是内核生命周期中的编排对象。每一级阶段都使用同一套生命周期，并通过
`drives` 显式驱动下一级阶段；阶段自身的 `Preset`、`Setup`、`Enable` 由同对象的 `emits`
完成连续推进。

## 标准生命周期

```text
Base --[Preset]--> Prepared --[Setup]--> Ready --[Enable]--> Online
```

| 状态 | 含义 |
| --- | --- |
| Base | 阶段对象已进入模型，但尚未收到 `Preset` 启动事件。 |
| Prepared | `Preset` 已完成，阶段的初始依赖和准备结果已经成立。 |
| Ready | `Setup` 已完成，阶段具备上线条件，但尚未提交对外可用边界。 |
| Online | `Enable` 已完成，阶段承诺的能力和后置事实可供上层依赖。 |

标准阶段必须保留四个状态和三个迁移。某个迁移可以没有实际 `drives` 动作，但不得因此省略
状态边界。确实无法使用标准生命周期的对象必须在 charter 中说明原因和范围，并在 model 中
作为显式例外，而不能让缺失的迁移由工具或实现隐式补齐。

## 类型生命周期继承

类型生命周期使用累积继承，而不是由最近声明的类型遮蔽基类型。该规则适用于所有 model 类型，
不按 `TaskFlow` 或其它类型名称特判；没有 lifecycle 贡献的空壳类型（例如 `PhaseObject`）不会因此
自动获得 lifecycle。

同一个 handler 的有效定义按基类型到派生类型、最后到实例声明的顺序组成。基类型拥有通用的
source/target state、`state_effect`、guard、完成事实和 completion event；派生类型与实例只能追加
自己的特化条件、动作和事实：

- 所有 `depends_on` 共同成立才可接受 Signal；
- `drives`、`within` 及其它执行 body member 按每个贡献者的源码顺序、再按基类到实例的贡献顺序执行；
- `ensures` 与目标状态 invariant 累积，并绑定实际 instance 的 `self`；
- 所有 `emits` 只在整个有效 transition 成功提交、目标 invariant 全部成立后，按基类到实例及各自
  源码顺序入队。

派生类型或实例不得重复父级已经规范化的 `depends_on`、`drives`、`ensures`、state update 或
`emits` 条目，也不得声明与继承 source/target state、参数或 `state_effect` 冲突的同名 handler。
model 必须以各贡献条目的 source span 报错；工具不得以静默去重、最近声明覆盖或执行两次来解释
重复声明。

`lifecycle_override: true` 是唯一完整替换机制。override 必须同时给出完整 initial state、state graph
和 handler 契约；它替换全部继承贡献，不允许只覆盖某个 guard、动作、完成事实或 event 来削弱
基类型契约。

## 同对象迁移链

标准阶段的三个迁移按以下规则生成：

1. `Preset` 从 `Base` 迁移到 `Prepared`，成功提交后 `emits Transition::Setup`。
2. `Setup` 从 `Prepared` 迁移到 `Ready`，成功提交后 `emits Transition::Enable`。
3. `Enable` 从 `Ready` 迁移到 `Online`；它不隐式发出其它阶段的事件。

这里的 `emits` 只串联当前阶段对象自己的生命周期迁移。它表示当前迁移已经提交目标状态并
通过目标状态 invariant 后产生的 completion event，不是 `drives` 的别名，也不用于自动连接
同级阶段。阶段范式不生成 `PhaseA.Enable emits PhaseB.Preset` 形式的 sibling 边。

## 父子驱动

上下级阶段之间只通过父 transition 中显式声明的 `drives` 建立关系。父阶段负责选择被驱动的
子阶段和顺序；子阶段不拥有下一 sibling 的推进权。

标准的完整子阶段驱动写作：

```text
drives {
    ChildPhase.Transition::Preset;
}
```

`ChildPhase.Preset` 被触发后，子阶段通过自己的 `emits` 依次完成 `Setup` 和 `Enable`。子阶段
达到 `Online` 后，该次 `drives` 才完成，父 transition 继续执行其后续 body member。一个父
transition 中有多个 `drives` 条目或多个 `drives` 块时，严格按照 model 源码顺序执行。

若 charter 只要求推进一个已经由其它边界建立到 `Prepared` 或 `Ready` 的阶段，可以显式驱动
其 `Setup` 或 `Enable`，但必须说明先前状态由谁建立以及为何不由该父阶段驱动完整生命周期。
这种情况是跨边界续推例外，不得被解释成标准阶段可以省略 `Preset`、`Setup` 或 `Enable`。

同一父阶段下的多个子阶段只是结构上的 siblings，不存在子阶段之间的隐式执行边。它们的
可观察顺序完全来自共同父 transition 的 source-ordered `drives`，以及父阶段自身的迁移链。

## Replicated phase family

当同一个 PhaseObject 对一组目标实例重复执行时，charter/model 中的单个 PhaseObject 表示
replicated phase family，而不是由协调者拥有的聚合生命周期。每个目标 key（例如 secondary
CPU 的 `logical_id`）都有独立的 `Base -> Prepared -> Ready -> Online` 四态和唯一执行 owner。

replicated family 的父 `drives` 和 sibling 顺序按相同 target key pointwise 解释：对同一个
`logical_id`，前一 sibling 必须到达 `Online` 后才能触发后一 sibling 的 `Preset`；不同
`logical_id` 的执行可以交错，不能从 model 的 source order 推导出跨目标的全局阶段屏障。
family 的 `Online` 查询表示所有目标实例都已到达 `Online`，只用于协调者完成 wait/barrier 和
后续父级提交，不改变各实例先前独立提交 Online 的时点或 owner。

协调者可以准备目标、发布启动数据、发起硬件启动并以 acquire 语义观察 family 状态，但不得
替目标实例提交 phase state 或发出 phase checkpoint。若某个目标入口只能 adoption 已由架构
建立的事实，adoption 仍属于该目标实例的 Preset，并必须在后续状态 checkpoint 前验证目标
identity、栈/任务指针等长期边界。

## 递归推进

父子驱动和同对象迁移链在每一级递归应用，形成嵌套的阶段推进：

```text
Parent.Preset
  drives ChildA.Preset
    ChildA.Preset commits Prepared, emits ChildA.Setup
    ChildA.Setup  commits Ready,    emits ChildA.Enable
    ChildA.Enable commits Online
  Parent.Preset commits Prepared, emits Parent.Setup
Parent.Setup
  drives ChildB.Preset
    ... ChildB commits Online
  Parent.Setup commits Ready, emits Parent.Enable
Parent.Enable
  drives ChildC.Preset
    ... ChildC commits Online
  Parent.Enable commits Online
```

如果一个父 transition 顺序驱动 `ChildC`、`ChildD`，`ChildC.Online` 后发生的是“返回父
transition 并执行下一条 `drives`”，不是 `ChildC` 直接触发 `ChildD`。这一规则使 model 可以
从任一父阶段向下展开，同时保持每一级状态提交和职责所有权。

## 条件与完成事实

- `depends_on` 描述 transition 被触发时必须已经成立的外部事实。父 `drives` 已唯一保证的
  结构顺序不需要伪装成 sibling 前驱关系；若 transition 还有独立入口或推导需要该事实，则
  应显式保留依赖。
- `ensures` 描述 transition 成功后成立的事实。父 transition 驱动完整子阶段时，应明确确认
  该子阶段到达 `Online`，以及父层真正依赖的其它结果。
- `invariant` 描述进入某状态后持续成立的事实。`Base` 不用于承载前一 sibling 的 `Online`
  条件；需要的前置事实属于触发该 transition 的 `depends_on`。
- `within` 只保护自己的词法 body。`drives`、`ensures` 和其它块按 model 源码顺序保留，不能
  为了生成方便统一前移或后移。
- `deferred` 必须留在实际未闭合的 transition 或对象边界，不得借父阶段 `Online` 隐式宣称
  deferred 能力已经实现。

## 清理路径

阶段可在主推进链之外定义清理路径：

```text
Cleanup: allowed source state -> Destroyed
```

清理路径只在 model 明确定义时存在。它不承担父 continuation、控制权移交或 sibling 推进
语义，也不能用 `handoff` 名称隐式替代。

## 生成检查

从 charter 生成每一级阶段 model 时，至少检查：

1. 阶段的 parent、职责和执行主体边界明确。
2. 标准四状态和三个迁移完整，或存在有理由的显式例外。
3. `Preset` 只 `emits` 本阶段 `Setup`，`Setup` 只 `emits` 本阶段 `Enable`。
4. 上下级关系由父 transition 的 source-ordered `drives` 明确表达。
5. 每个被驱动阶段的完成状态和父层所需事实由 `ensures`/invariant 明确确认。
6. context、执行主体交接和 deferred 边界没有被阶段树的视觉顺序掩盖。
7. replicated family 明确 target key、per-target owner、pointwise sibling 顺序和 family Online
   聚合查询，且没有引入隐式跨目标屏障。

本范式适用于 `Kernel` 驱动的各级标准阶段对象。普通资源对象仍遵循通用对象生命周期语义，
不因为被 Phase 驱动而自动成为阶段。
