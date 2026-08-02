# Coding Phases

Phase-level model-to-impl mappings live in the `.md` files here. Read them in
model phase-tree order. Coding `.spec` files have been retired and are forbidden
by the repository `coding-spec-check` gate.

## Boot

- [`boot.md`](boot.md) — boot 叶阶段 namespace 与直接 parent 映射
- `boot/core-prepare.md`
- `boot/mm-core-init.md`
- `boot/sched-init.md`

## Interrupt

- [`interrupt.md`](interrupt.md) — interrupt 叶阶段 namespace 与直接 parent 映射
- `interrupt/irq-time-init.md`
- `interrupt/local-irq-enable.md`
- `interrupt/irq-open-prepare.md`
- `interrupt/process-prepare.md`

## BootInitFlow owner

- [`../flows/boot_init_flow/README.md`](../flows/boot_init_flow/README.md) — BootInitFlow 总体 lowering 约束
- `../flows/boot_init_flow/preset.md`
- `../flows/boot_init_flow/setup.md`
- `../flows/boot_init_flow/enable.md`

## KernelInitFlow execution phases

- [`smp-runtime.md`](smp-runtime.md) — KernelInitFlow 直接子阶段映射
- `smp-runtime/pre-smp-init.md`
- `smp-runtime/smp-bringup.md`
- `smp-runtime/runtime-core.md`
- `smp-runtime/initcall.md`
- `smp-runtime/rootfs.md`
- `smp-runtime/finalize.md`

## Payload

- [`payload.md`](payload.md) — 两个 payload 叶阶段与 commit action 映射
