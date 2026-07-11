# 阶段范式

阶段（Phase）是内核生命周期的基本编排单元。每个阶段服从统一的状态模型，使系统各层级的建立过程具有一致的可推导结构。

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

| 迁移 | 语义 | 典型行为 |
|---|---|---|
| Preset | 启动事件，驱动阶段进入 Prepared | 检查依赖条件，驱动子阶段，发出 Setup |
| Setup | 建立内部框架，进入 Ready | 基于 Prepared 阶段的成果，完成内部组织关系，发出 Enable |
| Enable | 使能上线，进入 Online | 启动实际服务或功能，使阶段对外可用 |

每个迁移通过 `emits` 自动推进到下一迁移，形成连续的链式驱动。

## 串接关系

阶段的 Online 状态构成后续阶段的串接条件。后续阶段通过 Preset 的 `depends_on` 中检查前一阶段对象的 `Ready` 或 `Online` 状态来实现。`Base` 状态在模型初始即为真，不设 `invariant` 检查——前置条件的验证推迟到 Preset 决策点：

```
Level N-1 Online → Level N Preset
Level N Online   → Level N+1 Preset
```

串接关系的具体粒度取决于实际编排时序：父阶段的驱动链（`drives`）保证阶段顺序，`depends_on` 只在模型非确定性推进需额外约束时使用。不加 `depends_on` 的 Preset 意味着父阶段结构已足够保证排序。`invariant` 不用于 `Base` 状态，因为初始状态无法匹配尚未到达的 predecessor Online 条件。

## 清理路径

阶段在其主建立链之外保留可选的清理路径：

```
Cleanup: 任一状态 → Destroyed
```

清理路径可从 Preset、Setup 或 Enable 的失败处理进入，也用于阶段退出或资源释放。

## 适用范围

本范式适用于所有标准阶段对象。轻量级阶段可为 Preset、Setup 或 Enable 定义空操作，但状态机结构保持一致。
