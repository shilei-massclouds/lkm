# Kernel 系统编码指引

`systems/kernel` 是 Kernel 生命周期的编排层。按[阶段链式映射规则](../mapping.md#阶段链式映射规则)，编排层在 impl 中不出现独立函数，其 `drives` 语义坍缩为子阶段间的调用顺序。

## 串接顺序

Kernel 在模型中编排以下阶段树顶层子阶段。impl 中的串接链为：

```
BootPhase 叶子链完成
  → prepare: EntryPreludePhase.preset()  ← 启动入口
  → EntryPreludePhase → EntrySuccessorPhase → CorePreparePhase → MmCoreInitPhase → SchedInitPhase
  → InterruptPhase 叶子链:
    → IrqTimeInitPhase → LocalIrqEnablePhase → IrqOpenPreparePhase → ProcessPreparePhase
  → UpMultitaskPhase 叶子链:
    → BootInitRestInitPhase → BootInitScheduleHandoffPhase → BootIdleEntryPhase
  → SmpRuntimePhase 叶子链:
    → PreSmpInitPhase → SmpBringupPhase → RuntimeCorePhase → InitcallPhase → RootfsPhase → FinalizePhase
  → PayloadPhase.preset() → .setup() → .enable()  → 不返回
```

各段串接分别在对应编排层 coding 文件中详述。

`UpMultitaskPhase` 之后的串接必须服从任务所有权：实现可以先线性提交
BootIdle owner-split 的对象/checkpoint 事实，但离开该阶段时必须真实切换到
`KernelInitTask` 的 task stack；`SmpRuntimePhase` 和 `PayloadPhase` 只能由
`kernel_init_entry()` 执行。BootIdle continuation 若恢复，必须留在无限 idle
调度循环；KthreaddTask 当前可使用临时的无限主动 schedule 循环。二者均不得
沿原启动调用栈直接执行 selected payload。

## 状态记录

Kernel 在 impl 中需要一个轻量状态变量用于记录和检查生命周期边界。但该变量不驱动任何阶段，仅用于 invariant 检查和 checkpoint。

## 范围边界

`systems/kernel` 只负责生命周期迁移的编排。子层关注点（ELF 加载、系统调用分发、地址空间设置、信号运行时、TTY、文件系统、凭据等）由各自的阶段或对象模块负责。这些主题的编码规则位于对应的阶段/对象编码文件中，不在此处。

## 形式谓词

本层不定义形式谓词。以上生命周期和范围规则是对 Kernel 系统的完整编码指引。
