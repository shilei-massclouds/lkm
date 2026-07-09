# EntryPreludePhase coding

本文件承载 `spec/coding/phases/boot/entry-prelude.spec` 的说明性正文。Formal 文件只保留 rule ID、type 分组和短标签。

<!-- formal-predicate-notes:spec/coding/phases/boot/entry-prelude.spec START -->

## Formal predicate notes

以下说明从 `spec/coding/arceos_ex.md` 迁移而来；对应 formal 规则位于 [`entry-prelude.spec`](entry-prelude.spec)。

### ArceosExEntryPreludeCodingMust

#### RISC-V early alternatives boundary

Linux setup_vm() calls apply_early_boot_alternatives() while the MMU
is still off when CONFIG_RISCV_ALTERNATIVE_EARLY=y. The current
arceos_ex EntryPrelude implementation may defer the actual
alternatives/errata text patch object, but Vm.Preset and the phase
ready check must keep that deferral observable instead of silently
treating the Linux path as absent or implemented.

<!-- formal-predicate-notes:spec/coding/phases/boot/entry-prelude.spec END -->
