# Coding Phases

Phase-level model-to-impl mappings live in the `.md` files here. Read them in
model phase-tree order. Coding `.spec` files have been retired and are forbidden
by the repository `coding-spec-check` gate.

## Boot

- [`boot.md`](boot.md) — BootPhase 父 transition 驱动映射
- `boot/entry-prelude.md`
- `boot/entry-successor.md`
- `boot/core-prepare.md`
- `boot/mm-core-init.md`
- `boot/sched-init.md`

## Interrupt

- [`interrupt.md`](interrupt.md) — InterruptPhase 父 transition 驱动映射
- `interrupt/irq-time-init.md`
- `interrupt/local-irq-enable.md`
- `interrupt/irq-open-prepare.md`
- `interrupt/process-prepare.md`

## BootInitFlow

- [`boot-init.md`](boot-init.md) — BootInitFlow Phase 父 transition 驱动映射
- `boot-init/rest-init.md`

## SmpRuntime

- [`smp-runtime.md`](smp-runtime.md) — SmpRuntimePhase 父 transition 驱动映射
- `smp-runtime/pre-smp-init.md`
- `smp-runtime/smp-bringup.md`
- `smp-runtime/runtime-core.md`
- `smp-runtime/initcall.md`
- `smp-runtime/rootfs.md`
- `smp-runtime/finalize.md`

## Payload

- [`payload.md`](payload.md) — PayloadPhase transition 映射
