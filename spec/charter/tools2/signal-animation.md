# tools2 Signal 动画章程

## 目标与责任边界

tools2 必须能够把已经确定的 Signal trace 发布为单文件、可离线打开的交互 HTML，使读者可以按
Signal 顺序观察发送者、接收者、响应类型、真实状态变化和失败原因。该能力属于 tools2 的展示与
发布责任，不是新的 model 语义、derive 输入或浏览器 runtime。

动画采用独立 `animate` 阶段，且只消费 tools2 version 6 `model.json` 与 `view.json`。model 提供
System identity、有效 parent、状态集合和静态结构，view 提供 derive 已经确定的 event sequence、Signal、
source/target、handler、outcome、reason 与 before/after snapshot。animate 不重新求值 guard、选择
choice、创建 Signal、重跑 handler 或逆向执行 transition。

## 因果时刻

- `signal_sent` 只作为已经发送的协议证据，不直接生成视觉时刻。收到 `signal_received` 时生成
  `<signal-id>:request`；只有此时才揭示 source、target 与必要结构祖先，并使用接收时的稳定 before
  snapshot。已经发送但从未接收的 Signal 不得生成 request，也不得提前揭示目标。
- `drives` 与同步根请求完成、拒绝或失败时生成 `<signal-id>:feedback`，表示同一 Signal 的目标响应从
  target 返回 source；它不是第二个 Signal。`emits` 完成、拒绝或失败时生成 `<signal-id>:settle`，表示
  目标内部处理完成，不伪装成发送源等待或接收的反馈。
- `truncated`、`stopped` 或 `response_stopped` 生成 `<signal-id>:terminal`。如果 Signal 此前已经收到，
  它仍保留 request moment；如果尚未收到，则只有 terminal moment且不揭示目标或改变状态。
- moment kind 只取 `request`、`feedback`、`settle` 或 `terminal`；最终 `completed`、`rejected`、
  `failed`、`truncated`、`stopped` 继续由独立 outcome 字段记录。request 携带 source→target transfer，
  feedback 携带 target→source transfer，settle/terminal 不携带 transfer。
- 所有 moment 严格按 v8 view event `sequence` 排列。同步 `drives` 子 Signal 的 request/feedback 位于父
  feedback 之前；异步 `emits` 仍保持 derive 的 post-commit 与全局 FIFO 顺序。条件、invariant、FIFO、
  wait 和 handler 事件保留为因果证据，但不生成视觉时刻。
- 初始帧为空。request 按 source、target 首次接收顺序分配 `first_seen` 并补齐必要结构祖先；
  feedback、settle 和 terminal 不得改变 `first_seen` 或 sibling order。
- feedback/settle 的完成快照采用真实 terminal after snapshot；因此同步父响应只在全部同步子 feedback
  之后提交，异步目标在 settle 提交。rejected/failed 保留稳定 snapshot 和 reason，不伪造状态变化。
- `signal_received` 必须证明当时重放 snapshot 与 Signal `before_snapshot` 一致；完成事件的 before/after
  必须与 Signal snapshot 一致。缺失、重复、乱序、snapshot 不一致、delivery 与时刻分类冲突或 outcome
  与终止事件不匹配都是动画协议错误。
- 前进播放当前因果时刻；后退直接恢复前一个预生成 frame，不执行任何模型行为。
- 任意前进、后退和重复往返必须恢复相同的可见 System、parent 结构、状态、失败事实与同级顺序。

Transition 响应展示 target 的真实 before/after state；Action 不伪造 lifecycle state 变化。异常
outcome 必须保持原 outcome 和结构化 reason。没有自身 snapshot state、仅为补齐 parent 链而显示的
祖先是结构容器，不得获得伪造状态。外部 `Human` 等不属于 model parent 树的端点仍可作为独立端点
显示。

## 发布协议与播放器

animate 输出内嵌 `lkm.spec.signal-animation` version `3` 的自包含 HTML。animation v3 是由 v8 输入
确定生成的播放协议：`moments` 与 `frames` 一一对应；moment ID 固定为 `<signal-id>:request`、
`<signal-id>:feedback`、`<signal-id>:settle` 或 `<signal-id>:terminal`，kind 与 ID 后缀一致。
每个 moment 保留 event sequence、Signal/cause identity、source/target/name/delivery、handler、
outcome/reason 和 Transition before/after；每个 frame 以 `moment_id` 关联时刻。trace 同时记录
`total_signals`、`total_moments` 和 reached boundary。JSON 必须安全内嵌，HTML 必须原子写入；协议或
I/O 失败不得留下半成品。

播放器只负责把确定 frames 布局并呈现。父子包含高于视觉方向，嵌套节点始终位于父框内；所有
parent（含虚拟 `$root`）的 sibling order 都按首次出现序稳定升序排列，不得根据当前 Signal 重排。
前端从 parent 链计算层级，并按 parent 深度交替 sibling 方向：一级节点从 stage 左下角向右排列，
二级节点从 parent 内左下角向上排列，之后奇数层继续向右、偶数层继续向上。外部 `Human` 等根端点
按一级节点参与同一布局。每个节点的 identity 固定在自身框左下区域，children 位于 identity 上方；
父框只随已经可见的子树向上或向右扩展，不为未来节点预留空间。较早分支增长造成碰撞时，只把
较晚出现的同级分支向右或向上推开，不得向左、向下回推、移动较早锚点或改变首次出现顺序。

request 使用 source 到 target 的直线箭头；播放器根据两端中心差选择最近的一对水平或垂直相向
边，支持左到右、右到左、下到上和上到下，箭头尖必须精确落在 target 边界，Signal 名称位于线段
中点。source 是 target 任意层祖先时不显示箭头，但 source/target 高亮、Transition/Action 响应、异常
reason、滚动和导航仍按该 Signal 正常执行；self、后代到祖先、同级和跨分支 Signal 不受此隐藏规则
影响。即使祖先箭头隐藏，请求到达仍必须使 target 抖动。祖先关系只从当前 frame 已渲染节点的递归
包含关系判定，不进入 animation v3 或 Python frame。
self Signal 从节点右侧中部出发、在左侧中部结束，使用按节点实时宽高缩放的上方半圆弧；Signal 名称
位于上弧外侧。stage 内容必须提供同样按节点尺寸有界缩放的顶部净空，保证桌面、移动端和深层节点的
自环及文字不被裁剪。箭头几何必须在布局动画、父框膨胀、stage 滚动或尺寸变化时同步更新。stage 高度
增长时必须补偿纵向滚动，使左下布局锚点在 viewport 中保持稳定；后退按目标 frame 重算相同布局和
滚动边界。FLIP 只平滑已经确定的向外推开，reduced-motion 下直接到达确定位置。页面提供前后按钮、
`ArrowLeft`/`ArrowRight`、时刻数、Signal 总数和当前 Signal 文字说明，并尊重
`prefers-reduced-motion`。request 显示箭头和 source→target 请求信息，并在到达时抖动 target；
feedback 不画反向箭头，先显示 Transition 的真实 before state，再提交 after state并闪烁 target，
Action 同样闪烁但不伪造 lifecycle state；settle 不画箭头或反馈闪烁，提交真实状态并明确显示“目标
内部处理完成”，异常 settle 仍显示红色错误效果和 reason；terminal 保持确定状态并显示 outcome/reason。
reduced-motion 下取消位移、抖动、闪烁和脉冲，但立即恢复相同的确定 frame、活动端点、错误线型与
reason。普通和错误 SVG marker 使用精确 `5×4` 尺寸并保持箭头尖落在目标边界。首轮不包含自动播放、
速度、时间线跳转、浏览器 derive 或视频导出。

页面必须使用有上下限的流式尺寸而非绑定某个桌面分辨率。桌面外壳占据可用 viewport，主体按
紧凑 header、自动扩展 stage、紧凑控制栏排列；stage 占据主要空间并在内部滚动。窄屏允许元数据、
节点 identity 和控制栏自然换行以及页面纵向滚动，但页面本身不得横向溢出。主要长宽、间距、圆角、
字体、卡片 padding 和箭头曲率使用相对单位、容器/viewport 比例及有界 `clamp()`；只有边框和 SVG
stroke 等视觉细线可固定为 CSS pixel。

header 在一行优先显示小号品牌、请求标题以及 Source/Verdict/Signals/Protocol 四项紧凑元数据；
删除说明句。普通 system 节点不显示 `SYSTEM`，名称与裸状态名（例如 `Ready`）或 `Stateless` 位于
同一 identity 行且使用完全相同的流式字号；状态保持正常字重，空间不足时整体换行且不得覆盖；
底部 Transition 响应同样显示
`Base → Ready`，不显示 `State::`。External 和 Structure 保留类型标签。叶节点和 identity band 使用
约 `clamp(11rem, 16cqi, 15rem)` 的紧凑宽度，名称允许换行；含子系统的父框不得由 identity 获得固定
大宽度。控制栏在宽屏优先同行显示
时刻数、Signal 路径、响应或 reason，窄屏自然换行。source file 和 model fingerprint 只保留在 Protocol
项的 tooltip，不显示 footer。

## 兼容性决定

tools2 v8 model/view 的结构化字段提供上述生成语义；animation 封装协议保持 version 3。新生成器和
新 bundle 只发布 v3，不提供生成 v1/v2 的兼容开关；已经生成的自包含
v1/v2 HTML 仍可使用其各自内嵌播放器独立打开。前端不得从显示名称推断 handler/state，也不得复用老 `tools/`
静态 SVG renderer。
独立 CLI 是 `lkm-animate MODEL VIEW -o HTML`；tools2 driver 与便利入口另提供 `--html-out PATH`，并
保持 text `-o`、scenario、snapshot、work-dir 和 check 的 0/1 结果可组合。animate 协议/I/O 错误
返回 2。
