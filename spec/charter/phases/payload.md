# 应用交接期阶段

PayloadPhase 是 KernelInitTask 在内核初始化末尾准备并移交唯一 selected payload 的阶段。构建配置
必须在 `Hello`、`Smoke` 和 `UserBoot` 三种 payload 中恰好选择一种；本阶段不提供运行期切换或
插件机制。

> [model] MUST：应用交接期阶段正式命名是 PayloadPhase。

## 边界与职责

1. 入口边界：SmpRuntimePhase 已经 Online，KernelInitTask 仍运行在自己的 vmalloc stack 上。
2. Preset 验证 Initcall 已建立 `BinaryFormatRegistry`，并准备所有 payload 共用的
   `ExecSyncBoundaries` 与 clone deferred 边界。
3. Setup 确认唯一 selected payload，并完成该变种专属的 setup；只有 UserBoot 变种推进
   UserBootPayload.Setup。
4. Enable 准备不可返回入口；UserBoot 变种只选择 requested/default/fallback 候选并调用共享
   `ExecTransaction`，由 registry/ELF/address-space 管线完成映像准备，再提交 UserBootPayload.Online 和
   `KernelInitTask: KernelInitFlow -> UserAppFlow` handoff；Hello/Smoke 只确认对应内核态入口。
5. 出口边界：PayloadPhase Online 已提交，返回 Kernel.Enable continuation；Kernel Online 提交后
   才进入 selected payload 的不返回入口。

PayloadPhase.Online、Kernel.Online 和 selected payload no-return entry 是三个独立边界。任何变种在
handoff 准备阶段失败时都不得伪造前两个 Online；UserBoot 的 requested/default init 失败继续保持
既有 panic terminal 语义。

## 生命周期

### 范式

阶段遵循标准 `Base -> Prepared -> Ready -> Online` 生命周期。Preset、Setup 和 Enable 成功提交后，
前两个 transition 分别发出同对象 Setup 和 Enable；Enable 完成后返回 Kernel continuation。

### 状态与迁移

* Base：阶段尚未准备公共 payload 边界。

* Preset：驱动 ExecSyncBoundaries.Setup 和 UserCloneDeferredBoundaries.Setup，等待两者 Ready，并验证
  BinaryFormatRegistry.Ready，提交 Prepared 并发出 Setup。

* Setup：驱动 SelectedPayloadHandoff.Setup，确认 Config 中恰好一种 selected kind；只有 UserBoot
  分支要求 UserBootPayload.Ready，随后提交 Ready 并发出 Enable。

* Enable：驱动 SelectedPayloadHandoff.Enable。Hello/Smoke 绑定内核态 no-return entry；UserBoot
  完成用户映像与入口准备，依次建立 fresh `UserAppFlow`、Disable `KernelInitFlow`、提交 stable
  `KernelInitTask` 的 active handoff、Enable 新 Flow、Cleanup 旧 Flow，并提交 UserBootPayload.Online。
  selected handoff Online 后提交
  PayloadPhase.Online，并返回 Kernel.Enable continuation。

* Online：selected payload 的交接条件已经提交，但 payload 尚不必已经进入；Kernel.Online 与实际
  no-return entry 仍是后续独立边界。

三个 transition 和它们驱动的对象动作都由 KernelInitTask 执行，并持续验证唯一 task handoff、
entry 事实及真实 SP 位于 KernelInitTask vmalloc stack。

成功 exec 不替换 PID 1 Task。用户地址空间、files、credentials、signal 和 trap frame 仍直接关联
`KernelInitTask`；`UserAppFlow` 只保存本次应用 continuation 的独立 lifecycle。用户应用内部是
黑盒，syscall/trap 仍由相应内核对象处理。

## 引用

* [阶段范式](../phase-paradigm.md)
* [PayloadPhase model](../../model/phases/payload/phase.spec)
* [ExecTransaction](../objects/exec-transaction.md)
* [BinaryFormatRegistry](../objects/binary-format-registry.md)
* [ExecSyncBoundaries](../objects/exec-sync-boundaries.md)
* [ElfObject](../objects/elf-object.md)
