# PayloadPhase 编码指引

PayloadPhase 是[Kernel 系统编码](../systems/kernel.md)的最后一个子阶段编排层。按[阶段链式映射规则](../mapping.md#阶段链式映射规则)，编排层在 impl 中不出现独立函数，其 `drives` 语义坍缩为子阶段间的调用顺序。

## 串接顺序

PayloadPhase 当前为叶子阶段（其子阶段未展开），直接实现其三个迁移：

```
FinalizePhase.enable() 完成
  → PayloadPhase.preset()
    → .setup()
      → .enable()  → 不返回
```

具体迁移映射在对应文件中：
- [payload.md](payload.md)（待细化，或由载荷形态区分）
