# TrapType

`TrapType` 是所属 CPU 上异常和中断响应的稳定类型资源，`TrapFlowType` 是一次具体陷入的动态响应流；
二者的关系如同 `Task` 与 `TaskFlow`。`TrapType` 维护共用响应入口及分流前的公共边界，每次正式陷入
通过该入口建立 fresh `TrapFlowType`，再根据事件类别进入对应的中断或异常处理路径；具体处理语义由
`InterruptType` 或 `ExceptionType` 负责。

`TrapType` 通过生命周期逐步建立和开放响应入口。`Preset` 为所属 CPU 建立临时保护入口，用于处理
初始化过程中意外发生的异常或中断，便于测试和定位缺陷。`Setup` 把该 CPU 的异常/中断响应入口从
临时保护入口重置为正式的 `TrapFlowType` 响应流入口；在发布正式入口前，它驱动
`ExceptionType.Preset`，由后者继续驱动各异常子类型的 `Preset`，使当前已经支持的异常能力至少具有
确定的初始兜底。成功后 `TrapType` 进入 Ready，但该动作不开放中断，也不使完整异常处理服务 Online。
尚未闭合的具体处理能力可以保留为 deferred obligation，不能被解释为 `Setup` 已经提供的保证；
`Enable` 的语义由后续校准继续补充。

正式入口只能取得入口 CPU 自有 TrapType、Scheduler/InterruptType 和当前 Task 的窄运行期 lease；AP
不得取得全局 mutable Context。共享 ExceptionTable 只通过运行期只读引用查询。每 CPU 还维护稳定的
trap observation：root/interrupt/exception/SSIP 完成数、return-token 消费数、leaf-switch resume 数和
最后 generation；这些只用于长期诊断与验收，不改变 checkpoint 语义。

## Mapping

- Model: `spec/model/objects/trap_type.spec`
- Coding: `spec/coding/objects/trap-type.md`
- Implementation: `impl/arceos_ex/src/objects/trap_type.rs`
