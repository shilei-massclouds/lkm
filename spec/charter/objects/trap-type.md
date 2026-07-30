# TrapType

`TrapType` 维护异常和中断共用的总入口 `TrapFlowType`。异常或中断发生时，执行先进入这个总入口，
再根据事件类别进入相应的处理路径；`TrapType` 负责总入口及分流前的公共边界，具体处理语义由对应的
异常或中断类型负责。

`TrapType` 通过生命周期逐步建立和开放总入口服务。当前先定义 `Preset`：它为所属 CPU 建立临时
保护入口，用于处理初始化过程中意外发生的异常或中断，便于测试和定位缺陷。`Setup` 和 `Enable` 的
语义由后续校准继续补充。

## Mapping

- Model: `spec/model/objects/trap_type.spec`
- Coding: `spec/coding/objects/trap-type.md`
- Implementation: `impl/arceos_ex/src/objects/trap_type.rs`
