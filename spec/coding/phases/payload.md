# PayloadPhase 编码指引

PayloadPhase 是 [Kernel 系统编码](../systems/kernel.md)中 `Kernel.Enable` 的最后一个直接 drive
目标。model 来源为
[`spec/model/phases/payload/phase.spec`](../../model/phases/payload/phase.spec)，实现落点为
`impl/arceos_ex/src/phases/payload.rs`，selected adapter 位于 `impl/arceos_ex/src/apps/`。

## Transition 映射

| Transition | Source -> target | drives / completion | emits / continuation |
| --- | --- | --- | --- |
| Preset | Base -> Prepared | `preset()` 验证 KernelInitTask 主线和 `BinaryFormatRegistry.Ready`，依次 Setup `ExecSyncBoundaries`、`UserCloneDeferredBoundaries` | 提交 Prepared 后调用同对象 `setup()` |
| Setup | Prepared -> Ready | `setup_selected_payload(ctx)` 确认唯一 build-time kind；仅 user-boot 调用 `UserBootPayload.Setup` | 提交既有 Ready 后调用同对象 `enable()` |
| Enable | Ready -> Online | `prepare_selected_payload(ctx)`；hello/smoke 绑定内核入口，user-boot 完成 ELF/地址空间/trap/syscall、UserBootPayload.Online 和用户入口准备；随后提交 SelectedPayloadHandoff.Online | 提交 Payload Online、运行该 checkpoint handlers，再返回 `kernel::mark_online()` continuation |

`systems::kernel::enable_after_smp_runtime()` 只能调用 Payload.Preset。三个 transition 入口、两个同对象
continuation 和 Enable 的 Kernel continuation 都验证：当前引用是 KernelInitTask，唯一 stack switch/
entry count 为 1，既有 entry stack verification 成立，且此刻真实 `sp` 仍位于 KernelInitTask 的
16 KiB vmalloc stack。

## Selected payload adapter

Config 通过互斥的 `app_hello`、`app_smoke`、`app_user_boot` cfg 恰好绑定一个
`SelectedPayloadKind`。Context 持有 `SelectedPayloadHandoff`；adapter 只暴露 crate-internal 三个入口：

- `setup_selected_payload(ctx) -> EventResult`
- `prepare_selected_payload(ctx) -> EventResult`
- `enter_selected_payload(ctx) -> !`

前两个按编译期 kind 分发，第三个只能在 SelectedPayloadHandoff、PayloadPhase 和 Kernel 都 Online 后
进入对应不返回入口。hello/smoke 不推进 UserBootPayload；公共阶段也不准备 `UserTaskSet`。smoke
中的对象级 user-boot case 如需该集合，必须在 case 内建立并消费自己的测试状态。

user-boot prepare 保持 init candidate 选择、失败分类和 panic terminal 行为，然后把 normalized boot
arguments 交给 `ExecTransaction`；共享管线完成 ELF/interpreter、地址空间、用户栈、trap frame 和提交。
prepare 再完成 syscall、FilesStruct 与 stable `KernelInitTask` 的 user-resource binding，然后按
`Pid1UserAppFlow.Preset/Setup -> KernelInitFlow.Disable -> CommitFlowHandoff -> Pid1UserAppFlow.Enable
-> KernelInitFlow.Cleanup` 完成 `UserBootPayload.Enable`。enter 不再准备对象，只发出
`Pid1UserAppFlow.Online` checkpoint 并执行最终 RISC-V U-mode trap return。

## 提交与 checkpoint 顺序

Enable 的固定顺序是：

```text
variant prepare
  -> SelectedPayloadHandoff.Online
  -> PayloadPhase.Online
  -> PayloadPhase.Online checkpoint handlers
  -> Kernel.Online
  -> selected payload no-return entry
```

因此 PayloadPhase.Online 表示交接条件已经提交，不表示 payload 已经开始执行。user-boot requested-init
或 default-init 失败发生在 variant prepare 内，失败路径不得发出 SelectedPayloadHandoff.Online、
PayloadPhase.Online、Kernel.Online 或 Pid1UserAppFlow.Online checkpoint。

| Checkpoint | stable id | owner state |
| --- | --- | --- |
| `PayloadPhase.Started` | 481 | Base，Preset 已接受 |
| `PayloadPhase.Prepared` | 482 | Prepared，两个公共边界 Ready |
| `PayloadPhase.Ready` | 432 | Ready，selected setup 完成 |
| `PayloadPhase.Online` | 433 | Online，selected handoff Online |

新 checkpoint 只追加且默认 unmapped；既有 Ready/Online Linux exact mapping 和 ID 不变。所有四个
checkpoint 都由 KernelInitTask 发出。
