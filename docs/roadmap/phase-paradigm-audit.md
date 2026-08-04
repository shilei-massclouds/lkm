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
└── TaskFlow[n]
    └── private UserAppRuntime[n]
        └── current ApplicationInstance[n]

ApIdleTask[cpu]
└── ApIdleFlow[cpu]
```

## Audit conclusions

- Task 与 TaskFlow 是终身一对一 carrier；没有 dispatch-kind、replacement 或历史 owned-Flow list。
- BootInitFlow 的 idle actions 是同一 Flow 的内部 continuation。BootTask 切出和恢复不会创建或选择另一
  Flow，也不会重启已经完成的 boot actions。
- KernelInitFlow、KthreaddFlow、用户 Task 的普通 TaskFlow 在所属 Task 发布时已经 Online；主体工作均为可挂起
  Action。ApIdleFlow 在 HSM grant 前也已 Online，但 Task authority 仍为 Reserved。
- 用户型 Flow 最多拥有一个稳定且不可共享的 UserAppRuntime。exec 只替换 Runtime 内部 application；
  fork 才创建新的 Task/Flow/Runtime/Application identity。
- fork 通过单个 snapshot candidate 原子物化 `Task=Online / TaskFlow=Online /
  UserAppRuntime=Online`，不重放 Preset/Setup/Enable；发布失败不留下 PID、PTE、frame ref、slot 或可解析
  identity。parent 保持 `OnCpu/Online/Online`，child 不继承活动 trap/yield/CPU authority。
- Trap/Interrupt/Exception Flow 是 CPU effective-flow stack 的嵌套 leaf，不接管 Task ownership，也不改变
  Task OnCpu 或 TaskFlow Online。返回 token 只允许入口 CPU、原 TaskRef/TaskFlowRef generation 与 context
  epoch 消费一次；用户态/内核态差异只由体系结构现场决定。
- PhaseObject 只表达确有独立生命周期与阶段完成边界的工作；普通 runtime loop/application continuation
  保持 Action 或 ResourceObject，不提升为替换 Flow。

## Refinement closure

- Charter：changed；唯一普通 TaskFlow、被动 Runtime、fork 状态矩阵与 terminal 顺序为顶层权威；锁定的
  `spec/charter/systems/computer.md` reviewed, no change。
- Model：changed；`snapshot/materialize` 原子候选、fork fresh aggregate、exec identity 与 terminal cleanup 已形式化。
- Coding：changed；reserved/occupied publication、generation、ContextCoordinate、COW/refcount 与回滚已约束。
- Impl：changed；Task 内嵌普通 TaskFlow、fork snapshot candidate、Runtime/Application identity 与 overlay authority
  已同步。
- Compose：reviewed, no change；变化均为既有 arceos_ex crate 内私有 lowering。
- Testing：changed；覆盖原子失败回滚、父子状态/identity、exec replacement、terminal/stale ref 与统一 overlay。

## Completion chain and evidence

- `aa8f2d7f tools2: add atomic snapshot materialization`
- `cda55396 model: materialize fork child aggregates`
- `210a8ed9 runtime: tighten trap overlay return authority`
- `make -C tools2 test-all`：Python 109/109、frontend 16/16、E2E 9/9。
- `make verify` 与 `make coding-spec-check`：通过；模型 fingerprint 为
  `sha256:cd41bce10c5759a3107ebf4a0bda860197ccd339de6b7500f23ee9fdcdd4b1e8`。
- 仓库根直接 `make test`：188/188；两种 PLIC provider 的 app smoke 各 58/58、KUnit 各 25/25。
- 默认 `make stress-test`：四组各 10/10，合计 40/40；`make difftest`：1/1。
- `git diff --check`、Charter lock check 与旧术语扫描通过；最终无残留 QEMU。
