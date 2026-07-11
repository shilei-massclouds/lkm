# Coding Phases

Phase-level coding constraints live here when they need a dedicated topic file.

`../arceos_ex.spec` remains the compatibility entry and includes these
formal files. Read phase topics in model phase-tree order:

## Boot

- [`boot.md`](boot.md) — BootPhase 编排链串接顺序
- `boot/entry-prelude.md` / `boot/entry-prelude.spec`
- `boot/entry-successor.md` / `boot/entry-successor.spec`
- `boot/core-prepare.md` / `boot/core-prepare.spec`
- `boot/mm-core-init.md` / `boot/mm-core-init.spec`

## Interrupt

- [`interrupt.md`](interrupt.md) — InterruptPhase 编排链串接顺序
- `interrupt/irq-time-init.md` / `interrupt/irq-time-init.spec`
- `interrupt/local-irq-enable.md` / `interrupt/local-irq-enable.spec`
- `interrupt/irq-open-prepare.md` / `interrupt/irq-open-prepare.spec`
- `interrupt/process-prepare.md` / `interrupt/process-prepare.spec`

## UpMultitask

- [`up-multitask.md`](up-multitask.md) — UpMultitaskPhase 编排链串接顺序
- `up-multitask/rest-init.md` / `up-multitask/rest-init.spec`

## SmpRuntime

- [`smp-runtime.md`](smp-runtime.md) — SmpRuntimePhase 编排链串接顺序
- `smp-runtime/pre-smp-init.md` / `smp-runtime/pre-smp-init.spec`
- `smp-runtime/smp-bringup.md` / `smp-runtime/smp-bringup.spec`
- `smp-runtime/runtime-core.md` / `smp-runtime/runtime-core.spec`
- `smp-runtime/initcall.md` / `smp-runtime/initcall.spec`
- `smp-runtime/rootfs.md` / `smp-runtime/rootfs.spec`
- `smp-runtime/finalize.md` / `smp-runtime/finalize.spec`

## Payload

- [`payload.md`](payload.md) — PayloadPhase 编排链串接顺序
