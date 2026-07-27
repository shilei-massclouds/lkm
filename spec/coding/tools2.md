# tools2 Signal 工具链 Coding 规格

本文把 [`../model/SEMANTICS.md`](../model/SEMANTICS.md#sem-signal-tools2-001-full-model-signal-derivation-is-an-isolated-compatibility-semantics)
映射到完整主模型 Python 工具实现。它只约束 `tools2/`，不得改变或导入 `tools/`。

## 包与依赖边界

源码固定分为 `common`、`parse`、`model`、`derive`、`check`、`view`、`render`、`animate` 和 `pyveri` 九个
并列包，各包保留自己的 `pyproject.toml`。阶段入口沿用 `lkm-parse`、`lkm-model`、`lkm-derive`、
`lkm-check`、`lkm-view`、`lkm-render`，新增 `lkm-animate`，driver 仍为 `pyveri`；源码运行使用只包含 `tools2/*/src` 的独立
`PYTHONPATH`。任何 tools2 Python 文件不得 import `tools/`、老 `common` 或老 `pyveri`。

`common` 只承载 schema 常量、JSON I/O、source span/diagnostic、稳定 canonical JSON/fingerprint 和
跨阶段值校验。parse 不依赖后续阶段；model 只依赖 common 和 ast.json；derive 只依赖 common 和
model.json；check/view 只消费 derive.json；render 只消费 view.json；animate 同时消费 model.json 和
view.json。driver 通过各阶段公开 Python
入口按上述顺序调度，不越过中间协议直接拼装结果。

目录中的 `build/` 是可清理的保留中间产物目录，`out/` 是用户显式选择的长期输出目录；测试不得
依赖二者的预存内容。

## 完整主模型输入与诊断

parser/model 必须让 `spec/model/main.spec` 通过零 error、零 unsupported，并支持 include、嵌套泛型、
enum、type/object/system 与传递继承、属性、owned、association/reference、predicate/function、context/
lock、state、Type lifecycle/process、对象 lifecycle override、命名形参/result、`within`、结构化
`deferred`/`trimmed` evidence、`depends_on`、`ensures`、`updates`、`external`、`drives` 和 `emits`。v5 不接受
`lossy` 修饰；所有已发送 Signal 都必须被 handler 接受并处理。
model 必须把 `drives` 的顶层 `A || B` 规范化为有序 choice 节点；derive 在发送前检查候选可接受性，未选择候选不得进入 Signal 序列或消耗预算。
条件和 invariant 的顶层 `P || Q` 必须规范化为布尔 any-of 节点并按当前快照短路求值，不得保留为无法解释的 assertion 字符串。
`ensures` 的顶层 `P || Q` 同样保留源码顺序：提交时若某个分支已由 candidate snapshot 满足则保持该
分支，否则建立第一个分支；不得同时建立互斥 postcondition，也不得因 any-of 节点无法作为普通 fact
写入而中止完整闭包。
state/fact/reference 求值至少包括：

- `Target.state == State::Name` 与 state assignment；
- `predicate(arg, ...)` 的布尔事实和 predicate body 参数替换；
- `has_slot(instance.field, SlotKind::Member)` 按 model 中对象字段的声明类型逐段解析，并在该类型及其
  基类型的 `slots` 字段中验证对应槽位；该路径必须通用处理对象名、字段名和枚举成员，不使用主模型
  专用字符串表，成功的 condition event 使用 `proof_source: "model_structure"`；
- `Target.ref == Other` 与 reference assignment；
- enum literal、字符串、整数、布尔和系统引用 payload。

保留现有折叠调用形式，不接受显式 `signal` 或 `on Signal`。Type process、对象 override、静态
association/reference 和 effective parent 必须规范化为稳定 model 字段。未知 top-level/block member 或
未知表达式必须产生带 path、line、column 的 `unsupported` diagnostic；语法/引用/类型、unknown/self/
cyclic parent 矛盾使用 `error`。有 error/unsupported 时后续阶段仍可写出诊断 JSON，但不得执行推导。

完整模型允许唯一的 `external Human` 声明。external 不进入 object 表或 parent 树；其 `drives` 按源码
顺序同步执行并在首个失败处短路，全部成功后才把 `emits` 按源码顺序加入全局 FIFO。默认推导执行该
external 编排；显式 `--signal` 只发送该单个根 Signal。三个 Human Signal 都以 Human 为 source，彼此
没有共同 cause，`root_request` 指向第一个真实 Signal，而不是虚构的 `Human.Startup`。

## 中间协议

所有 JSON 顶层必须包含 `schema`、`version`、`producer: "tools2"` 和 `source`。schema 名沿用
`lkm.spec.ast/model/derive/check/view/snapshot`，tools2 各自使用 version `5`。消费者必须在读取后立即
验证三元组，不接受缺失 producer、老 producer、version 1/2/3/4 或其它 version。Signal/model/view JSON
不包含 `lossy`/`strict` 字段，outcome 不包含 `discarded`。

AST 保留 include 展开后的声明、source order 和每个声明/语句 span。model 建立 enum、system、parent、
state、handler、参数、条件、effect 和 call 索引，并输出 source 内容指纹。derive/check/view 字段遵循
formal semantics；快照统一包含 `states`、有序 `facts` 和 `references`，JSON 输出使用排序 key 和稳定
列表顺序。仓库内输入的顶层 `source`、include 路径和 span `source_file` 必须写为使用 `/` 的仓库
相对路径，仓库外输入保留绝对路径；model fingerprint 必须消费这些稳定路径以及完整 model 语义和
source span 内容，不得包含 checkout 绝对路径前缀。snapshot 继续使用既有字段，顶层 `source` 和
boundary provenance 内的 `source_file` 同样遵循该稳定路径规则，使同一 checkout 从不同 cwd 运行以及
不同绝对路径下的等价 checkout 产生相同 canonical bytes。

## CLI 与退出码

阶段工具都接受显式 `-o/--output`。parse 接收 `.spec`；model 接收 ast.json；derive 接收 model.json
以及可选的 `--signal Target.Name`、`-u/--until Target.Name`、`--source`、`--scenario`、`--max-depth`、`--max-breadth`；check 和 view
接收 derive.json；render 接收 view.json 且只实现 `--format text`；animate 接收 model.json、view.json
并原子写出 HTML。animate 必须先分别验证两个输入的 v5 schema/version/producer、source 与 model
fingerprint 身份，协议、身份、缺失端点或 I/O 错误返回 2 且不留下部分输出。

driver 接收 `.spec` 和同一组 derive 参数，另提供 `--snapshot-out`、`--work-dir`、文本 `-o` 与
`--html-out`。默认
source 是 `Human`，预算默认 `3/3`；`all` 解析为无限，整数必须非负。未给 work-dir 时使用
临时目录，给出时保留 `ast.json`、`model.json`、`derive.json`、`check.json` 和 `view.json`。只有 check
verdict 为 complete 或 reached 才原子写 snapshot-out；reached snapshot 写入 boundary provenance；
failed/bounded/until_signal_not_reached 或阶段错误时目标文件不得出现。

driver 必须在 model/view 成功产生后为 `--html-out` 调用 animate。HTML 与 text stdout/`-o`、scenario、
snapshot-out 和显式 work-dir 可同时使用；HTML 成功不得覆盖 check 的 0/1，animate 协议或 I/O 错误
把最终结果提升为阶段错误 2。shortcut 只透传同名参数，不复制动画生成逻辑。

仓库根可直接执行的唯一便利入口是 `tools2/bin/pyveri`；旧 `tools2/pyveri2` 必须删除。入口不得复制
阶段逻辑，只负责从自身路径解析仓库根、建立 tools2 独立
`PYTHONPATH` 并把短参数翻译给 driver：可选的 `-t/--trigger` 对应 `--signal`，`-u/--until` 对应
`--until`，`-s/--scenario` 对应 `--scenario`，`-f/--spec` 选择输入规格。`-t` 的默认值经规范化后是
模型中唯一的 `external` 编排，帮助 usage 必须显示 `[-t SIGNAL] [-u SIGNAL]`；`-f` 默认指向
`spec/model/main.spec`，快捷 source 默认 `Human`、预算默认 `all/all`；显式参数覆盖它们。预算、source、
work-dir、snapshot-out 和文本输出参数保持透传，底层 driver 的通用默认仍为 `3/3`。入口从脚本自身
位置解析仓库和包路径，因此从仓库根或其它当前目录调用的行为一致。

快捷入口必须保留 `-t` 是否由调用者显式给出的信息。只有显式 `-t/--trigger` 且没有显式
`-s/--scenario` 时，才以 common 规范化后的完整 Signal 名称查找
`tools2/scenarios/<CanonicalSignal>.snapshot.json` 并把该路径作为 `--scenario` 传给 driver；显式 `-s`
拥有最高优先级。候选路径 resolve 后必须仍位于 `tools2/scenarios/` 内，不能用 target/name、绝对路径、
`..` 或 symlink 越界。安全的候选不存在时，入口在启动 driver/derive 前返回 2，stderr 同时报告
canonical signal 和预期路径。`Target.Startup` 与 `Target.Preset` 选择同一文件。调用者省略 `-t` 时
不做默认场景查找，而从模型初态执行默认 Human 外部编排；因此 `-u Kernel.Enable` 仍从完整
上游链生成 snapshot。该查找只属于 shortcut，driver、derive CLI/API 和 scenario loader 不得复制它。

shortcut、driver 和 derive API/CLI 都调用 common 中同一 Signal request 规范化函数；任何 target 的
末段 `Startup` 都规范化为 `Preset`，其它名称不变。root/until request 和所有结构化产物只保留
canonical 名称，使 `Startup` 与 `Preset` 输入得到相同 Signal ID、事件序列和 canonical JSON。

derive 的发送实现必须通过单一 pre-send 函数创建 envelope。该函数接收已解析 source/target/name、
delivery、cause、coordinate 和 call span，先比较 canonical until request；匹配时记录 boundary 并在
任何 ID/event/enqueue/handler 操作前终止发送。内部使用专用控制流向上展开：尚未提交的 active response
写 `stopped` 和稳定 after snapshot，已完成 response 不改写；FIFO 中已有未处理 envelope 统一写
`stopped`，但不创建目标或余下 emits。该控制流不得被普通 DerivationProblem 捕获为 failed。

`drives` 同步子 Signal 和 `emits` FIFO Signal 使用相同严格处理：receiver、handler、payload、source
state 或 condition 拒绝，以及 handler body/invariant/下游 Signal 失败，都必须传播为最终 failed。
不得静默忽略、重试或转换为 discarded。`drives A || B` 只在发送前选择第一个可接受候选；未选候选
不创建 Signal。预算导致的 `truncated` 与显式 until 导致的 `stopped` 仍是独立非丢失结果。

默认 `tools2/bin/pyveri` 从主模型初态执行完整闭包时是验收场景：必须返回 0、check/view verdict
必须为 `complete`，且 Signal 列表不得包含 `rejected` 或 `failed`。canonical Kernel snapshot 续跑
同样必须完整成功。成功测试不得接受 `{0, 1}`；负向 fixture 继续精确断言返回 1、`failed` 及原因。
完整浏览器 fixture 的 Signal 与 moment 数由本次成功 derive 产物重建，不把历史 274/277 数量当成
协议常量。

derive 对已具名的符号引用执行 `==` / `!=` 时，必须比较引用值本身；例如
`CurrentTaskRef != KernelInitTaskRef` 可由两个同类型且名称不同的 `TaskRef` 值直接判定，不要求模型
另外制造 `assert:` fact。对象属性比较仍先读取 snapshot 中的 reference assignment；不能把“两个引用
恰好指向同一对象”误作“两个引用值相等”。

parse/model/derive 在成功写出合法诊断 JSON 时返回 0，I/O 或协议损坏返回 2。check 对 complete/reached 返回
0，对 failed/bounded/until_signal_not_reached 返回 1，协议损坏返回 2。view/render 不改变 check verdict；driver 最终采用
check 的退出码。所有用户错误写到 stderr，不输出 Python traceback。

## 展示边界

view 逐字段复制/整理 derive 的 root/until request、reached boundary、verdict、events、signals、snapshots、frontier 和 failure chain；
不得重新求值条件或通过名称推断 handler/outcome。`render_text(view)` 保持单参数调用接口，并在内部仅以
`os.environ.get("VERBOSE") == "1"` 选择 detailed renderer；render CLI、driver、shortcut 及 `-o` 输出
不得各自实现另一套选择。该环境变量只控制最终文本，不能传入或修改 parse/model/derive/check/view。

默认 compact renderer 首行输出 `verdict: VALUE`，随后按 `view["signals"]` 的原顺序输出：已解析
Transition 为 `SOURCE -- SIGNAL --> TARGET[BEFORE:AFTER]`，已解析 Action 为
`SOURCE -- SIGNAL --> TARGET`；未解析 handler 不猜类型，也不显示状态方括号。状态必须读取 target 在
Signal before/after snapshot 中的实际值。Signal 文本名 `Preset` 显示为 `Startup`。每行按
`coordinate.depth - min(0, min(coordinate.depth))` 使用两空格缩进，空 trace 不计算最小值；任何非
`completed` outcome 在同行追加 ` !! OUTCOME: REASON`。reached 另输出
`boundary: SOURCE -- SIGNAL --> TARGET (before send)`，failed 另保留一行 failure chain/reason。

verbose renderer 保持本轮修改前的详细格式和 canonical `Preset` 名称，以 cause depth 缩进 Signal，
明确标记 `drives wait`、`emits enqueue/dequeue`、payload、predicate proof source、effective context、
到达时快照来源、before/after state/fact/reference delta、reached boundary、stopped propagation、
reject、truncated coordinate、source span 和完整因果链。当前 tools2 version 5 view 已包含两种
renderer 所需字段，因此不得为文本模式升级协议或改写 view。
当前 tools2 render 不实现 DOT、SVG 或 HTML。交互 HTML 由独立 animate 包及 Svelte 5 + TypeScript
frontend 生成，不属于 `lkm-render --format text` 的格式分支。老静态 trace/SVG 任务继续保留，退役
老工具必须由用户另行决定。

## Animation v3 与确定帧

animate 必须按 v5 view `events[].sequence` 重放并投影 `lkm.spec.signal-animation` version `3` 因果时刻。
`signal_sent` 只验证 Signal 已发送，不生成 moment；`signal_received` 生成 `<signal-id>:request`。
completed/rejected/failed 的 `drives` 或同步根请求生成 `<signal-id>:feedback`，`emits` 生成
`<signal-id>:settle`；truncated/stopped/response_stopped 生成 `<signal-id>:terminal`。moment kind 只取
`request`、`feedback`、`settle`、`terminal`，最终结果独立记录在 outcome。request 的 transfer 为
source→target，feedback 为 target→source，settle/terminal 没有 transfer。每个 moment 还记录 event
sequence、Signal/cause identity、source/target/name/delivery、handler kind/name、reason 和 Transition
的真实 before/after state。
`trace` 同时记录 `total_signals` 与 `total_moments`；`moments` 与 `frames` 一一对应，frame 使用
`moment_id`。条件、invariant、FIFO、wait 和其它 evidence event 不生成 moment。

生成器从 `initial_snapshot` 开始重放，并为每个 Signal 验证 send→receive→terminal 的严格序列。
`signal_sent` 不 reveal 节点或改变 snapshot；`signal_received` 校验当前 snapshot 精确等于 Signal
`before_snapshot`，以这个稳定快照生成 request 并首次 reveal source/target。`response_completed` 校验
事件 before/after 与 Signal snapshot 一致并以真实 after 更新当前 snapshot；rejected/failed 的
feedback/settle 保留真实 reason 和稳定 snapshot。未收到的 truncated/stopped 只生成 terminal，不
reveal target 或改变状态。缺失、重复、乱序、未知 Signal、snapshot 不一致、delivery 分类冲突或未知
handler/outcome 都以协议错误失败。

初始 frame 不显示节点。节点只能在 request moment 首次出现；每个 request frame 累积当前 Signal 的
source/target 与必要祖先，外部端点可没有 model node，结构祖先没有 snapshot state 时必须保持
stateless。生成器以首次 receive 的 source、target reveal 顺序分配稳定 `first_seen`；feedback、settle
和 terminal 不 reveal 节点或改变序号。虚拟 `$root` 和任意 parent 的 `sibling_order` 都按该序号升序
生成，且所有 frame 都不得根据当前 Signal、层级视觉方向或端点关系重排。浏览器只由 frame 数据和
parent 链计算显示坐标；前后导航不得累积顺序。

reached boundary 复制到 trace 并在最终说明显示，但 before-send target 不得获得 Signal ID、moment、
可见节点或 `first_seen`；尤其 `-u Kernel.Enable` 不得创建 Kernel.Enable Signal/moment。

自包含 HTML 必须安全编码内嵌 JSON，阻止 `</script>`、`<!--` 等数据提前终止 script 内容；CSS、
播放器 JS 和 animation v3 数据均不得依赖网络。Svelte/TypeScript 源码、lockfile 与编译后的 JS/CSS
都纳入仓库，并提供确定 rebuild 和 stale-bundle 检查。

播放器从预生成 frame 恢复前后位置，不重新 derive 或逆执行。request moment 显示 source 到 target
的箭头和请求信息，并使请求到达的 target 抖动；即使 source 是 target 的祖先、箭头被隐藏，也不得
跳过 target 抖动。feedback 不画反向箭头，在响应阶段先显示 target 的 before state，再提交 frame 的
after state并闪烁 target；Action 同样闪烁但不伪造 state。settle 不画箭头或反馈闪烁，提交真实
after state并显示“目标内部处理完成”；异常 settle 仍显示红色错误效果和 reason。terminal 保持确定
frame 并显示 outcome/reason。
前端按 parent 链计算层级，外部根端点
按一级节点处理；奇数层 sibling 以底部对齐 row 从左向右排列，偶数层 sibling 以左边缘对齐的反向
column 从下向上排列，任意深度继续交替。stage 的布局原点位于左下角。节点 identity 固定在自身框
左下区域，children 区位于其上方；叶节点和 identity band 宽度约为
`clamp(11rem, 16cqi, 15rem)`，名称可换行，状态不拆分并整体换行。名称和状态使用同一个字号
`clamp()`，状态保持正常字重。父框只随当前可见 children 的真实 footprint 向上或向右扩展，不预留
未来空间；稳定 flex footprint 在碰撞时只把较晚 sibling 向右或向上推开，不得回推较早 sibling。

普通箭头比较 source/target 中心差，选择最近的一对水平或垂直相向边，覆盖左到右、右到左、下到上
和上到下；普通 path 只使用端到端的 `M … L …` 直线，起点位于 source 对应边，终点和箭头尖精确位于
target 对应边，label 位于两端中点。source DOM 节点递归包含 target DOM 节点且两者不相同时，播放
request 阶段不构造 SVG 箭头；该判定只使用当前 frame 的 DOM parent 包含关系，不修改 animation
v3、Python frame、model 或 derive，且不得跳过端点高亮、请求抖动、Transition/Action 响应、异常 reason、滚动或
导航。self、descendant 到 ancestor、同级和跨分支仍构造箭头。self Signal 从节点右侧中部到左侧中部，
控制点位于节点上方，弧高和横向 reach 按节点宽高计算；label 位于上弧外侧。`.frame-forest` 顶部
padding 必须提供有上下限的流式净空，至少容纳实时缩放自环及 label，避免桌面、移动端和深层节点被
stage 裁剪。播放器使用 animation frame 在 FLIP、父框膨胀、stage 滚动、window/元素尺寸变化期间重测
CSS pixel 坐标并更新 SVG；普通和错误 marker 精确使用 `markerWidth="5" markerHeight="4"`、对应
`refX/refY` 与 `M 0 0 L 5 2 L 0 4 z`，箭头尖仍落在 target 边界。这些坐标是浏览器实时测量结果，不是固定布局常量。树高度变化时补偿 stage
的 `scrollTop` 以保持左下锚点在 viewport 中稳定，前后 frame 都从目标布局重算相同滚动边界。失败类
outcome 使用红色虚线并显示 reason。Transition feedback 切换真实 state，Action feedback 只闪烁；
settle 不使用 feedback 闪烁，异常 settle 使用独立红色错误效果。
新增节点与向外推开使用 FLIP 平滑过渡，活动端点滚入视区；
`prefers-reduced-motion` 下取消非必要位移、抖动、闪烁与脉冲，但立即保留相同确定 frame、活动端点、
线型、reason 和导航。
控制面只含前后按钮、ArrowLeft/ArrowRight、时刻数、Signal 总数和当前 Signal 说明，不增加自动播放、
速度或时间线。存在 reached boundary 时，最后一个 moment 的说明同时显示其 before-send target。

frontend CSS 以 `%`/`dvh` 的外壳和 `auto minmax(0, 1fr) auto` 主 grid 建立有界流式尺寸体系；stage
不设固定高度而占满剩余空间，内部内容以 `min-content`、`fr` 和滚动容器保持可达。间距、圆角、字体、
卡片 padding 与节点最小宽度使用 `rem`/`ch`、viewport 或 container 比例及 `clamp()`，响应断点使用
`rem` 或 container query。桌面保持页面无纵向滚动并让 stage 占主要 viewport；窄屏允许页面纵向滚动，
但页面无横向溢出，必要的横向滚动局限于 stage。

header 只显示紧凑品牌、请求标题与四项同行 `label:value` 元数据，不渲染说明句或可见 footer；Protocol
tooltip 承载 source file 与 model fingerprint。普通 system identity 不渲染 `SYSTEM`，名称和裸状态名
或 `Stateless` 同行并使用完全相同的流式字号；名称可换行，状态保持正常字重、完整并整体换行。底部
Transition 响应也不得渲染
`State::`；JSON 中的状态值不变。External/Structure 继续渲染类型标签。transport 不设固定
最小高度，宽屏优先把时刻数、Signal 路径、响应/reason 和按钮同行，窄屏自然换行。
