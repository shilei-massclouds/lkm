# Rust Coding 规格

本文记录把当前模型规格落到 Rust 内核实现时需要遵守的补充约束。

## 适用范围

本文件补充 Rust 语言层面的模块组织、安全边界、状态表示、错误处理和裸机约束。它不改变 `spec/model` 中的对象生命周期语义。

## 基础约束

- 内核主体默认使用 `#![no_std]`。
- 早期启动路径不得依赖堆分配，除非对应 allocator 对象已经进入可用状态。
- 全局静态对象必须有清晰的初始化阶段边界，不能依赖 Rust 默认初始化语义掩盖模型事件。
- 架构相关寄存器、裸指针、链接符号和页表写入应通过小而明确的 unsafe 封装暴露给安全代码。

## 生命周期表示

- 模型事件默认映射为 `preset`、`setup`、`enable`、`cleanup` 函数。
- 对需要强约束的对象，优先考虑 typestate 或私有字段限制错误调用顺序。
- 对启动早期不适合 typestate 的全局对象，应至少提供 debug checkpoint 或运行期断言。
- 状态名应使用模型规范集合：`Base`、`Prepared`、`Ready`、`Online`、`Destroyed`。

## 错误处理

- 可恢复错误使用明确的 `Result`。
- 违反启动阶段硬前提的错误可以进入 panic 或早期 halt，但必须报告失败对象和事件。
- 对规格中的 `deferred` 项，不得静默实现；应保留 stub、feature gate 或明确 TODO。

## unsafe 边界

每个 `unsafe` 块应能归入以下类别之一：

- 启动 ABI 交接。
- CSR 或特权指令访问。
- 裸物理/虚拟地址访问。
- 页表和 TLB 相关操作。
- 链接脚本符号引用。
- 与外部汇编或固件 ABI 交互。

新增或修改 `unsafe` 块、`unsafe fn`、`unsafe extern`、`unsafe impl` 和 `#[unsafe(...)]`
属性时，必须在就近位置显式标注原因。说明至少应覆盖其所属类别、对应的规格对象或事件边界、
为什么安全 Rust 无法表达该操作，以及调用者或维护者必须维持的不变量。对纯机械、局部且语义相同的
重复 `unsafe`，可以由所在函数或模块统一说明，但不能完全省略说明。

仅在 safe code 无法满足规格约束或裸机实现要求时才使用 unsafe code。如果无法避免，应尽量缩小
`unsafe` 的作用范围，并将其封装在明确的内部边界内，对外优先暴露 safe 接口供其它代码使用。

无法归类的 `unsafe` 应先补充 coding 规格或重新设计接口。

## 测试与检查

- 能在 host 上测试的对象状态转换应写普通 Rust 单元测试。
- 只能在 QEMU/硬件上验证的阶段边界应提供 trace/checkpoint 输出。
- 关键 `depends_on` 和 `ensures` 应尽量转化为断言、日志点或可采集状态。

## 待补充

- 目标内核 crate 边界。
- 是否引入统一 lifecycle trait。
- panic、early halt 和 trace 输出的具体格式。
