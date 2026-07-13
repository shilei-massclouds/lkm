# 多核运行期阶段

SmpRuntimePhase 是 Kernel.Enable 在真实 KernelInitTask 执行线上驱动的第二个直接阶段。
它承接 UpMultitaskPhase.Online 后完成的 BootIdleTask -> KernelInitTask 栈切换，从
`kernel_init()` / `kernel_init_freeable()` 继续推进，直到内核初始化收尾完成并可进入
PayloadPhase。

> [model] MUST：多核运行期阶段正式命名是 SmpRuntimePhase，执行主体是 KernelInitTask。

## 边界与子阶段

- 入口：UpMultitaskPhase.Online，KernelInitTask 已唯一进入自己的 vmalloc task stack。
- 直接子阶段：PreSmpInitPhase、SmpBringupPhase、RuntimeCorePhase、InitcallPhase、
  RootfsPhase、FinalizePhase，共六个，顺序固定。
- 出口：SmpRuntimePhase.Online 后返回 Kernel.Enable continuation，再由 Kernel 驱动
  PayloadPhase。

SmpBringupPhase 的父级驱动属于 KernelInitTask 的 BP 执行线。该阶段内部
ApEntryPreludePhase、ApSmpCallinPhase 和 ApOnlineIdlePhase 仍由各 AP 执行；BP 只负责
准备、发起 HSM 启动和等待 completion，不获得 AP 子阶段的执行所有权。

## 生命周期

SmpRuntimePhase 和六个直接子阶段均遵循标准
`Base -> Prepared -> Ready -> Online` 生命周期。Started checkpoint 表示对应 Preset 已被
接受，不是持久状态。

### Preset：Base -> Prepared

SmpRuntimePhase.Preset 检查 UpMultitaskPhase.Online、KernelInitTask.Online、唯一入口和
KernelInitTask 栈事实，驱动 PreSmpInitPhase.Preset。PreSmpInitPhase.Online 后，父
continuation 提交 SmpRuntimePhase.Prepared。

### Setup：Prepared -> Ready

SmpRuntimePhase.Setup 检查 PreSmpInitPhase.Online，驱动 SmpBringupPhase.Preset。
SmpBringupPhase.Online 后，父 continuation 提交 SmpRuntimePhase.Ready。

### Enable：Ready -> Online

SmpRuntimePhase.Enable 检查 SmpBringupPhase.Online，依次驱动 RuntimeCorePhase、
InitcallPhase、RootfsPhase 和 FinalizePhase 的 Preset。每个直接子阶段只有在到达 Online
后才能返回父 continuation 并启动下一个 sibling。FinalizePhase.Online 后，父 continuation
提交 SmpRuntimePhase.Online 并返回 Kernel.Enable。

六个直接子阶段的对象动作都属于各自 Preset；其 Setup 和 Enable 不再驱动对象或 sibling，
只检查既有事实并发布 Ready、Online。所有下游阶段依赖前一阶段精确 Online，不消费阶段
Ready 作为完成边界。

## 所有权与可观测性

SmpRuntimePhase 入口和每个父 continuation 都必须重新检查当前实际 SP 位于
KernelInitTask 的 task stack，KernelInitTask entry count 和真实 stack-switch count 仍各为 1，
且当前任务引用仍是 KernelInitTask。该检查覆盖父阶段和六个 BP 主线子阶段的完整 checkpoint
链；AP checkpoint 使用 AP idle task owner，不套用 BP task-owner 断言。

每个标准阶段保留 Started、Prepared、Ready、Online 四个 checkpoint。既有 Started/Ready
checkpoint 的标识和 Linux 映射保持稳定；新增 Prepared/Online 默认不建立 Linux paired hard
scope 映射。

## 引用

- [阶段范式](../phase-paradigm.md)
- [内核阶段](../systems/kernel.md)
- [UpMultitaskPhase](up-multitask.md)
