# 阶段范式

阶段（Phase）是内核生命周期的基本编排单元。每个阶段服从统一的状态模型，使系统各层级的建立过程具有一致的可推导结构。同级阶段之间可以串接，上下级阶段之间可以嵌套。

## 标准生命周期

```
Base --[Preset]--> Prepared --[Setup]--> Ready --[Enable]--> Online
```

### 状态定义

| 状态 | 含义 |
|---|---|
| Base | 阶段对象已实例化，但尚未收到启动事件。前置条件应在 Preset 的 `depends_on` 中声明。 |
| Prepared | 阶段已收到启动事件，完成初始检查和前置条件确认，准备工作完成。 |
| Ready | 阶段已完成必要内部建立，具备进入正式运行状态的条件，但尚未真正使能。 |
| Online | 阶段已使能，其提供的功能或服务可被上层或其他阶段依赖使用。 |

### 迁移定义
迁移对调用者/发起者来说是事件，对于对象来说是响应事件的过程，过程中完成内部动作，把状态推进到下一步。阶段对象同样符合上述定义。
`Preset`迁移是阶段的启动事件。

| 迁移 | 语义 | 典型行为 |
|---|---|---|
| Preset | 启动事件，初始准备，驱动阶段进入 Prepared | 检查依赖条件，执行初始检查，自动向Prepared状态发出Setup迁移请求 |
| Setup | 构造事件，建立内部框架，进入 Ready | 基于 Prepared 阶段的成果，建立内部组织关系，向Ready状态发出Enable迁移请求 |
| Enable | 上线事件，收尾本阶段任务，进入 Online | 启动阶段涉及对象的服务或功能，使它们对外可用 |

每个迁移通过向目标状态 `emits` 下一级迁移事件，在到达目标状态后再次触发迁移过程，形成连续的链式状态驱动。

## 串接关系

对于阶段范式，同级阶段之间可以前后自动串接。

前一阶段的 `Enable` 完成、提交 `Online` 状态后，通过 `emits` 发出后一阶段的 `Preset`。
`Online` 和后一阶段 `Base` 是两个不同对象各自的状态，不发生状态重叠。后一阶段是否还需要在
`Preset.depends_on` 中检查前一阶段 `Online`，取决于父阶段 `drives` 和 `emits` 链是否已经
唯一保证顺序；若为了非确定性推导或外部入口仍需该事实，应显式保留检查。

```
Level N-1 Enable commits Online --emits--> Level N Preset
Level N   Enable commits Online --emits--> Level N+1 Preset
```

串接关系的具体粒度取决于实际编排时序：父阶段的驱动链（`drives`）保证阶段顺序，`depends_on` 只在模型非确定性推进需额外约束时使用。不加 `depends_on` 的 Preset 意味着父阶段结构已足够保证排序。`invariant` 不用于 `Base` 状态，因为初始状态无法匹配尚未到达的 predecessor Online 条件。

## 清理路径

阶段在其主推进链之外保留可选的清理路径：

```
Cleanup: 任一状态 → Destroyed
```

清理路径可从 Preset、Setup 或 Enable 的失败处理进入，也用于阶段退出或资源释放。

## 适用范围

本范式适用于所有标准阶段对象。轻量级阶段可为 Preset、Setup 或 Enable 定义空操作，但状态机结构保持一致。
