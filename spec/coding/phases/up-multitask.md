# UpMultitaskPhase 编码指引

UpMultitaskPhase 是[Kernel 系统编码](../systems/kernel.md)的第三个子阶段编排层。按[阶段链式映射规则](../mapping.md#阶段链式映射规则)，编排层在 impl 中不出现独立函数，其 `drives` 语义坍缩为子阶段间的调用顺序。

## 串接顺序

UpMultitaskPhase 的子阶段在 impl 中的串接链：

```
ProcessPreparePhase.enable() 完成
  → BootInitRestInitPhase.preset()
    → .setup()
      → .enable()
        → BootInitScheduleHandoffPhase.preset()
          → .setup()
            → .enable()
              → BootIdleEntryPhase.preset()
                → .setup()
                  → .enable()
                    → SmpRuntimePhase 首个子阶段
```

各子阶段的详细迁移映射分别在对应文件中：
- [up-multitask/rest-init.md](up-multitask/rest-init.md)
