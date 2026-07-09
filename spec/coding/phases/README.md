# Coding Phases

Phase-level coding constraints live here when they need a dedicated topic file.

`../arceos_ex.spec` remains the compatibility entry and includes these
formal files. Read phase topics in model phase-tree order:

## Boot

- `boot/entry-prelude.spec` / `boot/entry-prelude.md`
- `boot/entry-successor.spec` / `boot/entry-successor.md`
- `boot/core-prepare.spec` / `boot/core-prepare.md`
- `boot/mm-core-init.spec` / `boot/mm-core-init.md`

## Interrupt

- `interrupt/irq-time-init.spec` / `interrupt/irq-time-init.md`
- `interrupt/local-irq-enable.spec` / `interrupt/local-irq-enable.md`
- `interrupt/irq-open-prepare.spec` / `interrupt/irq-open-prepare.md`
- `interrupt/process-prepare.spec` / `interrupt/process-prepare.md`

## UpMultitask

- `up-multitask/rest-init.spec` / `up-multitask/rest-init.md`

## SmpRuntime

- `smp-runtime/pre-smp-init.spec` / `smp-runtime/pre-smp-init.md`
- `smp-runtime/smp-bringup.spec` / `smp-runtime/smp-bringup.md`
- `smp-runtime/runtime-core.spec` / `smp-runtime/runtime-core.md`
- `smp-runtime/initcall.spec` / `smp-runtime/initcall.md`
- `smp-runtime/rootfs.spec` / `smp-runtime/rootfs.md`
- `smp-runtime/finalize.spec` / `smp-runtime/finalize.md`
