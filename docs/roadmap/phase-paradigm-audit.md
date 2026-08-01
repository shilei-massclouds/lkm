# Flow / Phase ownership audit

本页记录当前所有权形态。历史上的可替换 Flow 设计已被本轮固定 Task/TaskFlow 语义取代。

## Current ownership tree

```text
BootTask
└── BootInitFlow
    ├── entry/setup phases
    ├── rest-init phases
    ├── idle setup
    ├── schedule/return actions
    └── idle loop

KernelInitTask
└── KernelInitFlow
    └── KernelInitUserAppRuntime
        └── current ApplicationInstance

KthreaddTask
└── KthreaddFlow

UserTask[n]
└── UserTaskFlow[n]
    └── private UserAppRuntime[n]
        └── current ApplicationInstance[n]

ApIdleTask[cpu]
└── ApIdleFlow[cpu]
```

## Audit conclusions

- Task 与 TaskFlow 是终身一对一 carrier；没有 dispatch-kind、replacement 或历史 owned-Flow list。
- BootInitFlow 的 idle actions 是同一 Flow 的内部 continuation。BootTask 切出和恢复不会创建或选择另一
  Flow，也不会重启已经完成的 boot actions。
- KernelInitFlow、KthreaddFlow、UserTaskFlow 在所属 Task 发布时已经 Online；主体工作均为可挂起
  Action。ApIdleFlow 在 HSM grant 前也已 Online，但 Task authority 仍为 Reserved。
- 用户型 Flow 最多拥有一个稳定且不可共享的 UserAppRuntime。exec 只替换 Runtime 内部 application；
  fork 才创建新的 Task/Flow/Runtime identity。
- Trap/Interrupt/Exception Flow 是 CPU effective-flow stack 的嵌套 leaf，不接管 Task ownership，也不改变
  Task OnCpu 或 TaskFlow Online。
- PhaseObject 只表达确有独立生命周期与阶段完成边界的工作；普通 runtime loop/application continuation
  保持 Action 或 ResourceObject，不提升为替换 Flow。

## Refinement closure

- Charter：changed；当前所有权与执行边界为顶层权威。
- Model：changed；owned associations、fixed Ref 与 contextual actions 已形式化。
- Coding：changed；Rust storage、epoch、teardown order 已约束。
- Impl：changed；module/storage/checkpoint 已同步。
- Compose：reviewed, no change；变化均为既有 arceos_ex crate 内私有 lowering。
- Testing：changed；覆盖 boot identity、first/resume unified dispatch、exec/fork identity 与 terminal cleanup。
