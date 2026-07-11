# BootPhase 编码指引

BootPhase 是[Kernel 系统编码](../systems/kernel.md)的首个子阶段编排层。按[阶段链式映射规则](../mapping.md#阶段链式映射规则)，编排层在 impl 中不出现独立函数，其 `drives` 语义坍缩为子阶段间的调用顺序。

## 串接顺序

BootPhase 的子阶段在 impl 中的串接链：

```
entry_prelude 启动入口
  → EntryPreludePhase.preset()
    → .setup()
      → .enable()
        → EntrySuccessorPhase.preset()
          → .setup()
            → .enable()
              → CorePreparePhase.preset()
                → .setup()
                  → .enable()
                    → MmCoreInitPhase.preset()
                      → .setup()
                        → .enable()
                          → SchedInitPhase.preset()
                            → .setup()
                              → .enable()
                                → InterruptPhase 首个子阶段
```

各子阶段的详细迁移映射分别在对应文件中：
- [boot/entry-prelude.md](boot/entry-prelude.md)
- [boot/entry-successor.md](boot/entry-successor.md)
- [boot/core-prepare.md](boot/core-prepare.md)
- [boot/mm-core-init.md](boot/mm-core-init.md)
- [boot/sched-init.md](boot/sched-init.md)（待创建）
