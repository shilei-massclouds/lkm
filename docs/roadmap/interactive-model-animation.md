# LKM Model 交互动画展示计划

## 目标

在现有 model 工具链的静态图展示基础上，新增一种交互式 HTML 展示。动画内容由 model 规格及其 derive 结果自动生成，不引入需要人工维护的第二套动画规格或动画脚本 DSL。

交互动画采用信号驱动语义：从一个信号发出，到目标系统完成本次响应，构成一个可前进、可后退的完整步骤。

## 已确认的设计决定

- `.spec` 仍是唯一规格源。
- `model.json` 表达静态对象、状态、transition 和关系。
- `derive.json` 表达一次实际推导中的当前状态、guard 求值、状态推进、接受、丢弃和阻塞结果。
- `view.json` 是自动生成的展示中间文件，可包含交互动画需要的结构化步骤；它不是人工维护的语义来源。
- 保留现有静态 `svg` 输出。
- 新增交互式 `html` 输出，HTML 内部使用 SVG 绘图和 JavaScript 控制。
- 不新增 `animated-svg`。自动播放能力由交互式 HTML 覆盖，避免维护两套动画 renderer。
- 交互只浏览一次确定的 derive trace，不在浏览器内重新推导模型或选择新的执行分支。

## 数据流

```text
.spec
  -> parse
ast.json
  -> model
model.json
  -> derive
derive.json
  -> view
view.json（包含 signal transaction steps）
  -> render --format svg
静态 SVG
  -> render --format html
交互式 HTML
```

静态结构视图可以由 `model.json` 生成。需要展示动态 guard、lossy discard、blocked 和实际状态推进的交互 trace 必须由 `derive.json` 生成。

## 交互步骤语义

一个步骤是一个完整的 signal transaction：

```text
信号发出
  -> 解析动态 receiver/目标对象
  -> 检查目标当前 lifecycle state
  -> 以当前推导快照求值 guard/depends_on
  -> 执行 transition 或 action
  -> 应用 updates/ensures
  -> 处理该响应中的嵌套 drives/emits
  -> 目标响应以 accepted、discarded 或 blocked 结束
```

默认情况下，响应期间产生的嵌套信号属于当前顶层 signal transaction，并在该步骤内部按顺序展示。后续如有需要，可以增加“展开子步骤”模式，但不得改变顶层一步的事务边界。

步骤结果至少包括：

- `accepted`：guard 成立，目标 transition/action 完成，状态可能推进。
- `lossy_discarded`：信号已发出但目标当前不能接受，记录丢弃，目标状态不变。
- `strict_blocked`：严格信号不能完成，展示阻塞位置和失败条件。
- `completed_no_state_change`：响应完成但没有 lifecycle 状态变化。

## Animation View 中间结构

扩展 view schema，以结构化字段表达动画，而不是由 HTML renderer 根据标签文本、颜色或名称猜测语义。建议增加 `animation_steps`，每一步至少包含：

```json
{
  "id": "step-17",
  "kind": "signal_transaction",
  "source": "BootTask",
  "signal": "BootTask.initial_flow.Transition::Preset",
  "receiver": "BootInitFlow",
  "outcome": "accepted",
  "before_snapshot": {
    "BootInitFlow.state": "Base"
  },
  "events": [
    {
      "kind": "receiver_resolved",
      "receiver": "BootInitFlow"
    },
    {
      "kind": "guard_check",
      "predicate": "task_flow_dispatch_guard_satisfied",
      "result": true,
      "conditions": [
        {
          "expression": "BootTask.state == Online",
          "result": true
        },
        {
          "expression": "BootDispatchWindow.current_task -> BootTask",
          "result": true
        }
      ]
    },
    {
      "kind": "transition",
      "object": "BootInitFlow",
      "event": "Preset",
      "from": "Base",
      "to": "Prepared"
    }
  ],
  "after_snapshot": {
    "BootInitFlow.state": "Prepared"
  },
  "source_refs": []
}
```

实际 schema 应复用 derive 中已有的稳定 record、transition、runtime instance 和 trace 标识，避免复制不能保持一致的数据。所有步骤和子事件应保留顺序、父子关联以及可追溯到 `.spec` 的 source span/record ID。

## 前进与后退

交互播放器维护当前步骤索引，并使用确定的状态快照显示每个边界：

- 向前：从当前步骤的 `before_snapshot` 展示内部事件，并收敛到 `after_snapshot`。
- 向后：恢复上一边界的快照，再按需要播放反向视觉过渡。
- 不通过“逆向执行 transition”实现后退，因为对象创建、binding 和 update 不保证可逆。
- 重复前进和后退必须得到完全相同的显示状态，不得重新执行 derive。

快照不一定要在 JSON 中完整复制。实现可以使用初始完整快照加每步确定性 delta，并在 view 或浏览器加载时建立可随机访问的边界快照；但这属于存储优化，不能改变交互语义。

## HTML 交互

默认键位：

- `ArrowRight` / `PageDown`：下一步。
- `ArrowLeft` / `PageUp`：上一步。
- `Space`：自动播放或暂停。
- `Home`：回到初始状态。
- `End`：跳到最终状态。

页面同时提供可点击控制，不能只依赖键盘。至少包括：

- 上一步、下一步；
- 播放、暂停；
- 当前步骤和总步骤数；
- 播放速度；
- 当前 signal、receiver、guard 条件和 outcome 的文字说明。

动画应服务于语义理解：高亮当前 source、signal 路径、receiver、guard 条件、状态变化和失败位置，不加入与推导无关的装饰效果。颜色之外还应使用文本、图形或线型区分结果，保证基本可访问性。

## 命令行接口

底层 render 工具收敛为：

```bash
lkm-render trace.view.json --format svg -o trace.svg
lkm-render trace.view.json --format html -o trace.html
```

最终支持的格式集合为：

```text
text | dot | svg | html
```

其中：

- `text`、`dot`、`svg` 保持现有行为兼容。
- `html` 表示自包含或明确管理资源的交互式 trace 页面。
- 不提供 `animated-svg`。

顶层 `pyveri` 可以增加相应便利选项，但应继续通过 `view.json -> render` 调用，不允许顶层 driver 绕过 view schema 直接从 derive 数据拼装 HTML。具体顶层参数名在检查现有 CLI 一致性后确定。

## 实施层次

本需求扩展了展示接口和中间数据契约，实施时遵循仓库 charter-first 工作流：

1. 在 charter 中确认交互 trace 的用途、一步的 signal transaction 边界、可逆浏览语义以及非模拟器边界。
2. 更新 model/tooling 语义文档，明确 derive、view、render 各层职责。
3. 更新 coding 规格，定义 view schema、稳定 ID、快照/delta 和 renderer 接口。
4. 复核 compose 层；仅在存在相关输出组合或发布要求时修改。
5. 扩展 view 类型和 JSON schema，生成结构化 `animation_steps`。
6. 扩展 view builder，从 derive trace/records 构造 signal transaction 和前后状态边界。
7. 新增 HTML renderer；复用现有 SVG 布局和视觉元素，不复制模型/推导逻辑。
8. 扩展 render CLI 的 `--format` choices，并为顶层 pyveri 增加一致的便利入口。
9. 补充工具测试、浏览器无关的 HTML 结构测试和必要的端到端测试。
10. 运行 focused validation、`git diff --check`，最后从仓库根目录直接运行 `make test`。

## 验证要求

至少覆盖：

- 现有静态 SVG 输出保持不变或只有明确接受的变更。
- 同一 derive trace 可同时生成静态 SVG 和交互 HTML。
- accepted signal 能从发出展示到目标响应完成。
- parent Task Online 且 DispatchWindow 指向 parent 时，TaskFlow guard 显示为通过并允许推进。
- guard 任一条件失败时，明确高亮失败条件且不推进目标状态。
- lossy signal 显示为 discarded，一步完成后状态不变。
- strict signal 显示为 blocked，步骤停止在确定边界。
- 动态 receiver 和 TaskRef 解引用后的实际目标显示正确。
- nested emits 的显示顺序和顶层事务完成边界正确。
- 向前、向后、重复往返均恢复相同状态。
- Home、End、播放暂停和速度控制工作正确。
- HTML 在无外部网络依赖的情况下可以打开和操作，除非 charter 明确允许打包本地资源。
- view/render 不从名称、标签或颜色猜测 guard、transition 或 outcome 语义。
- 严格模型验证保持零 obligation、blocked、contradiction（针对预期成功的主模型验证）。
- 最终执行 focused tests、`git diff --check` 和仓库根目录直接 `make test`。

## 非目标

- 不在浏览器中重新运行 derive。
- 不允许用户通过按键选择新的模型分支。
- 不把 HTML 播放器变成 runtime 模拟器。
- 不维护人工动画时间线作为第二份规格。
- 不新增 `animated-svg` renderer。
- 第一阶段不要求 MP4 导出；如以后需要，HTML 可以作为确定性渲染源，再由独立导出工具录制或转换。
