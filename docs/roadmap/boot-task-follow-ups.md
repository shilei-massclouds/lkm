# Boot/Task 后续工作清单

本文件记录 2026-07-22 `BootTask` 入口即 Online、`BootInitFlow` Phase 化实施轮次明确排除的
后续工作。除特别注明外，各项在实施时应另开 charter-first 轮次，先确认设计意图与跨层影响，
不在当前轮次顺带修改。

## 1. Init phase 命名与 KernelInitFlow Phase 化

- 将当前 TaskFlow 对象 `KernelInitFlow` 重新审视为 PhaseObject，并计划更名为
  `KernelInitPhase`。
- 将当前 PhaseObject `BootInitFlow` 计划更名为 `BootInitPhase`。
- 迁移时同时核对 TaskFlow ownership、Phase parent、checkpoint 名称、实现类型和差分 artifact，
  不保留兼容 alias。

## 2. Task 创建 lifecycle 与 kernel_clone 对齐

- 重新定义 `Task` 类型的 `Preset/Setup/Enable` 为“新任务创建”生命周期。
- 目标映射：`Preset` 对应 `copy_process()`，`Setup` 可为空 transition，`Enable` 对应
  `wake_up_new_task()`。
- 核对当前 TaskCreationCore、PID 分配、copy_thread、runqueue publication、初始 Flow binding
  与失败回滚事实应分别归属哪个 transition。

## 3. 引导子阶段执行载体审计

- 逐一复核所有 boot/runtime 子阶段应挂到哪个 TaskFlow 或 Task phase。
- 以真实 current task、实际栈、调度切换、中断/异常进入返回和 continuation ownership 为边界，
  修正仅由源码函数邻接或旧聚合 wrapper 推导出的 parent/执行链。

## 4. TaskCoreCrate 传入参数

- 单独澄清 `TaskCoreCrate` 的传入参数集合、所有权、生命周期、调用边界和 provider/compose 责任。
- 在设计决策前先记录现有调用点、参数来源、跨 crate 暴露和测试依赖，不预设接口修复方案。

## 5. 信号触发与系统响应序列图

- 长期计划：引入从外部/内部信号触发到系统对象响应、阶段推进、调度或返回的序列图。
- 序列图应由已确认的对象/事件/transition 权威生成或校验，避免成为与 charter/model 分离的
  第二套行为定义。
