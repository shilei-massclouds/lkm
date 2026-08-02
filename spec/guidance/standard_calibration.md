# 标准校准

“标准校准”是显式触发的、按系统执行的加强版 `charter-first` 工作流。它用于根据参考 Linux 的
顶层调用顺序逐项复核候选系统边界，再把确认后的 formal 系统严格闭合到
`Charter -> Model -> Coding -> Impl`。它是 Guidance 支撑流程，不改变核心语义权威顺序，也不把
Linux 源码、函数或调用边界本身提升为系统设计。

只有用户明确要求执行“标准校准”时才进入本流程。普通 `charter-first`、Linux 对照、启动审计或
相邻系统修改都不构成隐式触发。当前
[`BootInitFlow` 顶层函数/系统表](../charter/phases/boot-init-flow.md#start_kernel-到-setup_arch-返回的顶层函数系统清单)
及后续同类表只作为待逐项复核的校准清单，不表示其中的候选系统名称、边界或分类已经通过标准校准。

## 推进单位与顺序

标准校准默认按参考 Linux 的真实顶层调用顺序推进。每次先处理当前 Linux 顶层函数的候选映射，再
进入下一项；若多个调用属于同一个 formal 系统，必须合并为一次系统校准，在同一个系统边界内完整
审查这些调用，不得按调用次数重复创建系统或拆分提交。

formal 系统是规格闭合和提交单位。trimmed 与 deferred 条目不是 formal 系统，不创建本流程规定的
四层 owner 文件；它们以单个 Linux 顶层函数为分类审计单位。

## 每个系统的基线

审查每个候选 formal 系统前，必须建立并记录该系统自己的可复现基线：

1. 记录本仓库和 sibling Linux 仓库各自的提交 ID，并确认两个工作树均干净；工作树不干净时停止，
   不得把既有修改混入校准结果。
2. 记录 sibling Linux `.config` 的路径、内容校验值和与本系统相关的配置值；未出现在 `.config` 中的
   布尔选项必须按未启用处理。
3. 记录支持候选映射与分类的 Linux 源码证据，包括实际调用点、所选架构或配置分支、实现或空实现
   定义，以及必要的调用链上下文。Linux 证据只用于决策和验证，不成为 Coding 中的跨实现映射。
4. 修改前在本仓库根目录直接运行 `make test`，记录完整结果并确认通过。不得用 focused test、shell
   包装、管道或重定向替代这一基线门禁。

即使连续系统共享同一仓库提交或 `.config`，也必须在每个系统的交付记录中重新确认并记录这些
基线事实；可以引用同一份已展示证据，但不能省略该系统的基线结论。

## 候选映射审查

基线通过后，先审查清单中的候选分类，不得直接按照现有名称或分类创建文件：

- **trimmed**：只核对当前配置、架构、参考输入或编译期裁剪使该调用为空或不产生实质工作的证据，
  并核对稳定 boundary ID、category、evidence 与 `revisit_when` 等 structured inventory 记录一致。
  trimmed 不是未实现功能，不得据此生成运行时 stub 或 formal 系统。
- **deferred**：只核对当前配置下真实调用仍存在，并核对候选责任、分类理由、稳定 boundary ID、
  evidence 与 `close_when` 等 structured inventory 记录一致。不得把未确认语义补成 formal 设计。
- **formal**：先重新确认系统对外功能、边界、相关系统、信号、状态迁移与动作责任。现有表中的系统名
  只是候选；边界不合理时，必须先向用户提出合并、拆分、改名或降为既有 owner system action 的建议，
  不得机械接受，也不得从当前目录结构或实现反推设计。

formal 候选只有在系统边界审查完成后才能进入第一个人工门禁。若候选应与稍后出现的调用合并，先
收集属于同一系统的全部调用证据，再作为一个系统进入门禁；Linux 调用顺序仍须在系统内部保留。

## 第一个人工门禁：路径与 Charter

对每个 formal 系统，执行者必须先询问并取得用户指定的相对路径 `P`。`P` 不含四层根目录和扩展名，
每个路径分量必须是小写 snake_case；执行者不得根据现有目录、Linux 名称或候选系统名自行推断 `P`。

取得 `P` 后，执行者必须向用户提交该系统完整的自然语言 Charter 草案。草案至少说明系统功能与
边界、相关系统、接受和发出的信号、状态/迁移/动作、关键不变量、失败边界，以及 Linux/config 证据
在决策中的作用。草案必须把系统语义与跨实现证据分开，不得把 Linux 函数或源码符号写成系统的
规范接口。

用户明确确认 Charter 草案前：

- 不得把草案写入仓库；
- 不得创建或修改 Model、Coding、Impl、Compose 或 Testing；
- 不得把用户指定 `P`、对候选边界的讨论或普通 `charter-first` 请求视为草案确认。

如果目标 Charter 已列入 `charter-locks.json`，草案确认也不构成解锁授权。执行者仍须针对
`spec/charter/P.md` 单独取得明确解锁授权，并只使用 `python3 tools/charter_lock.py unlock PATH` 解锁；
完成获准修改后，任务结束前必须使用 `python3 tools/charter_lock.py lock PATH` 刷新哈希并恢复锁定。

## 四层 owner 与闭合

Charter 草案经明确确认，并在适用时取得具体锁文件的解锁授权后，固定生成以下四个同径 owner 文件：

```text
spec/charter/P.md
spec/model/P.spec
spec/coding/P.md
impl/arceos_ex/src/P.rs
```

四层中的 `P` 必须完全相同，只有扩展名随层变化；目录与文件基名都使用小写 snake_case。每一层中
属于该系统的内容必须集中在该层唯一的 owner 文件，不得把系统语义或实现拆到 helper 文件。必要的
注册、装配和测试旁支可以引用 owner，但不得重新定义系统功能、边界、状态、动作或不变量。

随后严格按以下顺序闭合，不得并行跳层：

1. **Charter**：只落盘用户已确认的自然语言设计，并记录“已修改”或“已审查、无需修改”。
2. **Model**：只形式化和精确化 Charter，记录同样的闭合结论；不得改变 Charter 含义。
3. **Coding**：只约束 Model 到数据结构、算法、内存布局、寄存器等代码表示的映射，记录闭合结论；
   不得维护 arceos_ex 与 Linux/ArceOS 的源码、函数、符号或 checkpoint 映射。
4. **Impl**：只实现前三层共同要求的效果，记录闭合结论；不得从现有代码重新解释规格。

适用的 Compose 装配与 Testing 验证必须分别审查并记录结论，但它们是旁支，不加入或覆盖核心闭合
顺序。任何下层工作发现 Charter 缺口、歧义或新可观察行为、接口、对象边界时，必须立即停止向下
闭合，返回自然语言 Charter 草案和第一个人工门禁；不得静默扩写已确认 Charter，也不得让 Model、
Coding、Impl、Compose 或测试代替用户决定。

临时跨层不一致只能留在未提交工作树。一个 formal 系统的四层和适用旁支闭合完成前，不得开始
另一个 formal 系统的落盘修改。

## 验证、展示与第二个人工门禁

四层闭合后，先运行该系统适用的专项规格检查、focused tests 和运行时验证；只要本轮包含代码变更，
最终必须在仓库根目录直接运行 `make test`。还必须运行适用的锁检查和 `git diff --check`。

提交前必须一次性向用户展示：

- 四个 owner 文件及适用注册、装配、测试旁支的实际 diff；
- Charter、Model、Coding、Impl 各自“已修改”或“已审查、无需修改”的闭合结论，以及 Compose、
  Testing 的独立审查结论；
- 两个仓库提交与干净工作树、sibling `.config` 和 Linux 源码证据；
- 全部专项验证和根目录 `make test` 的实际结果。

这是第二个人工门禁。用户再次明确确认并授权提交前，不得提交。提交必须保持“每个 formal 系统一
提交”：一个提交只包含一个系统的四层闭合及其必要注册、装配和测试，不得混入其它 formal 系统或
无关修改。多个 Linux 调用合并到同一 formal 系统时，它们随该系统形成一个提交，不得按函数拆分。

## trimmed / deferred 分类审计

trimmed 或 deferred 条目不进入 formal 系统的两个门禁，也不创建上述四层 owner 文件。每次只以一个
Linux 顶层函数为单位核对和更新校准清单及适用 structured inventory，保留确切配置、架构、调用点、
空实现或真实实现证据，并记录 Charter、Model、Coding、Impl 以及适用 Compose、Testing 的
“已修改”或“已审查、无需修改”结论。

执行者必须展示该单项分类审计的实际 diff、Linux/config 证据和适用验证结果，取得用户明确确认与
提交授权后，才可提交该 Linux 顶层函数的分类审计。分类审计提交不得夹带 formal 系统文件、其它
Linux 顶层函数或运行时行为实现。
