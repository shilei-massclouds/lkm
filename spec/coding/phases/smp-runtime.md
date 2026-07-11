# SmpRuntimePhase 编码指引

SmpRuntimePhase 是[Kernel 系统编码](../systems/kernel.md)的第四个子阶段编排层。按[阶段链式映射规则](../mapping.md#阶段链式映射规则)，编排层在 impl 中不出现独立函数，其 `drives` 语义坍缩为子阶段间的调用顺序。

## 串接顺序

SmpRuntimePhase 的子阶段在 impl 中的串接链：

```
BootIdleEntryPhase.enable() 完成
  → PreSmpInitPhase.preset()
    → .setup()
      → .enable()
        → SmpBringupPhase.preset()
          → .setup()
            → .enable()
              → RuntimeCorePhase.preset()
                → .setup()
                  → .enable()
                    → InitcallPhase.preset()
                      → .setup()
                        → .enable()
                          → RootfsPhase.preset()
                            → .setup()
                              → .enable()
                                → FinalizePhase.preset()
                                  → .setup()
                                    → .enable()
                                      → PayloadPhase 首个子阶段
```

各子阶段的详细迁移映射分别在对应文件中：
- [smp-runtime/pre-smp-init.md](smp-runtime/pre-smp-init.md)
- [smp-runtime/smp-bringup.md](smp-runtime/smp-bringup.md)
- [smp-runtime/runtime-core.md](smp-runtime/runtime-core.md)
- [smp-runtime/initcall.md](smp-runtime/initcall.md)
- [smp-runtime/rootfs.md](smp-runtime/rootfs.md)
- [smp-runtime/finalize.md](smp-runtime/finalize.md)
