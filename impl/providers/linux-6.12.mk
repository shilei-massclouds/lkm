# Linux 6.12 provider registry.
#
# Expected sibling project layout:
#
#   <parent>/
#     linux-6.12/
#     lkm/
#
# From this repository, the default Linux provider tree is ../linux-6.12.
# The objects listed here are external build outputs from that sibling tree;
# they are not vendored into lkm.
#
# Current object metadata for drivers/irqchip/irq-sifive-plic.o:
# - SHA-256: fbc3c16758f3a086af25e695cc20a4d56c94cee49faddb0553c0325e7429ca81
# - ELF: ELF64 little-endian RISC-V relocatable, RVC, soft-float ABI.
# - Toolchain comment captured from the original baseline:
#   GCC: (Ubuntu 13.3.0-6ubuntu2~24.04) 13.3.0
# - Global defined symbols: none from riscv64-linux-gnu-nm -g --defined-only.
# - Important Linux sections that must be retained by the linker script:
#   .init.text, .alternative, __bug_table, .data..percpu,
#   .data..ro_after_init, .initcall6.init, __irqchip_of_table.
# - Selected local symbols shaping the upper interface:
#   plic_driver, plic_driver_init, plic_platform_probe, plic_probe,
#   plic_match, plic_irqdomain_ops, plic_chip, plic_edge_chip,
#   plic_handle_irq, plic_starting_cpu, plic_dying_cpu,
#   plic_irq_syscore_ops.
# - Undefined symbol classes handled by the Linux ABI shim:
#   platform driver registration; IRQ core/domain/chip; OF/fwnode/resource
#   lookup; RISC-V hart/INTC integration; CPU/per-CPU/cpumask; allocation and
#   bitmap helpers; spinlock/barrier helpers; printk/ratelimit/stack-protector
#   diagnostics.

LINUX_PROVIDER_DIR ?= $(abspath $(ROOT)/../linux-6.12)

PROVIDER_NAMES += linux-object
PROVIDER_linux-object_KIND := plic
PROVIDER_linux-object_CFG := plic_provider_linux_object
PROVIDER_linux-object_OBJECTS := $(LINUX_PROVIDER_DIR)/drivers/irqchip/irq-sifive-plic.o
