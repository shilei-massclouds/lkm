# 阶段范式代码映射

本文定义 `spec/model` 中阶段对象到 impl 的权威自然语言映射。charter 如何生成阶段 model 见
[`spec/charter/phase-paradigm.md`](../charter/phase-paradigm.md)，通用对象、transition、状态
和 checkpoint 规则见 [`mapping.md`](mapping.md)。本文只 lowering 已经存在的 model，不补写
缺失的生命周期、`drives`、`emits` 或 sibling 顺序。

## 输入语义

实现前必须从 model 逐项读取：

- 阶段 parent、初始状态、每个 transition 的 source/target state；
- source-ordered `depends_on`、`drives`、`within`、`ensures` 和 `deferred`；
- 目标状态 invariant；
- transition 提交后的 `emits`。

标准阶段的 `Preset`、`Setup`、`Enable` 由同对象 `emits` 连接。父子阶段关系由父 transition
中的 `drives` 建立；同级阶段之间没有隐式调用边。若 model 与该规则冲突，应先回到 charter
和 model 审计，不能在 coding 或 impl 中自行选择一条顺序。

## 源码边界

Phase 在 impl 中映射为过程 module，不映射为普通资源对象式 `struct + impl Lifecycle`。每个
阶段 module 必须提供以下可定位边界，具体名称由对应阶段 coding 文件记录：

- `preset`、`setup`、`enable` 的 start 边界；
- 有子阶段或控制权交接时，各 `drives` 后的 completion continuation；
- 四状态的读取、源状态检查和目标状态提交；
- model 规定的依赖、后置条件、invariant 和 checkpoint；
- 返回父 transition 的 continuation，或已规格化的不返回终点。

composite phase 可以使用轻量状态和 continuation 函数，不要求一个同步栈帧包裹完整子树；
但它不能只靠子阶段的视觉调用顺序代表自己的 transition。叶子 phase 同样保留三个 transition
和四状态，只是其 `drives` 通常落到普通对象 transition/action。

replicated phase family 必须把 target key 映射为独立状态槽，例如由 AP 单写、协调 CPU 只读的
`[AtomicU8; MAX_CPUS]`。每个实例仍由过程 module 的 `preset`/`setup`/`enable` 推进，不得退化为
协调者持有的单个 `Lifecycle` 聚合对象。family 查询可以提供 `state_for(key)` 和
`all_online(target_set)`，但 `all_online` 只能聚合读取已提交状态，不能代写状态或补发 checkpoint。

## Transition lowering

每个阶段 transition 按以下顺序映射：

1. **检查 source state**：确认 owner 仍处于 model 声明的源状态，并拒绝重复或越级触发。
2. **检查 `depends_on`**：使用对象状态查询、结构化事实、构建期证明或明确的架构 adoption。
3. **记录开始边界**：若该阶段定义 `Phase.Started`，在 Preset 被接受且当前边界可执行的检查
   完成后发出一次；它不是状态提交。
4. **执行 transition body**：严格按源码顺序 lowering `drives`、`within` 和相关检查。不得把
   不同 context 的动作合并、前移或后移。
5. **确认完成事实**：检查被驱动 transition 的结果和可在提交前验证的 `ensures`。失败、
   `Blocked` 或未满足事实都不得继续提交 owner 状态。
6. **提交 target state**：把阶段状态从 source 原子地或单写者地更新到 target。checkpoint
   不能代替这次状态写入。
7. **检查 target boundary**：确认持久状态读回为 target，并验证该状态 invariant。若检查失败，
   必须 fail-stop 或回滚，且不得执行 `emits`。
8. **发出完成 checkpoint**：在目标状态已经写入并通过边界检查之后发出已声明的 state
   checkpoint。
9. **lowering `emits`**：按 model 顺序触发 completion event。标准阶段中，Preset 提交
   Prepared 后调用本阶段 Setup，Setup 提交 Ready 后调用本阶段 Enable。

架构入口太早而无法完成全部 Rust 检查时，可以先用稳定汇编编码记录 `Started`，再在最早可行
的 continuation 完成 state/dependency adoption。对应阶段 coding 文件必须记录哪些检查前置、
哪些延后以及为何安全；Rust continuation 不得重复发出同一 checkpoint。

## 状态设置与检查

每个标准阶段都必须有可长期读取的 `Base`、`Prepared`、`Ready`、`Online` 状态表示。允许使用
静态原子值、单写者 cell、集中 phase-state registry 或等价机制，但必须满足：

- 初始化值确实为 `Base`；
- 每个 transition 入口检查精确 source state；
- 只有 transition 成功路径写入精确 target state；
- 查询函数名与含义一致，例如 `is_online()` 只能在实际 `Online` 时返回 true；
- 不得把 model `Online` 长期命名或记录为 impl `Ready`；
- checkpoint、函数返回和调用顺序都不能替代状态存储与状态检查。

如果受早期汇编或地址空间切换限制，状态写入可以分为“稳定早期事实 + Rust adoption”，但 coding
文件必须给出唯一 owner、adoption 时点和重复执行保护。该例外不允许跳过中间状态。

## 父子 continuation

父 transition 始终拥有自己的 `drives` 序列。驱动子阶段时，impl 的逻辑形状为：

```text
parent_transition_start
  -> child_a.preset
     -> child_a.setup
        -> child_a.enable
  -> parent_transition_after_child_a
  -> child_b.preset
     -> child_b.setup
        -> child_b.enable
  -> parent_transition_after_child_b
  -> parent target-state commit/checkpoint
  -> parent emits
```

`parent_transition_after_child_a` 可以由普通函数返回、显式 callback、保存后的 continuation、
任务切换恢复点或架构 handoff 实现。无论物理控制流采用哪种形式，它都属于父 transition；
`child_a.enable` 不得直接拥有或命名 `child_b.preset` 的 sibling 推进语义。

如果 continuation 跨 task、CPU、异常流、栈或地址空间，必须在移交前持久化足够的 owner state
和 continuation identity，并由 model 中的 entry/release/dispatch/completion/handoff 事实支撑。
新执行主体上的恢复点继续执行父 `drives` 的下一项，而不是创建第二条未建模阶段链。无限 idle
或服务循环只执行自己所属任务的运行期职责，不能因为物理上仍持有旧栈帧而继续父阶段。

replicated siblings 的 continuation 按 target key 绑定：`ChildA[key].Online` 只能恢复同一个
`key` 的父 continuation 并触发 `ChildB[key].Preset`。不同 key 可以并发或交错；协调者只有在
model 要求 family completion 时才用 acquire load 等待所有目标 Online。AP/设备等远端 owner
提交状态时使用 release 或 AcqRel，协调者不得用普通非同步字段推断 completion。

## `emits` lowering

标准阶段的 `emits` 只连接本阶段迁移：

```text
Preset commit Prepared -> setup_start()
Setup  commit Ready    -> enable_start()
Enable commit Online   -> return/invoke parent continuation
```

最后一行不是隐式 `emits`；它是先前父 `drives` 的完成返回。若 model 显式存在其它 completion
event，impl 必须按 model 单独 mapping，但不得从阶段树位置猜测 cross-object emit。

`handoff()` 不是 `cleanup` 的别名。只有 model 明确定义 Cleanup 时才实现清理迁移；正常父
continuation、任务调度移交和资源销毁必须分别命名和映射。

## Checkpoint mapping

阶段状态与 checkpoint 是两类机制：状态承载语义，checkpoint 只承载稳定观察时点。

- `Phase.Started` 观察 Preset 被接受；它发生时 owner 仍处于 `Base`，且必须在任何被驱动子阶段
  的 `Started` 之前。
- `Phase.Prepared`、`Phase.Ready`、`Phase.Online` 若进入 checkpoint inventory，必须分别在
  对应 target state 已写入、读回并通过 invariant 后发出，名称不得错配其它状态。
- 每个阶段 coding 文件都必须对四个状态点写明“状态设置/检查落点”和“checkpoint 名或无”。
  即使某状态没有独立 checkpoint，也不能省略状态提交。
- 对后续阶段形成依赖、发生执行主体/栈/地址空间交接、用于 Linux 差分或包含长驱动链的状态
  边界，`SHOULD` 提供独立 checkpoint；省略时必须说明已有哪一个稳定边界足以观察。
- transition 内可以增加长期有用的对象动作、context 或 handoff checkpoint，但它们必须由
  对应 owner 发出，且不能使用尚未提交的阶段状态名称。
- checkpoint handler 不改变状态、不触发 `emits`、不成为依赖。失败诊断也不能伪装成成功
  checkpoint。
- replicated family 的 Started 和三个 target-state checkpoint 由每个 target owner 分别发出，
  因此同名 checkpoint 可以重复；owner 标签必须包含稳定 target identity。协调者的 all-online
  barrier 和 ack 使用自己的 checkpoint，不能复用 family state checkpoint。

## 失败与不返回路径

任一 `depends_on`、`drives`、`ensures` 或 invariant 失败时，当前 transition 不得发出成功
checkpoint 或执行 `emits`。错误处理应保留 owner、source state、失败 predicate 和最后完成的
drive/continuation，供长期 failure diagnostic 使用。

Payload、idle 或服务循环可以形成不返回路径，但必须先完成其 model 要求的状态提交和父
continuation。若某个子阶段按设计永不完成，父 transition 就不能同时宣称后续 `drives` 或
`Online` 已完成；这类冲突必须回到 charter/model 处理。

## 阶段 coding 文件要求

每个 system/phase 专题 `.md` 至少包含下表信息：

| 项目 | 必须记录的映射 |
| --- | --- |
| transition | source/target state 与 start 函数 |
| depends_on | 具体检查或 adoption 落点 |
| drives/within | source order、被调用函数、context 边界 |
| continuation | 每个 child/控制权交接后的父恢复点 |
| ensures/invariant | 提交前后检查及失败行为 |
| state | Base/Prepared/Ready/Online 的存储、设置和查询 |
| checkpoint | Started 和各 target state 的 checkpoint 或省略理由 |
| emits/return | 同对象下一迁移或父 continuation |

实现例外必须写明原因、生效范围和仍被保留的 model 边界。阶段专题文件不得用一张平坦调用链
图取代父 transition 所有权和状态提交说明。

## 验证

阶段映射变更至少需要：

1. 检查 model 可达链中 `drives`/`emits` 的顺序和 owner。
2. 通过阶段状态与 checkpoint inventory 核对名称、提交时点和重复发出。
3. 对控制权交接使用 focused run/checkpoint 证明实际 task、CPU、栈和 continuation。
4. 执行仓库规定的 focused gate，并最终执行根目录 `make test`。

运行结果只证明实现路径，不反向修改 model 语义。若 trace 与本规则冲突，应先定位边界，再按
charter -> model -> coding -> impl 顺序修正。
