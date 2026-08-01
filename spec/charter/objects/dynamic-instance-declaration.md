# 运行期实例声明

`declare` 是 `.spec` 控制流在执行期建立 fresh Type instance 的正式机制。它解决同一 fork、exec
或其它 action 声明点被多次执行时必须产生不同对象 identity 的问题；它不是静态 object 的简写，
也不是可复用具名 slot。

## 语法与执行位置

首版正式语法仅是 `drives` 内的顺序语句：

```text
drives {
    declare child of Task;
    declare child_flow of UserTaskFlow;

    child.Transition::Preset(...);
    child_flow.Transition::Preset(...);
}
```

owned collection 元素使用同一执行语义的 indexed declaration：

```text
owned {
    indexed cpus[key: LogicId]: CPU;
}

drives {
    declare self.cpus[0] of CPU;
    self.cpus[0].Transition::Preset;
}
```

indexed declaration 不建立词法 alias；canonical identity 就是完整 indexed path。key 必须满足字段
声明的类型，只有 collection owner 的 handler 可以插入，且重复 key 必须拒绝。child declaration、
同一父 handler 内随后的 child transitions、父状态变化和 invariant 共同形成一个 candidate；任一
步骤失败时整次 handler 不发布 child 或 key。这个原子发布规则只适用于 indexed-owned insertion，
不改变下述普通 fresh alias declaration 在后续失败时保留诊断实例的规则。

- `declare name of Type;` 只允许出现在 transition/action 以及其嵌套 `within` 的 `drives` 中；
  不支持顶层动态声明。
- 声明是有顺序的可执行语句。执行到该语句时才创建一个所属 Type 的 fresh、独立 instance；
  instance 初始 lifecycle state 来自最近声明该状态图的 Type；该 instance 随后使用与继承该 Type
  的静态 object 相同的完整状态、迁移、ensures 和 invariant。进入初始状态时，invariant 中的
  `self` 必须替换成这次声明产生的实际 runtime identity。
- 声明本身不调用 `Preset`，也不隐式调用任何其它 process。同一声明点每次执行都创建不同
  identity，不复用上一次实例，也不共享 lifecycle state、facts、transition commits 或 trace 节点。
- `declare` 不隐式建立 parent、owner、active binding 或集合成员关系；这些关系必须由后续
  lifecycle/action 和正式 facts 显式建立。

## Alias 与词法 SSA 作用域

声明中的 `name` 是引用 runtime identity 的词法 alias。它与 `let` 和过程参数共享同一个 SSA
命名空间，并服从下列硬规则：

- 同一可见作用域禁止重复声明、shadow，以及与可见静态 object 重名；alias 必须先声明后使用。
- alias 从声明语句之后开始，对当前过程的后续语句以及之后进入的嵌套 `within` 可见。
- 嵌套作用域内声明的 alias 只对该嵌套作用域从声明点开始的后续语句可见，不反向泄漏到父
  作用域，也不对已经结束的 sibling scope 可见。
- callee 不捕获 caller alias；callee 只能通过显式 typed process 参数接收 instance 引用。
- 未知 Type、接收者 Type 上不存在的 process、参数 Type 不匹配和非法 lifecycle 迁移都必须
  在检查或推导边界报告错误，不能退化成静态名称查找或字符串分发。
- object 若需要偏离 Type lifecycle，只能显式声明完整 override；禁止以 object 局部状态或迁移
  与 Type 状态图合并。运行期 `declare` 不具有 object declaration，因此不能建立实例专用 override。

## Instance 存活与重新定位

alias 作用域结束不销毁 instance。只有该 instance 显式成功完成 `Disable/Cleanup` 等适用
lifecycle，才退出其生命周期；声明后任一后续语句失败都不获得隐式 rollback 或 Cleanup。

运行期 instance 可以通过 ownership facts、集合成员 facts、Action result 返回的 typed `Ref`，
或后续 process 的显式参数持久化和重新定位。词法 alias 不是全局名称，不得作为跨调用、跨 trace
或持久集合的 identity。Task、Flow 等具有所有权约束的 Type 必须用正式 facts 证明归属；两个
Task 不得因 alias 重用或 Ref 误绑定而共享同一 Flow。

## 静态模型与运行期 identity

静态 object model 继续只枚举源码中的具名 objects。声明语句作为 declaration site/template
保存在 owner process 的有序语句中；runtime instance 只在 derive/trace 执行到声明点时产生，
不得写回静态 object 集合。

每个 runtime instance 的稳定 trace identity 由以下部分组成：

1. 根调用路径；
2. owner process identity；
3. 声明点在 owner process 中的稳定源码语句序号；
4. alias；
5. 该调用路径下该声明点的 occurrence。

源码行号只用于诊断位置，不参与 identity。因而在声明点之前插入空行或移动无关源码行，不会改变
已有 runtime identity；同一声明点在同一调用路径下的多次 occurrence 则必然得到不同 identity。
调用路径、declaration site、runtime identity 和 static object 必须在 JSON/trace 中使用不同字段，
不能把其中任意两者折叠成同一字符串名称。

## View 与 render

静态 view/render 展示 owner process 中的 declaration site、alias 和 declared Type，不伪造运行期
object。derive/trace view 展示每次实际创建的 runtime instance 及其稳定 identity、独立 state 和
后续 process 节点；多个 occurrence 不得折叠成一个 alias 节点。诊断可以附带源码文件和行号，
但展示层不能用行号替代稳定 identity。

## 当前范围

当前支持已有顺序 process/`within` 控制流和 indexed-owned insertion，不新增循环语法、并发声明
调度或通用垃圾回收；也不同时引入其它 Signal 新语法或全仓 Stream -> Flow 迁移。
