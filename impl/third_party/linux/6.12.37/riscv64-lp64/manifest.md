# Linux 6.12.37 RISC-V LP64 Third-Party Objects

This directory contains Linux-built binary inputs used by the ArceOS-ex
cross-validation work. These files are treated as third-party inputs, not as
ArceOS-ex source files.

## Source Tree

- Local source tree: `/home/cloud/gitStudy/linux-6.12.37`
- Source tree VCS status: not a Git working tree in this local copy
- Captured on: 2026-06-14
- Config snapshot: `config`

## Object: irq-sifive-plic.o

- Source object:
  `/home/cloud/gitStudy/linux-6.12.37/drivers/irqchip/irq-sifive-plic.o`
- Stored object:
  `objects/drivers/irqchip/irq-sifive-plic.o`
- SHA-256:
  `fbc3c16758f3a086af25e695cc20a4d56c94cee49faddb0553c0325e7429ca81`
- Size: 39784 bytes
- ELF class: ELF64, little-endian
- ELF type: relocatable
- Machine: RISC-V
- Flags: `0x1, RVC, soft-float ABI`
- Toolchain comment: `GCC: (Ubuntu 13.3.0-6ubuntu2~24.04) 13.3.0`
- Global defined symbols: none from
  `riscv64-linux-gnu-nm -g --defined-only`

## Config Snapshot

- Stored config: `config`
- SHA-256:
  `5a8c8ca1cfbeb9b445004432e30f9cabd93b44b896765481e69a617cb7cf5f96`
- Relevant options observed in the snapshot:
  - `CONFIG_RISCV=y`
  - `CONFIG_SMP=y`
  - `CONFIG_HOTPLUG_CPU=y`
  - `CONFIG_RISCV_ALTERNATIVE=y`
  - `CONFIG_STACKPROTECTOR=y`
  - `CONFIG_STACKPROTECTOR_STRONG=y`
  - `CONFIG_OF=y`
  - `CONFIG_OF_IRQ=y`
  - `CONFIG_IRQCHIP=y`
  - `CONFIG_RISCV_INTC=y`
  - `CONFIG_SIFIVE_PLIC=y`
  - `CONFIG_FTRACE=y`

## Important Sections

The object has Linux-specific sections that must be handled by the ArceOS-ex
linker script and Linux ABI shim:

| Section | Size | Notes |
| --- | ---: | --- |
| `.text` | `0x1330` | Driver runtime code. |
| `.init.text` | `0x003c` | Linux init-only code, including the platform driver init path. |
| `.alternative` | `0x0050` | RISC-V alternatives metadata. |
| `__bug_table` | `0x0030` | Linux bug table records. |
| `.data..percpu` | `0x0040` | Linux per-CPU data used by the object. |
| `.data..ro_after_init` | `0x0005` | Linux ro-after-init data. |
| `.initcall6.init` | `0x0008` | Linux level-6 initcall pointer. |
| `__irqchip_of_table` | `0x00c8` | Early irqchip table entry for the special Allwinner compatible. |

The `.initcall6.init` entry is a Linux `initcall_t` function pointer and must
not be mixed with the existing Rust `InitcallEntry` section format.

## Local Symbol Summary

Selected local symbols that define the upper interface shape:

- `plic_driver`
- `plic_driver_init`
- `plic_platform_probe`
- `plic_probe`
- `plic_match`
- `plic_irqdomain_ops`
- `plic_chip`
- `plic_edge_chip`
- `plic_handle_irq`
- `plic_starting_cpu`
- `plic_dying_cpu`
- `plic_irq_syscore_ops`

## Undefined Symbols

Undefined symbols observed with
`riscv64-linux-gnu-nm -u objects/drivers/irqchip/irq-sifive-plic.o`:

```text
___ratelimit
__cpu_online_mask
__cpu_present_mask
__cpuhp_setup_state
__irq_set_handler
__kmalloc_cache_noprof
__kmalloc_noprof
__mmiowb_state
__per_cpu_offset
__platform_driver_register
__raw_spin_lock_init
__stack_chk_fail
_printk
_raw_spin_lock_irqsave
_raw_spin_unlock_irqrestore
bitmap_free
bitmap_zalloc
cpu_bit_bitmap
devm_platform_ioremap_resource
disable_percpu_irq
enable_percpu_irq
generic_handle_domain_irq
handle_edge_irq
handle_fasteoi_irq
iounmap
irq_create_mapping_affinity
irq_domain_free_irqs_top
irq_domain_instantiate
irq_domain_set_info
irq_domain_translate_onecell
irq_domain_translate_twocell
irq_find_matching_fwspec
irq_get_irq_data
irq_modify_status
irq_set_affinity
kfree
kmalloc_caches
nr_cpu_ids
of_fwnode_ops
of_iomap
of_irq_count
of_irq_parse_one
of_match_node
of_property_read_variable_u32_array
register_syscore_ops
riscv_get_intc_hwnode
riscv_hartid_to_cpuid
riscv_of_parent_hartid
```

Initial shim classification:

- Platform/driver registration:
  `__platform_driver_register`
- IRQ core/domain/chip:
  `__irq_set_handler`, `generic_handle_domain_irq`,
  `handle_edge_irq`, `handle_fasteoi_irq`,
  `irq_create_mapping_affinity`, `irq_domain_free_irqs_top`,
  `irq_domain_instantiate`, `irq_domain_set_info`,
  `irq_domain_translate_onecell`, `irq_domain_translate_twocell`,
  `irq_find_matching_fwspec`, `irq_get_irq_data`, `irq_modify_status`,
  `irq_set_affinity`
- OF/fwnode/platform resources:
  `devm_platform_ioremap_resource`, `of_fwnode_ops`, `of_iomap`,
  `of_irq_count`, `of_irq_parse_one`, `of_match_node`,
  `of_property_read_variable_u32_array`
- RISC-V integration:
  `riscv_get_intc_hwnode`, `riscv_hartid_to_cpuid`,
  `riscv_of_parent_hartid`
- CPU/per-CPU/cpumask:
  `__cpu_online_mask`, `__cpu_present_mask`, `__cpuhp_setup_state`,
  `__per_cpu_offset`, `cpu_bit_bitmap`, `disable_percpu_irq`,
  `enable_percpu_irq`, `nr_cpu_ids`
- Allocation/bitmap:
  `__kmalloc_cache_noprof`, `__kmalloc_noprof`, `bitmap_free`,
  `bitmap_zalloc`, `kfree`, `kmalloc_caches`
- Locking/barriers:
  `__mmiowb_state`, `__raw_spin_lock_init`,
  `_raw_spin_lock_irqsave`, `_raw_spin_unlock_irqrestore`
- Diagnostics/fallback:
  `___ratelimit`, `_printk`, `__stack_chk_fail`,
  `register_syscore_ops`

The first Linux-object provider milestone should define ABI-correct fallback
symbols for every undefined symbol above. Functional semantics are added only
when the boot path reaches a symbol that is required for the current validation
layer.
