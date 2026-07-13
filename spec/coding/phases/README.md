# Coding Phases

Phase-level model-to-impl mappings live in the `.md` files here. Read them in
model phase-tree order. Same-name `.spec` files are migration leftovers and
will be removed as each subtree audit completes.

## Boot

- [`boot.md`](boot.md) — BootPhase 编排链串接顺序
- `boot/entry-prelude.md`
- `boot/entry-successor.md`
- `boot/core-prepare.md`
- `boot/mm-core-init.md`
- `boot/sched-init.md`

## Interrupt

- [`interrupt.md`](interrupt.md) — InterruptPhase 编排链串接顺序
- `interrupt/irq-time-init.md`
- `interrupt/local-irq-enable.md`
- `interrupt/irq-open-prepare.md`
- `interrupt/process-prepare.md`

## UpMultitask

- [`up-multitask.md`](up-multitask.md) — UpMultitaskPhase 编排链串接顺序
- `up-multitask/rest-init.md`

## SmpRuntime

- [`smp-runtime.md`](smp-runtime.md) — SmpRuntimePhase 编排链串接顺序
- `smp-runtime/pre-smp-init.md`
- `smp-runtime/smp-bringup.md`
- `smp-runtime/runtime-core.md`
- `smp-runtime/initcall.md`
- `smp-runtime/rootfs.md`
- `smp-runtime/finalize.md`

## Payload

- [`payload.md`](payload.md) — PayloadPhase 编排链串接顺序
