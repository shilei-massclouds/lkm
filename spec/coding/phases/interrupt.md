# InterruptPhase 编码指引

InterruptPhase 是[Kernel 系统编码](../systems/kernel.md)的第二个子阶段编排层。按[阶段链式映射规则](../mapping.md#阶段链式映射规则)，编排层在 impl 中不出现独立函数，其 `drives` 语义坍缩为子阶段间的调用顺序。

## 串接顺序

InterruptPhase 的子阶段在 impl 中的串接链：

```
SchedInitPhase.enable() 完成
  → IrqTimeInitPhase.preset()
    → .setup()
      → .enable()
        → LocalIrqEnablePhase.preset()
          → .setup()
            → .enable()
              → IrqOpenPreparePhase.preset()
                → .setup()
                  → .enable()
                    → ProcessPreparePhase.preset()
                      → .setup()
                        → .enable()
                          → UpMultitaskPhase 首个子阶段
```

各子阶段的详细迁移映射分别在对应文件中：
- [interrupt/irq-time-init.md](interrupt/irq-time-init.md)
- [interrupt/local-irq-enable.md](interrupt/local-irq-enable.md)
- [interrupt/irq-open-prepare.md](interrupt/irq-open-prepare.md)
- [interrupt/process-prepare.md](interrupt/process-prepare.md)
