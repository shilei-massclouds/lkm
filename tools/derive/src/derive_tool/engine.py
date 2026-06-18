"""Minimal static derivation engine for LKM object models."""

from __future__ import annotations

import re

from common.defaults import DEFAULT_TARGET
from common.derive_types import (
    DerivationRecord,
    DerivationResult,
    DerivationStatus,
    DerivationTraceNode,
    EventTransition,
)
from common.model_types import EventDef, ObjectDef, ObjectModel, StateDef
from common.spec_ast import Block, SourceSpan, WithinDecl


_TARGET_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)\Z"
)
_STATE_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.state\s*==\s*State::([A-Za-z_][A-Za-z0-9_]*)\Z"
)
_EVENT_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_ACTION_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_CHILD_EVENT_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.([a-z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_CHILD_ACTION_EXPR_RE = re.compile(
    r"\A([A-Z][A-Za-z0-9_]*)\.([a-z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_ACTION_BIND_RE = re.compile(
    r"\Alet\s+([a-z][A-Za-z0-9_]*)\s*:\s*([A-Z][A-Za-z0-9_]*)\s*<-\s*"
    r"([A-Za-z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_REF_EVENT_EXPR_RE = re.compile(
    r"\A([a-z][A-Za-z0-9_]*)\.Event::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_REF_ACTION_EXPR_RE = re.compile(
    r"\A([a-z][A-Za-z0-9_]*)\.Action::([A-Za-z_][A-Za-z0-9_]*)(?:\s*\((.*)\))?\Z",
    re.S,
)
_TYPE_EVENT_RE_TEMPLATE = r"\bEvent::{}\b"
_REF_TARGET_PROCESS_TYPES = {
    "RunQueueRef": "RunQueue",
    "TaskRef": "Task",
}
_REF_SELF_ACTION_TYPES = {
    "TaskRef",
}
_PREDICATE_CALL_RE = re.compile(r"\A([A-Za-z_][A-Za-z0-9_]*)\s*\(")
_RELATION_RE = re.compile(r"(==|!=|>=|<=|>|<)")
_HAS_SLOT_RE = re.compile(
    r"\Ahas_slot\(\s*Config\.fixmap\s*,\s*FixMapSlot::([A-Za-z_][A-Za-z0-9_]*)\s*\)\Z"
)
_NO_SERVICE_RE = re.compile(r"\Ano_service\(\s*([A-Z][A-Za-z0-9_]*)\s*\)\Z")
_SLOT_ENTRY_RE = re.compile(r"\A([A-Za-z_][A-Za-z0-9_]*)\s*:\s*FixMapSlot(?:Range)?<")

_AUTO_PREDICATES = {
    "aligned": "alignment",
    "has_slot": "config_structure",
    "inside": "range",
    "no_service": "state_alias",
    "page_aligned": "alignment",
    "readonly": "object_attribute",
}
_ASSUMPTION_PREDICATES = {
}
_ATTRS_ACCESSIBLE_PROOFS = {
    "Config": ("config_attributes", "config_source_candidate"),
    "FixMap": ("fixmap_layout", "config_source_candidate"),
    "LinearMap": ("linear_map_layout", "config_source_candidate"),
    "Lds": ("linker_layout", "linker_script_candidate"),
    "BootArgs": ("boot_arguments", "boot_protocol_candidate"),
    "PhysicalMemory": ("platform_memory_layout", "fdt_candidate"),
    "Riscv64": ("architecture_register_file", "riscv_isa_spec_candidate"),
    "StaticObjects": ("static_object_layout", "linker_symbol_candidate"),
}
_BOOT_PROTOCOL_PROOFS = {
    "attrs_accessible(self)": ("boot_arguments", "riscv_boot_protocol"),
    "boot_hartid == Riscv64.a0": ("boot_arguments", "riscv_boot_protocol"),
    "dtb_pa == Riscv64.a1": ("boot_arguments", "riscv_boot_protocol"),
}
_CONFIG_SOURCE_PROOFS = {
    "attrs_accessible(self)": ("config_attributes", "config_source"),
    "page_size > 0": ("configuration", "config_source"),
    "pmd_size >= page_size": ("configuration", "config_source"),
    "aligned(pmd_size, page_size)": ("configuration", "config_source"),
    "pt_size_on_stack > 0": ("configuration", "config_source"),
    "pt_size_on_stack < page_size": ("configuration", "config_source"),
    "boot_stack_size >= page_size": ("configuration", "config_source"),
    "aligned(boot_stack_size, page_size)": ("configuration", "config_source"),
    "kernel_link_addr != 0": ("configuration", "config_source"),
    "page_aligned(kernel_link_addr)": ("configuration", "config_source"),
    "valid_virt_addr(kernel_link_addr)": ("address_mapping", "config_source"),
    "kernel_image_va_window_size > 0": ("configuration", "config_source"),
    "kernel_image_va_window_size >= pmd_size": ("configuration", "config_source"),
    "valid_satp_mode(satp_mode)": ("configuration", "config_source"),
    "valid_fixmap_config(fixmap)": ("configuration", "config_source"),
}
_LDS_LINKER_PROOFS = {
    "attrs_accessible(self)": ("linker_layout", "linux_linker_script"),
    "global_pointer != 0": ("linker_layout", "linux_linker_script"),
    "kernel_start != 0": ("linker_layout", "linux_linker_script"),
    "text_start == kernel_start": ("linker_layout", "linux_linker_script"),
    "text_end > text_start": ("linker_layout", "linux_linker_script"),
    "rodata_end >= rodata_start": ("linker_layout", "linux_linker_script"),
    "data_end >= data_start": ("linker_layout", "linux_linker_script"),
    "elf_entry == kernel_start": ("linker_layout", "linux_linker_script"),
    "kernel_end > kernel_start": ("linker_layout", "linux_linker_script"),
    "entry_head_text_layout_ready(Lds)": (
        "linker_layout",
        "linux_linker_script",
    ),
    "pre_mmu_access_discipline_ready(Lds)": (
        "linker_layout",
        "linux_linker_script",
    ),
    "trampoline_access_discipline_ready(Lds)": (
        "linker_layout",
        "linux_linker_script",
    ),
    "inside(text_start, text_end, kernel_start, kernel_end)": (
        "linker_layout",
        "linux_linker_script",
    ),
    "inside(rodata_start, rodata_end, kernel_start, kernel_end)": (
        "linker_layout",
        "linux_linker_script",
    ),
    "inside(data_start, data_end, kernel_start, kernel_end)": (
        "linker_layout",
        "linux_linker_script",
    ),
    "bss_start != 0": ("linker_layout", "linux_linker_script"),
    "bss_end > bss_start": ("linker_layout", "linux_linker_script"),
    "inside(bss_start, bss_end, kernel_start, kernel_end)": (
        "linker_layout",
        "linux_linker_script",
    ),
    "per_cpu_start != 0": ("linker_layout", "linux_linker_script"),
    "per_cpu_end > per_cpu_start": ("linker_layout", "linux_linker_script"),
    "per_cpu_load != 0": ("linker_layout", "linux_linker_script"),
    "per_cpu_static_image_layout_ready(Lds)": (
        "linker_layout",
        "linux_linker_script",
    ),
    "init_stack_start != 0": ("linker_layout", "linux_linker_script"),
    "init_stack_end > init_stack_start": ("linker_layout", "linux_linker_script"),
    "page_aligned(init_stack_start)": ("linker_layout", "linux_linker_script"),
    "page_aligned(init_stack_end)": ("linker_layout", "linux_linker_script"),
    "boot_stack_size == Config.boot_stack_size": (
        "stack_layout",
        "config_driven_linker_script",
    ),
    "init_stack_end - init_stack_start == boot_stack_size": (
        "stack_layout",
        "linux_linker_script",
    ),
}
_KERNEL_IMAGE_LINKER_PROOFS = {
    "valid_segment_set(segments)": ("linker_layout", "linux_linker_script"),
    "segments.bss.range == range(Lds.bss_start, Lds.bss_end)": (
        "linker_layout",
        "linux_linker_script",
    ),
    "inside(segments.bss.range.start, segments.bss.range.end, start, end)": (
        "linker_layout",
        "linux_linker_script",
    ),
}
_LINEAR_MAP_LAYOUT_PROOFS = {
    "linear_map_area_reserved(self)": ("address_layout", "config_address_layout"),
    "fixmap_adjacent_to_linear_map(FixMap, LinearMap)": (
        "address_layout",
        "config_address_layout",
    ),
}
_STATIC_OBJECT_PROOFS = {
    "attrs_accessible(self)": ("static_object_layout", "linux_static_objects"),
    "valid_object_storage(init_task)": (
        "object_storage",
        "linux_static_objects",
    ),
    "valid_function_symbol(early_event_entry)": (
        "linker_symbol",
        "linux_static_objects",
    ),
    "valid_function_symbol(formal_event_entry)": (
        "linker_symbol",
        "linux_static_objects",
    ),
    "valid_page_table_storage(trampoline_pg_dir)": (
        "object_storage",
        "linux_static_objects",
    ),
    "valid_page_table_storage(early_pg_dir)": (
        "object_storage",
        "linux_static_objects",
    ),
    "valid_page_table_storage(swapper_pg_dir)": (
        "object_storage",
        "linux_static_objects",
    ),
}
_PHYSICAL_MEMORY_PROOFS = {
    "attrs_accessible(self)": ("platform_memory_layout", "fdt_memory_layout"),
    "valid_phys_range_set(ram)": ("platform", "fdt_memory_layout"),
    "valid_phys_range_set(iomap)": ("platform", "fdt_memory_layout"),
    "disjoint(ram, iomap)": (
        "platform_memory_layout",
        "fdt_memory_layout",
    ),
}
_EXTERNAL_SOURCE_PROOFS = {
    **{
        ("Config", "config::entry_prelude", expression): proof
        for expression, proof in _CONFIG_SOURCE_PROOFS.items()
    },
    **{
        ("Lds", "linker::linux_6_12_37", expression): proof
        for expression, proof in _LDS_LINKER_PROOFS.items()
    },
    **{
        ("StaticObjects", "static::linux_6_12_37", expression): proof
        for expression, proof in _STATIC_OBJECT_PROOFS.items()
    },
    **{
        ("PhysicalMemory", "fdt::memory", expression): proof
        for expression, proof in _PHYSICAL_MEMORY_PROOFS.items()
    },
    ("Riscv64", "external_spec::riscv_isa", "attrs_accessible(self)"): (
        "architecture_register_file",
        "riscv_isa_spec",
    ),
    ("SbiSpec", "external_spec::riscv_sbi", "sbi_hsm_available()"): (
        "sbi_hsm",
        "riscv_sbi_spec",
    ),
    (
        "OpenSbiFirmware",
        "firmware::opensbi",
        "ordered_booting_enabled()",
    ): (
        "firmware_boot_policy",
        "opensbi_firmware",
    ),
    (
        "OpenSbiFirmware",
        "firmware::opensbi",
        "primary_hart_only_at_kernel_entry()",
    ): (
        "firmware_entry_state",
        "opensbi_firmware",
    ),
    (
        "OpenSbiFirmware",
        "firmware::opensbi",
        "primary_hart_sie_clear_at_kernel_entry()",
    ): (
        "firmware_entry_state",
        "opensbi_firmware",
    ),
    (
        "OpenSbiFirmware",
        "firmware::opensbi",
        "firmware_dtb_blob_in_ram_at_kernel_entry(BootArgs.dtb_pa)",
    ): (
        "firmware_entry_state",
        "opensbi_firmware",
    ),
    (
        "OpenSbiFirmware",
        "firmware::opensbi",
        "firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa)",
    ): (
        "firmware_entry_state",
        "opensbi_firmware",
    ),
    (
        "OpenSbiFirmware",
        "firmware::opensbi",
        "firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa)",
    ): (
        "firmware_entry_state",
        "opensbi_firmware",
    ),
    (
        "PlatformCpuInfo",
        "fdt::cpus",
        "platform_hart_id_valid(BootArgs.boot_hartid)",
    ): (
        "platform_cpu_description",
        "fdt_cpu_description",
    ),
}
_EXTERNAL_PREDICATES = {
    "context_is": "system_exclusive_context",
    "disjoint": "platform_memory_layout",
    "fits_in_fixmap_slot": "address_mapping",
    "fits_in_kernel_image_map": "address_mapping",
    "fixmap_slot_accessible": "address_mapping",
    "fixmap_slot_mapping_ready": "address_mapping",
    "firmware_dtb_blob_in_ram_at_kernel_entry": "firmware_entry_state",
    "firmware_dtb_blob_complete_at_kernel_entry": "firmware_entry_state",
    "firmware_dtb_blob_accessible_at_kernel_entry": "firmware_entry_state",
    "fixmap_adjacent_to_linear_map": "address_layout",
    "gp_relative_access_ready": "architecture_state",
    "kernel_fpu_disabled": "riscv_status_register",
    "kernel_image_accessible": "address_mapping",
    "kernel_image_mapping_ready": "address_mapping",
    "kernel_cmdline_ready": "boot_input",
    "interrupt_concurrency_closed": "system_exclusive_context",
    "early_dtb_parse_ready": "boot_input",
    "early_ioremap_slots_ready": "address_mapping",
    "early_params_dispatched": "boot_parameter",
    "earlycon_backend_online": "console",
    "earlycon_sbi_backend_ready": "console",
    "earlycon_sbi_config_ready": "boot_parameter",
    "init_mm_bounds_ready": "task_address_space",
    "init_stack_canary_ready": "stack_protection",
    "init_task_active_mm_ready": "task_address_space",
    "linear_map_area_reserved": "address_layout",
    "linux_banner_buffered": "console",
    "kernel_vector_disabled": "riscv_status_register",
    "memblock_allocator_ready": "physical_memory_management",
    "memblock_candidate_ranges_ready": "fdt_memory_layout",
    "memblock_reserved_ranges_ready": "physical_memory_management",
    "memblock_resize_allowed": "physical_memory_management",
    "memory_zeroed": "memory_content",
    "ordered_booting_enabled": "firmware_boot_policy",
    "platform_hart_id_valid": "platform_cpu_description",
    "phys_to_virt_transition_completed": "architecture_state",
    "printk_buffer_flushed_to_earlycon": "console",
    "printk_buffer_ready": "console",
    "primary_hart_only_at_kernel_entry": "firmware_entry_state",
    "primary_hart_sie_clear_at_kernel_entry": "firmware_entry_state",
    "sbi_capability_view_ready": "sbi_capability",
    "slot_contains": "fixmap_slot_content",
    "soc_early_platform_ready": "platform",
    "sbi_hsm_available": "sbi_hsm",
    "supervisor_interrupts_disabled": "riscv_status_register",
    "swapper_vm_current": "address_mapping",
    "swapper_vm_mappings_ready": "address_mapping",
    "task_concurrency_closed": "system_exclusive_context",
    "temporary_fixmap_page_table_slots_clean": "address_mapping",
    "trampoline_mapping_ready": "address_mapping",
    "valid_dtb_header": "boot_input",
    "valid_dtb_magic": "boot_input",
    "valid_function_symbol": "linker_symbol",
    "valid_hart_id": "boot_hart_identity",
    "boot_cpu_active": "cpu_state",
    "boot_cpu_hartid_ready": "boot_hart_identity",
    "boot_cpu_online": "cpu_state",
    "boot_cpu_present": "cpu_state",
    "cpu_id_map_ready": "cpu_topology",
    "valid_object_storage": "object_storage",
    "valid_page_table_storage": "object_storage",
    "valid_phys_range_set": "platform",
    "valid_fixmap_config": "configuration",
    "valid_satp_mode": "configuration",
    "valid_segment_set": "linker_layout",
    "valid_stack_pointer": "architecture_state",
    "valid_task_ref": "object_storage",
    "valid_task_storage": "object_storage",
    "valid_trampoline_map": "address_mapping",
    "valid_virt_addr": "address_mapping",
}
_DERIVED_PROVIDERS = {
    "context_is": "prior_derivation_facts",
    "disjoint": "fdt_candidate",
    "kernel_fpu_disabled": "isa_spec_and_boot_code",
    "kernel_vector_disabled": "isa_spec_and_boot_code",
    "fixmap_adjacent_to_linear_map": "config_source_candidate",
    "fixmap_slot_accessible": "prior_derivation_facts",
    "fixmap_slot_mapping_ready": "boot_code_candidate",
    "fits_in_fixmap_slot": "config_source_candidate",
    "fits_in_kernel_image_map": "config_source_candidate",
    "firmware_dtb_blob_in_ram_at_kernel_entry": "opensbi_firmware",
    "firmware_dtb_blob_complete_at_kernel_entry": "opensbi_firmware",
    "firmware_dtb_blob_accessible_at_kernel_entry": "opensbi_firmware",
    "interrupt_concurrency_closed": "prior_derivation_facts",
    "kernel_image_accessible": "prior_derivation_facts",
    "kernel_image_mapping_ready": "boot_code_candidate",
    "kernel_cmdline_ready": "fdt_candidate",
    "early_dtb_parse_ready": "fdt_candidate",
    "early_ioremap_slots_ready": "boot_code_candidate",
    "early_params_dispatched": "boot_code_candidate",
    "earlycon_backend_online": "boot_code_candidate",
    "earlycon_sbi_backend_ready": "sbi_capability",
    "earlycon_sbi_config_ready": "boot_parameter",
    "init_mm_bounds_ready": "boot_code_candidate",
    "init_stack_canary_ready": "boot_code_candidate",
    "init_task_active_mm_ready": "boot_code_candidate",
    "linear_map_area_reserved": "config_source_candidate",
    "linux_banner_buffered": "boot_code_candidate",
    "memblock_allocator_ready": "boot_code_candidate",
    "memblock_candidate_ranges_ready": "fdt_candidate",
    "memblock_reserved_ranges_ready": "boot_code_candidate",
    "memblock_resize_allowed": "boot_code_candidate",
    "printk_buffer_flushed_to_earlycon": "boot_code_candidate",
    "printk_buffer_ready": "boot_code_candidate",
    "gp_relative_access_ready": "prior_derivation_facts",
    "memory_zeroed": "boot_code_candidate",
    "phys_to_virt_transition_completed": "prior_derivation_facts",
    "primary_hart_only_at_kernel_entry": "opensbi_firmware",
    "primary_hart_sie_clear_at_kernel_entry": "opensbi_firmware",
    "slot_contains": "prior_derivation_facts",
    "sbi_hsm_available": "riscv_sbi_spec",
    "sbi_capability_view_ready": "riscv_sbi_and_firmware",
    "supervisor_interrupts_disabled": "isa_spec_and_boot_code",
    "swapper_vm_current": "prior_derivation_facts",
    "swapper_vm_mappings_ready": "boot_code_candidate",
    "task_concurrency_closed": "prior_derivation_facts",
    "temporary_fixmap_page_table_slots_clean": "boot_code_candidate",
    "ordered_booting_enabled": "opensbi_firmware",
    "platform_hart_id_valid": "fdt_cpu_description",
    "trampoline_mapping_ready": "boot_code_candidate",
    "valid_trampoline_map": "config_and_linker_candidate",
    "valid_fixmap_config": "config_source_candidate",
    "valid_dtb_header": "boot_code_candidate",
    "valid_dtb_magic": "boot_code_candidate",
    "valid_function_symbol": "linker_symbol_candidate",
    "valid_hart_id": "fdt_and_boot_protocol",
    "valid_object_storage": "linker_symbol_candidate",
    "valid_page_table_storage": "linker_symbol_candidate",
    "valid_phys_range_set": "fdt_candidate",
    "valid_satp_mode": "config_source_candidate",
    "valid_virt_addr": "config_source_candidate",
    "valid_segment_set": "linker_script_candidate",
    "valid_stack_pointer": "prior_derivation_facts",
    "valid_task_ref": "prior_derivation_facts",
    "valid_task_storage": "linker_symbol_candidate",
    "soc_early_platform_ready": "fdt_and_platform_candidate",
}
_CONTAINS_PROOFS = {
    "contains(PhysicalMemory.ram, header_range)": (
        "dtb_header_range",
        "opensbi_firmware",
    ),
    "contains(PhysicalMemory.ram, range)": (
        "physical_memory_membership",
        "opensbi_firmware",
    ),
}
_RELATION_PROOFS = {
    "boot_hartid == Riscv64.a0": (
        "boot_arguments",
        "boot_protocol_candidate",
    ),
    "page_size > 0": (
        "configuration",
        "config_source_candidate",
    ),
    "pmd_size >= page_size": (
        "configuration",
        "config_source_candidate",
    ),
    "aligned(pmd_size, page_size)": (
        "configuration",
        "config_source_candidate",
    ),
    "pt_size_on_stack > 0": (
        "configuration",
        "config_source_candidate",
    ),
    "pt_size_on_stack < page_size": (
        "configuration",
        "config_source_candidate",
    ),
    "kernel_link_addr != 0": (
        "configuration",
        "config_source_candidate",
    ),
    "page_aligned(kernel_link_addr)": (
        "configuration",
        "config_source_candidate",
    ),
    "kernel_image_va_window_size > 0": (
        "configuration",
        "config_source_candidate",
    ),
    "kernel_image_va_window_size >= pmd_size": (
        "configuration",
        "config_source_candidate",
    ),
    "dtb_pa == Riscv64.a1": (
        "boot_arguments",
        "boot_protocol_candidate",
    ),
    "global_pointer != 0": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "kernel_start != 0": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "text_end > text_start": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "rodata_end >= rodata_start": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "data_end >= data_start": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "kernel_end > kernel_start": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "bss_start != 0": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "bss_end > bss_start": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "inside(text_start, text_end, kernel_start, kernel_end)": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "inside(rodata_start, rodata_end, kernel_start, kernel_end)": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "inside(data_start, data_end, kernel_start, kernel_end)": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "inside(bss_start, bss_end, kernel_start, kernel_end)": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "init_stack_start != 0": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "init_stack_end > init_stack_start": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "page_aligned(init_stack_start)": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "page_aligned(init_stack_end)": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "segments.bss.range == range(Lds.bss_start, Lds.bss_end)": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "inside(segments.bss.range.start, segments.bss.range.end, start, end)": (
        "linker_layout",
        "linker_script_candidate",
    ),
    "Lds.init_stack_end - Lds.init_stack_start >= Config.page_size": (
        "stack_layout",
        "config_and_linker_candidate",
    ),
    "inside(Riscv64.sp, Lds.init_stack_end, Lds.init_stack_start, Lds.init_stack_end)": (
        "stack_layout",
        "prior_derivation_facts",
    ),
    "inside(Riscv64.sp, virt_addr(Lds.init_stack_end, EarlyVm, KernelImageMap), virt_addr(Lds.init_stack_start, EarlyVm, KernelImageMap), virt_addr(Lds.init_stack_end, EarlyVm, KernelImageMap))": (
        "stack_layout",
        "prior_derivation_facts",
    ),
    "header_range.start == BootArgs.dtb_pa": (
        "dtb_header_range",
        "boot_code_candidate",
    ),
    "header_range.end == BootArgs.dtb_pa + size_of::<DtbHeader>()": (
        "dtb_header_range",
        "boot_code_candidate",
    ),
    "range.start == BootArgs.dtb_pa": (
        "dtb_range",
        "boot_code_candidate",
    ),
    "range.end == BootArgs.dtb_pa + header.total_size": (
        "dtb_range",
        "boot_code_candidate",
    ),
    "fdt_slot == Config.fixmap.fdt": (
        "fixmap_layout",
        "config_source_candidate",
    ),
    "Riscv64.sie == 0": (
        "register_effect",
        "prior_derivation_facts",
    ),
    "Riscv64.sip == 0": (
        "register_effect",
        "prior_derivation_facts",
    ),
    "Riscv64.stvec == phys_addr(StaticObjects.early_event_entry)": (
        "register_effect",
        "prior_derivation_facts",
    ),
    "Riscv64.stvec == virt_addr(StaticObjects.formal_event_entry, EarlyVm, KernelImageMap)": (
        "register_effect",
        "prior_derivation_facts",
    ),
    "Riscv64.sscratch == 0": (
        "register_effect",
        "prior_derivation_facts",
    ),
    "Riscv64.tp == phys_addr(StaticObjects.init_task)": (
        "register_effect",
        "prior_derivation_facts",
    ),
    "Riscv64.tp == virt_addr(StaticObjects.init_task, EarlyVm, KernelImageMap)": (
        "register_effect",
        "prior_derivation_facts",
    ),
    "Riscv64.sp == phys_addr(Lds.init_stack_end - Config.pt_size_on_stack)": (
        "register_effect",
        "prior_derivation_facts",
    ),
    "Riscv64.sp == virt_addr(Lds.init_stack_end - Config.pt_size_on_stack, EarlyVm, KernelImageMap)": (
        "register_effect",
        "prior_derivation_facts",
    ),
    "Riscv64.gp == phys_addr(Lds.global_pointer)": (
        "register_effect",
        "prior_derivation_facts",
    ),
    "Riscv64.gp == virt_addr(Lds.global_pointer, EarlyVm, KernelImageMap)": (
        "register_effect",
        "prior_derivation_facts",
    ),
    "Riscv64.satp == satp_of(StaticObjects.early_pg_dir, Config.satp_mode)": (
        "register_effect",
        "prior_derivation_facts",
    ),
}


def derive(model: ObjectModel, target: str = DEFAULT_TARGET) -> DerivationResult:
    """Derive a target event from the model's declared initial states."""

    return _Deriver(model, target).run()


def summarize_derivation(result: DerivationResult) -> str:
    """Return a compact derivation summary."""

    counts = _record_counts(result.records)
    if result.ok:
        status = "ok"
    elif result.contradictions:
        status = "contradiction"
    elif result.blocked:
        status = "blocked"
    else:
        status = "incomplete"

    lines = [
        f"derive: {status}",
        f"target: {result.target}",
        f"target_reached: {'yes' if result.target_reached else 'no'}",
        f"transitions: {len(result.transitions)}",
    ]
    for status_name in (
        DerivationStatus.PROVED,
        DerivationStatus.ASSUMED,
        DerivationStatus.OBLIGATION,
        DerivationStatus.DEFERRED,
        DerivationStatus.BLOCKED,
        DerivationStatus.CONTRADICTION,
    ):
        lines.append(f"{status_name.value}: {counts.get(status_name, 0)}")
    return "\n".join(lines)


def render_derivation_text(result: DerivationResult) -> str:
    """Render a human-readable derivation report."""

    lines = [summarize_derivation(result)]

    if result.trace:
        lines.append("")
        lines.append("trace:")
        for node in result.trace:
            _append_trace_node(lines, node, depth=0)
    elif result.transitions:
        lines.append("")
        lines.append("transitions:")
        for transition in result.transitions:
            lines.append(f"- {transition.label}")

    for status in (
        DerivationStatus.BLOCKED,
        DerivationStatus.CONTRADICTION,
        DerivationStatus.DEFERRED,
        DerivationStatus.OBLIGATION,
    ):
        records = [record for record in result.records if record.status is status]
        if not records:
            continue
        lines.append("")
        lines.append(f"{status.value}:")
        if status is DerivationStatus.OBLIGATION:
            lines.extend(_format_obligation_category_summary(records))
            lines.extend(_format_obligation_provider_summary(records))
        for record in records:
            lines.append(f"- {_format_record(record)}")

    return "\n".join(lines)


class _Deriver:
    def __init__(self, model: ObjectModel, target: str) -> None:
        self.model = model
        self.target = target
        self.states: dict[str, str] = {}
        self.records: list[DerivationRecord] = []
        self.transitions: list[EventTransition] = []
        self.trace: list[DerivationTraceNode] = []
        self.trace_stack: list[_TraceFrame] = []
        self.stack: list[tuple[str, str]] = []
        self.process_stack: list[tuple[str, str, str, str]] = []
        self.state_validation_events: dict[tuple[str, str], EventDef] = {}
        self.validated_states: set[tuple[str, str]] = set()
        self.proved_expressions: set[str] = set()

    def run(self) -> DerivationResult:
        target_object, target_event = self._parse_target()
        target_state = self._target_state(target_object, target_event)
        self._initialize_states()

        if target_object is not None and target_event is not None:
            self._derive_event(target_object, target_event)

        return DerivationResult(
            target=self.target,
            target_object=target_object,
            target_event=target_event,
            target_state=target_state,
            states=dict(self.states),
            records=tuple(self.records),
            transitions=tuple(self.transitions),
            trace=tuple(self.trace),
        )

    def _parse_target(self) -> tuple[str | None, str | None]:
        match = _TARGET_RE.match(self.target)
        if match is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"invalid target event: {self.target}",
            )
            return None, None
        return match.group(1), match.group(2)

    def _initialize_states(self) -> None:
        for obj in self.model.objects.values():
            if obj.initial_state is None:
                continue
            self.states[obj.name] = obj.initial_state
            self._record(
                DerivationStatus.ASSUMED,
                f"initial state: {obj.name}.state == State::{obj.initial_state}",
                obj.decl.span,
                object_name=obj.name,
                state_name=obj.initial_state,
            )

        for obj in self.model.objects.values():
            if obj.initial_state is not None:
                self._validate_state(obj.name, obj.initial_state)

    def _target_state(
        self, object_name: str | None, event_name: str | None
    ) -> str | None:
        if object_name is None or event_name is None:
            return None
        obj = self.model.objects.get(object_name)
        if obj is None:
            return None
        event = _find_event(obj, event_name)
        if event is None:
            return None
        return event.target_state

    def _derive_event(self, object_name: str, event_name: str) -> bool:
        key = (object_name, event_name)
        if key in self.stack:
            self._record(
                DerivationStatus.BLOCKED,
                f"recursive event cycle: {_event_label(object_name, event_name)}",
                object_name=object_name,
                event_name=event_name,
            )
            return False

        obj = self.model.objects.get(object_name)
        if obj is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"unknown object in target event: {object_name}",
                object_name=object_name,
                event_name=event_name,
            )
            return False

        current_state = self.states.get(object_name)
        event = self._event_from_current_state(obj, event_name, current_state)
        if event is None:
            return False

        trace_frame = _TraceFrame(
            object_name=object_name,
            event_name=event_name,
            source_state=event.source_state,
            target_state=event.target_state,
            span=event.decl.span,
        )
        self.trace_stack.append(trace_frame)
        self.stack.append(key)
        exit_status = DerivationStatus.BLOCKED
        exit_message: str | None = None
        try:
            self._collect_deferred(event.decl.deferred, event, "event")
            if not self._verify_blocks(event.decl.depends_on, "depends_on", event=event):
                exit_message = "depends_on blocked"
                return False

            bindings: dict[str, dict[str, str]] = {}
            event_result_hints = _block_entries(event.decl.ensures)
            if not self._drive_blocks(
                event.decl.drives,
                event,
                bindings=bindings,
                result_hints=event_result_hints,
            ):
                exit_message = "drives blocked"
                return False

            for within in event.decl.within:
                if not self._execute_within(
                    within,
                    event,
                    bindings=bindings,
                    result_hints=event_result_hints,
                ):
                    exit_message = f"within blocked: {within.context}"
                    return False

            if self.states.get(object_name) != event.source_state:
                self._record(
                    DerivationStatus.CONTRADICTION,
                    "event source state changed during drives: "
                    f"{object_name}.state is State::{self.states.get(object_name)}, "
                    f"expected State::{event.source_state}",
                    event.decl.span,
                    object_name=object_name,
                    event_name=event_name,
                )
                exit_status = DerivationStatus.CONTRADICTION
                exit_message = (
                    f"source state changed to State::{self.states.get(object_name)}"
                )
                return False

            self.states[object_name] = event.target_state
            self.transitions.append(
                EventTransition(
                    object_name=object_name,
                    event_name=event_name,
                    source_state=event.source_state,
                    target_state=event.target_state,
                )
            )
            self._record(
                DerivationStatus.PROVED,
                f"transition: {_event_label(object_name, event_name)} "
                f"State::{event.source_state} -> State::{event.target_state}",
                event.decl.span,
                object_name=object_name,
                event_name=event_name,
                state_name=event.target_state,
            )
            if self._validate_state(object_name, event.target_state, entered_by=event):
                exit_status = DerivationStatus.PROVED
                return True
            exit_message = "target state invariant blocked"
            return False
        finally:
            self.stack.pop()
            self.trace_stack.pop()
            self._finish_trace(trace_frame, exit_status, exit_message)


    def _drive_blocks(
        self,
        blocks: list[Block],
        event: EventDef,
        *,
        action_provider: str = "action_drive",
        bindings: dict[str, dict[str, str]] | None = None,
        process_parent: str | None = None,
        result_hints: tuple[str, ...] = (),
    ) -> bool:
        bindings = bindings if bindings is not None else {}
        for block in blocks:
            for entry, entry_span in block.entry_spans:
                if not self._drive_entry(
                    entry,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    bindings=bindings,
                    process_parent=process_parent,
                    result_hints=result_hints,
                ):
                    return False
        return True

    def _drive_entry(
        self,
        entry: str,
        entry_span: SourceSpan,
        event: EventDef,
        *,
        action_provider: str,
        bindings: dict[str, dict[str, str]],
        process_parent: str | None,
        result_hints: tuple[str, ...],
    ) -> bool:
        bind = _ACTION_BIND_RE.match(entry)
        if bind is not None:
            name, type_name, object_name, action_name, args = bind.group(1, 2, 3, 4, 5)
            obj = self.model.objects.get(object_name)
            display_call = _process_call_expression(
                f"{object_name}.Action::{action_name}", args
            )
            display_expression = f"let {name}: {type_name} <- {display_call}"
            receiver = _binding_or_ref_value(object_name, bindings)
            receiver_type = receiver.get("type") if receiver is not None else None
            ref_process_type = _ref_process_type(
                self.model, receiver_type, "Action", action_name
            )
            if ref_process_type is not None:
                call = _normalize_ref_process_expression(
                    self.model,
                    object_name,
                    receiver_type or "",
                    "Action",
                    action_name,
                    args,
                    display_call,
                )
            else:
                call = _normalize_process_expression(
                    self.model,
                    object_name,
                    "Action",
                    action_name,
                    args,
                    display_call,
                )
            expression = f"let {name}: {type_name} <- {call}"
            bindings[name] = {"type": type_name}
            process_type = ref_process_type or (obj.kind if obj is not None else None)
            result_value = _action_result_value_from_hints(
                self.model,
                object_name,
                action_name,
                type_name,
                args,
                process_type,
                result_hints,
                bindings,
            )
            if result_value is None:
                result_value = _action_result_value(
                    self.model, object_name, action_name, type_name
                )
            if result_value is not None:
                bindings[name]["value"] = result_value
            self._record(
                DerivationStatus.PROVED,
                f"action result bound: {name}: {type_name} <- {object_name}.Action::{action_name}",
                entry_span,
                object_name=event.object_name,
                event_name=event.name,
                expression=expression,
                display_expression=display_expression if expression != display_expression else None,
                source_kind="drives",
                predicate=None,
                proof_class="action_result_binding",
                proof_provider=action_provider,
                process_parent=process_parent,
            )
            parent_expression = display_expression if expression != display_expression else expression
            if ref_process_type is not None:
                target_object = _ref_process_self_name(
                    self.model, receiver, ref_process_type, object_name
                )
                canonical_args = _canonicalize_ref_aliases(args or "", bindings)
                if not self._execute_type_process_drives(
                    ref_process_type,
                    target_object or object_name,
                    "Action",
                    action_name,
                    canonical_args if args is not None else args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    process_parent=parent_expression,
                    bindings=bindings,
                    result_hints=result_hints,
                ):
                    return False
                self._record_type_process_ensures(
                    ref_process_type,
                    target_object or object_name,
                    "Action",
                    action_name,
                    canonical_args if args is not None else args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    bindings=bindings,
                )
            elif obj is not None:
                if not self._execute_type_process_drives(
                    obj.kind,
                    object_name,
                    "Action",
                    action_name,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    process_parent=parent_expression,
                    bindings=bindings,
                    result_hints=result_hints,
                ):
                    return False
                self._record_type_process_ensures(
                    obj.kind,
                    object_name,
                    "Action",
                    action_name,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    bindings=bindings,
                )
            return True

        ref_event = _REF_EVENT_EXPR_RE.match(entry)
        if ref_event is not None:
            receiver_name, driven_event, args = ref_event.group(1, 2, 3)
            receiver = bindings.get(receiver_name)
            receiver_type = receiver.get("type") if receiver is not None else None
            process_type = _ref_process_type(
                self.model, receiver_type, "Event", driven_event
            )
            if process_type is not None:
                target_object = _ref_process_self_name(
                    self.model, receiver, process_type, receiver_name
                )
                expression = _normalize_ref_process_expression(
                    self.model,
                    receiver_name,
                    receiver_type,
                    "Event",
                    driven_event,
                    args,
                    entry,
                )
                display_expression = _process_call_expression(
                    f"{receiver_name}.Event::{driven_event}", args
                )
                self._record(
                    DerivationStatus.PROVED,
                    f"ref type process committed: {receiver_name}.Event::{driven_event}",
                    entry_span,
                    object_name=event.object_name,
                    event_name=event.name,
                    expression=expression,
                    display_expression=display_expression if expression != display_expression else None,
                    source_kind="drives",
                    predicate=None,
                    proof_class="type_process_commit",
                    proof_provider=action_provider,
                    process_parent=process_parent,
                )
                parent_expression = (
                    display_expression if expression != display_expression else expression
                )
                if not self._execute_type_process_drives(
                    process_type,
                    target_object or receiver_name,
                    "Event",
                    driven_event,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    process_parent=parent_expression,
                    bindings=bindings,
                    result_hints=result_hints,
                ):
                    return False
                self._record_type_process_ensures(
                    process_type,
                    target_object or receiver_name,
                    "Event",
                    driven_event,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    bindings=bindings,
                )
                return True

        child_event = _CHILD_EVENT_EXPR_RE.match(entry)
        if child_event is not None:
            parent_name, child_name, driven_event, args = child_event.group(1, 2, 3, 4)
            child_type = _owned_child_type(self.model, parent_name, child_name)
            if child_type is None:
                self._record(
                    DerivationStatus.BLOCKED,
                    "unknown child event receiver: "
                    f"{parent_name}.{child_name}.Event::{driven_event}",
                    entry_span,
                    object_name=event.object_name,
                    event_name=event.name,
                    expression=entry,
                )
                return False

            receiver = f"{parent_name}.{child_name}"
            expression = _normalize_process_call(
                self.model,
                child_type,
                f"{receiver}.Event::{driven_event}",
                "Event",
                driven_event,
                args,
                entry,
            )
            display_expression = _process_call_expression(
                f"{receiver}.Event::{driven_event}", args
            )
            self._record(
                DerivationStatus.PROVED,
                f"child type process committed: {receiver}.Event::{driven_event}",
                entry_span,
                object_name=event.object_name,
                event_name=event.name,
                expression=expression,
                display_expression=display_expression if expression != display_expression else None,
                source_kind="drives",
                predicate=None,
                proof_class="type_process_commit",
                proof_provider=action_provider,
                process_parent=process_parent,
            )
            parent_expression = display_expression if expression != display_expression else expression
            if not self._execute_type_process_drives(
                child_type,
                receiver,
                "Event",
                driven_event,
                args,
                entry_span,
                event,
                action_provider=action_provider,
                process_parent=parent_expression,
                bindings=bindings,
                result_hints=result_hints,
            ):
                return False
            self._record_type_process_ensures(
                child_type,
                receiver,
                "Event",
                driven_event,
                args,
                entry_span,
                event,
                action_provider=action_provider,
                bindings=bindings,
            )
            return True

        match = _EVENT_EXPR_RE.match(entry)
        if match is not None:
            driven_object, driven_event, args = match.group(1, 2, 3)
            obj = self.model.objects.get(driven_object)
            receiver = _binding_or_ref_value(driven_object, bindings)
            receiver_type = receiver.get("type") if receiver is not None else None
            process_type = _ref_process_type(
                self.model, receiver_type, "Event", driven_event
            )
            if obj is None and process_type is not None:
                target_object = _ref_process_self_name(
                    self.model, receiver, process_type, driven_object
                )
                expression = _normalize_ref_process_expression(
                    self.model,
                    driven_object,
                    receiver_type or "",
                    "Event",
                    driven_event,
                    args,
                    entry,
                )
                display_expression = _process_call_expression(
                    f"{driven_object}.Event::{driven_event}", args
                )
                self._record(
                    DerivationStatus.PROVED,
                    f"ref type process committed: {driven_object}.Event::{driven_event}",
                    entry_span,
                    object_name=event.object_name,
                    event_name=event.name,
                    expression=expression,
                    display_expression=display_expression if expression != display_expression else None,
                    source_kind="drives",
                    predicate=None,
                    proof_class="type_process_commit",
                    proof_provider=action_provider,
                    process_parent=process_parent,
                )
                parent_expression = (
                    display_expression if expression != display_expression else expression
                )
                if not self._execute_type_process_drives(
                    process_type,
                    target_object or driven_object,
                    "Event",
                    driven_event,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    process_parent=parent_expression,
                    bindings=bindings,
                    result_hints=result_hints,
                ):
                    return False
                self._record_type_process_ensures(
                    process_type,
                    target_object or driven_object,
                    "Event",
                    driven_event,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    bindings=bindings,
                )
                return True
            if obj is not None and _find_event(obj, driven_event) is None:
                if _type_declares_event(self.model, obj, driven_event):
                    expression = _normalize_process_expression(
                        self.model,
                        driven_object,
                        "Event",
                        driven_event,
                        args,
                        entry,
                    )
                    display_expression = _process_call_expression(
                        f"{driven_object}.Event::{driven_event}", args
                    )
                    self._record(
                        DerivationStatus.PROVED,
                        f"type process committed: {driven_object}.Event::{driven_event}",
                        entry_span,
                        object_name=event.object_name,
                        event_name=event.name,
                        expression=expression,
                        display_expression=display_expression if expression != display_expression else None,
                        source_kind="drives",
                        predicate=None,
                        proof_class="type_process_commit",
                        proof_provider=action_provider,
                        process_parent=process_parent,
                    )
                    parent_expression = (
                        display_expression if expression != display_expression else expression
                    )
                    if not self._execute_type_process_drives(
                        obj.kind,
                        driven_object,
                        "Event",
                        driven_event,
                        args,
                        entry_span,
                        event,
                        action_provider=action_provider,
                        process_parent=parent_expression,
                        bindings=bindings,
                        result_hints=result_hints,
                    ):
                        return False
                    self._record_type_process_ensures(
                        obj.kind,
                        driven_object,
                        "Event",
                        driven_event,
                        args,
                        entry_span,
                        event,
                        action_provider=action_provider,
                        bindings=bindings,
                    )
                    return True
            else:
                if self._derive_event(driven_object, driven_event):
                    return True
            self._record(
                DerivationStatus.BLOCKED,
                "driven event blocked: "
                f"{_event_label(driven_object, driven_event)}",
                entry_span,
                object_name=event.object_name,
                event_name=event.name,
                expression=entry,
            )
            return False

        ref_action = _REF_ACTION_EXPR_RE.match(entry)
        if ref_action is not None:
            receiver_name, action_name, args = ref_action.group(1, 2, 3)
            receiver = _binding_or_ref_value(receiver_name, bindings)
            receiver_type = receiver.get("type") if receiver is not None else None
            process_type = _ref_process_type(
                self.model, receiver_type, "Action", action_name
            )
            if process_type is not None:
                target_object = _ref_process_self_name(
                    self.model, receiver, process_type, receiver_name
                )
                canonical_args = _canonical_process_args(
                    self.model,
                    process_type,
                    "Action",
                    action_name,
                    args,
                    bindings,
                )
                canonical_entry = _process_call_expression(
                    f"{receiver_name}.Action::{action_name}", canonical_args
                )
                expression = _normalize_ref_process_expression(
                    self.model,
                    receiver_name,
                    receiver_type,
                    "Action",
                    action_name,
                    canonical_args,
                    canonical_entry,
                )
                display_expression = _process_call_expression(
                    f"{receiver_name}.Action::{action_name}", args
                )
                self._record(
                    DerivationStatus.PROVED,
                    f"ref action committed: {receiver_name}.Action::{action_name}",
                    entry_span,
                    object_name=event.object_name,
                    event_name=event.name,
                    expression=expression,
                    display_expression=display_expression if expression != display_expression else None,
                    source_kind="drives",
                    predicate=None,
                    proof_class="type_process_commit",
                    proof_provider=action_provider,
                    process_parent=process_parent,
                )
                parent_expression = (
                    display_expression if expression != display_expression else expression
                )
                if not self._execute_type_process_drives(
                    process_type,
                    target_object or receiver_name,
                    "Action",
                    action_name,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    process_parent=parent_expression,
                    bindings=bindings,
                    result_hints=result_hints,
                ):
                    return False
                self._record_type_process_ensures(
                    process_type,
                    target_object or receiver_name,
                    "Action",
                    action_name,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    bindings=bindings,
                )
                return True

        child_action = _CHILD_ACTION_EXPR_RE.match(entry)
        if child_action is not None:
            parent_name, child_name, action_name, args = child_action.group(1, 2, 3, 4)
            child_type = _owned_child_type(self.model, parent_name, child_name)
            if child_type is None:
                self._record(
                    DerivationStatus.BLOCKED,
                    "unknown child action receiver: "
                    f"{parent_name}.{child_name}.Action::{action_name}",
                    entry_span,
                    object_name=event.object_name,
                    event_name=event.name,
                    expression=entry,
                )
                return False

            receiver = f"{parent_name}.{child_name}"
            expression = _normalize_process_call(
                self.model,
                child_type,
                f"{receiver}.Action::{action_name}",
                "Action",
                action_name,
                args,
                entry,
            )
            display_expression = _process_call_expression(
                f"{receiver}.Action::{action_name}", args
            )
            self._record(
                DerivationStatus.PROVED,
                f"child action committed: {receiver}.Action::{action_name}",
                entry_span,
                object_name=event.object_name,
                event_name=event.name,
                expression=expression,
                display_expression=display_expression if expression != display_expression else None,
                source_kind="drives",
                predicate=None,
                proof_class="action_commit",
                proof_provider=action_provider,
                process_parent=process_parent,
            )
            parent_expression = display_expression if expression != display_expression else expression
            if not self._execute_type_process_drives(
                child_type,
                receiver,
                "Action",
                action_name,
                args,
                entry_span,
                event,
                action_provider=action_provider,
                process_parent=parent_expression,
                bindings=bindings,
                result_hints=result_hints,
            ):
                return False
            self._record_type_process_ensures(
                child_type,
                receiver,
                "Action",
                action_name,
                args,
                entry_span,
                event,
                action_provider=action_provider,
                bindings=bindings,
            )
            return True

        action = _ACTION_EXPR_RE.match(entry)
        if action is not None:
            object_name, action_name, args = action.group(1, 2, 3)
            receiver = _binding_or_ref_value(object_name, bindings)
            receiver_type = receiver.get("type") if receiver is not None else None
            process_type = _ref_process_type(
                self.model, receiver_type, "Action", action_name
            )
            if object_name not in self.model.objects and process_type is not None:
                target_object = _ref_process_self_name(
                    self.model, receiver, process_type, object_name
                )
                canonical_args = _canonical_process_args(
                    self.model,
                    process_type,
                    "Action",
                    action_name,
                    args,
                    bindings,
                )
                canonical_entry = _process_call_expression(
                    f"{object_name}.Action::{action_name}", canonical_args
                )
                expression = _normalize_ref_process_expression(
                    self.model,
                    object_name,
                    receiver_type or "",
                    "Action",
                    action_name,
                    canonical_args,
                    canonical_entry,
                )
                display_expression = _process_call_expression(
                    f"{object_name}.Action::{action_name}", args
                )
                self._record(
                    DerivationStatus.PROVED,
                    f"ref action committed: {object_name}.Action::{action_name}",
                    entry_span,
                    object_name=event.object_name,
                    event_name=event.name,
                    expression=expression,
                    display_expression=display_expression if expression != display_expression else None,
                    source_kind="drives",
                    predicate=None,
                    proof_class="type_process_commit",
                    proof_provider=action_provider,
                    process_parent=process_parent,
                )
                parent_expression = (
                    display_expression if expression != display_expression else expression
                )
                if not self._execute_type_process_drives(
                    process_type,
                    target_object or object_name,
                    "Action",
                    action_name,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    process_parent=parent_expression,
                    bindings=bindings,
                    result_hints=result_hints,
                ):
                    return False
                self._record_type_process_ensures(
                    process_type,
                    target_object or object_name,
                    "Action",
                    action_name,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    bindings=bindings,
                )
                return True
            expression = _normalize_process_expression(
                self.model,
                object_name,
                "Action",
                action_name,
                args,
                entry,
            )
            display_expression = _process_call_expression(
                f"{object_name}.Action::{action_name}", args
            )
            self._record(
                DerivationStatus.PROVED,
                f"action committed: {object_name}.Action::{action_name}",
                entry_span,
                object_name=event.object_name,
                event_name=event.name,
                expression=expression,
                display_expression=display_expression if expression != display_expression else None,
                source_kind="drives",
                predicate=None,
                proof_class="action_commit",
                proof_provider=action_provider,
                process_parent=process_parent,
            )
            parent_expression = display_expression if expression != display_expression else expression
            obj = self.model.objects.get(object_name)
            if obj is not None:
                if not self._execute_type_process_drives(
                    obj.kind,
                    object_name,
                    "Action",
                    action_name,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    process_parent=parent_expression,
                    bindings=bindings,
                    result_hints=result_hints,
                ):
                    return False
                self._record_type_process_ensures(
                    obj.kind,
                    object_name,
                    "Action",
                    action_name,
                    args,
                    entry_span,
                    event,
                    action_provider=action_provider,
                    bindings=bindings,
                )
            return True

        self._record(
            DerivationStatus.BLOCKED,
            f"cannot parse drives entry: {entry}",
            entry_span,
            object_name=event.object_name,
            event_name=event.name,
            expression=entry,
        )
        return False

    def _execute_type_process_drives(
        self,
        type_name: str,
        self_name: str,
        process_kind: str,
        process_name: str,
        args: str | None,
        span: SourceSpan,
        event: EventDef,
        *,
        action_provider: str,
        process_parent: str | None = None,
        bindings: dict[str, dict[str, str]] | None = None,
        result_hints: tuple[str, ...] = (),
    ) -> bool:
        key = (type_name, self_name, process_kind, process_name)
        if key in self.process_stack:
            self._record(
                DerivationStatus.BLOCKED,
                "recursive type process drives: "
                f"{type_name}.{process_kind}::{process_name}",
                span,
                object_name=event.object_name,
                event_name=event.name,
            )
            return False

        drives = _process_drives(self.model, type_name, process_kind, process_name)
        withins = _process_withins(self.model, type_name, process_kind, process_name)
        process_ensures = _process_ensures(
            self.model,
            type_name,
            process_kind,
            process_name,
        )
        if not drives and not withins:
            return True

        argument_bindings = _process_argument_bindings(
            self.model,
            type_name,
            process_kind,
            process_name,
            args,
        )
        replacements = {**argument_bindings, "self": self_name}
        inherited_bindings = dict(bindings or {})
        signature = dict(
            _process_signature(self.model, type_name, process_kind, process_name) or ()
        )
        drive_bindings = dict(inherited_bindings)
        for name, value in argument_bindings.items():
            inherited = inherited_bindings.get(value)
            param_type = signature.get(name)
            if inherited is not None:
                drive_bindings[name] = _process_param_binding(
                    self.model,
                    inherited,
                    value,
                    param_type,
                )
                drive_bindings[name]["display"] = value
                continue
            ref_type = (
                param_type
                if param_type in _REF_TARGET_PROCESS_TYPES
                else _known_ref_value_type(value)
            )
            if ref_type is not None:
                drive_bindings[name] = {"type": ref_type, "value": value}
            else:
                drive_bindings[name] = {"type": "ProcessArgument", "value": value}
        drive_bindings["self"] = {"type": "Self", "value": self_name}
        hint_replacements = dict(replacements)
        for name in argument_bindings:
            value = drive_bindings.get(name, {}).get("value")
            if value is not None:
                hint_replacements[name] = value
        local_result_hints = result_hints + tuple(
            _substitute_process_bindings(ensure, hint_replacements)
            for ensure in process_ensures
        )

        self.process_stack.append(key)
        try:
            for block in drives:
                entries = [
                    _substitute_process_bindings(entry, replacements)
                    for entry in block.entries
                ]
                body = ";\n".join(entries)
                if body:
                    body += ";"
                if not self._drive_blocks(
                    (
                        Block(
                            block.kind,
                            body,
                            block.span,
                            header=block.header,
                            body_start_line=block.body_start_line,
                        ),
                    ),
                    event,
                    action_provider=action_provider,
                    bindings=drive_bindings,
                    process_parent=process_parent,
                    result_hints=local_result_hints,
                ):
                    return False
            for within in withins:
                substituted_within = _substitute_within_bindings(within, replacements)
                if not self._execute_within(
                    substituted_within,
                    event,
                    bindings=drive_bindings,
                    process_parent=process_parent,
                    result_hints=local_result_hints,
                ):
                    return False
        finally:
            self.process_stack.pop()
        return True

    def _execute_within(
        self,
        within,
        event: EventDef,
        *,
        bindings: dict[str, dict[str, str]] | None = None,
        process_parent: str | None = None,
        result_hints: tuple[str, ...] = (),
    ) -> bool:
        bindings = dict(bindings or {})
        bindings.update(_within_parameter_bindings(within.parameters, bindings))
        local_result_hints = result_hints + _block_entries(within.ensures)
        context = self.model.exclusive_contexts.get(within.context)
        if context is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"unknown exclusive_context: {within.context}",
                within.span,
                object_name=event.object_name,
                event_name=event.name,
            )
            return False

        self._record(
            DerivationStatus.PROVED,
            f"within entered: {within.context}",
            within.span,
            object_name=event.object_name,
            event_name=event.name,
            expression=f"within {within.context}",
            source_kind="within",
            proof_class="exclusive_context",
            proof_provider="guard",
            process_parent=process_parent,
        )
        entered_by = (
            context.guard.entered_by
            if context.guard is not None
            else within.entered_by
        )
        exited_by = (
            context.guard.exited_by
            if context.guard is not None
            else within.exited_by
        )

        if not self._commit_within_boundary(
            entered_by,
            event,
            source_kind="within_entered_by",
        ):
            return False
        self._collect_deferred(within.deferred, event, "within")
        if not self._verify_blocks(
            within.depends_on,
            "within depends_on",
            event=event,
            bindings=bindings,
        ):
            return False
        if not self._drive_blocks(
            within.drives,
            event,
            action_provider="within_context",
            bindings=bindings,
            process_parent=process_parent,
            result_hints=local_result_hints,
        ):
            return False
        for child_within in within.within:
            if not self._execute_within(
                child_within,
                event,
                bindings=bindings,
                process_parent=process_parent,
                result_hints=local_result_hints,
            ):
                return False
        if not self._prove_blocks(
            within.ensures,
            "within ensures",
            event=event,
            proof_class="exclusive_context_fact",
            proof_provider="within_ensures",
            bindings=bindings,
        ):
            return False
        if not self._commit_within_boundary(
            exited_by,
            event,
            source_kind="within_exited_by",
        ):
            return False
        self._record(
            DerivationStatus.PROVED,
            f"within exited: {within.context}",
            within.span,
            object_name=event.object_name,
            event_name=event.name,
            expression=f"within {within.context} exited",
            source_kind="within",
            proof_class="exclusive_context",
            proof_provider="guard",
            process_parent=process_parent,
        )
        return True

    def _commit_within_boundary(
        self, blocks: list[Block], event: EventDef, *, source_kind: str
    ) -> bool:
        for block in blocks:
            for entry, entry_span in block.entry_spans:
                match = _EVENT_EXPR_RE.match(entry)
                if match is None:
                    self._record(
                        DerivationStatus.BLOCKED,
                        f"cannot parse within boundary entry: {entry}",
                        entry_span,
                        object_name=event.object_name,
                        event_name=event.name,
                        expression=entry,
                    )
                    return False
                object_name, event_name = match.group(1), match.group(2)
                self._record(
                    DerivationStatus.PROVED,
                    f"within boundary committed: {object_name}.Event::{event_name}",
                    entry_span,
                    object_name=event.object_name,
                    event_name=event.name,
                    expression=entry,
                    source_kind=source_kind,
                    predicate=None,
                    proof_class="context_guard_event",
                    proof_provider="guard",
                )
        return True

    def _prove_blocks(
        self,
        blocks: list[Block],
        kind: str,
        *,
        event: EventDef,
        proof_class: str,
        proof_provider: str,
        bindings: dict[str, dict[str, str]] | None = None,
    ) -> bool:
        bindings = bindings or {}
        for block in blocks:
            for entry, entry_span in block.entry_spans:
                canonical_entry = _canonicalize_ref_aliases(entry, bindings)
                classification = _classify_obligation(
                    canonical_entry, kind, event.object_name
                )
                self._record(
                    DerivationStatus.PROVED,
                    f"{kind}: {entry}",
                    entry_span,
                    object_name=event.object_name,
                    event_name=event.name,
                    expression=entry,
                    source_kind=kind,
                    predicate=classification["predicate"],
                    proof_class=proof_class,
                    proof_provider=proof_provider,
                )
        return True

    def _record_type_process_ensures(
        self,
        type_name: str,
        self_name: str,
        process_kind: str,
        process_name: str,
        args: str | None,
        span: SourceSpan,
        event: EventDef,
        *,
        action_provider: str,
        bindings: dict[str, dict[str, str]] | None = None,
    ) -> None:
        argument_bindings = _process_argument_bindings(
            self.model,
            type_name,
            process_kind,
            process_name,
            args,
        )
        inherited_bindings = dict(bindings or {})
        replacements: dict[str, str] = {"self": self_name}
        signature = dict(
            _process_signature(self.model, type_name, process_kind, process_name) or ()
        )
        for name, value in argument_bindings.items():
            inherited = inherited_bindings.get(value)
            if inherited is not None:
                binding = _process_param_binding(
                    self.model,
                    inherited,
                    value,
                    signature.get(name),
                )
                replacements[name] = binding.get("value", value)
            else:
                replacements[name] = value
        for ensure in _process_ensures(
            self.model,
            type_name,
            process_kind,
            process_name,
        ):
            expression = _substitute_process_bindings(ensure, replacements)
            classification = _classify_obligation(
                expression, "type process ensures", event.object_name
            )
            self._record(
                DerivationStatus.PROVED,
                f"type process ensures: {expression}",
                span,
                object_name=event.object_name,
                event_name=event.name,
                expression=expression,
                source_kind="type_process_ensures",
                predicate=classification["predicate"],
                proof_class="type_process_ensures",
                proof_provider=action_provider,
            )


    def _event_from_current_state(
        self, obj: ObjectDef, event_name: str, current_state: str | None
    ) -> EventDef | None:
        if current_state is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"object has no current state: {obj.name}",
                obj.decl.span,
                object_name=obj.name,
                event_name=event_name,
            )
            return None

        state = obj.states.get(current_state)
        if state is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"unknown current state: {obj.name}.State::{current_state}",
                obj.decl.span,
                object_name=obj.name,
                event_name=event_name,
                state_name=current_state,
            )
            return None

        event = state.events.get(event_name)
        if event is not None:
            return event

        other = _find_event(obj, event_name)
        if other is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"unknown event: {_event_label(obj.name, event_name)}",
                obj.decl.span,
                object_name=obj.name,
                event_name=event_name,
            )
        else:
            self._record(
                DerivationStatus.BLOCKED,
                f"event not enabled from State::{current_state}: "
                f"{_event_label(obj.name, event_name)} requires State::{other.source_state}",
                other.decl.span,
                object_name=obj.name,
                event_name=event_name,
                state_name=current_state,
            )
        return None

    def _validate_state(
        self, object_name: str, state_name: str, entered_by: EventDef | None = None
    ) -> bool:
        key = (object_name, state_name)
        if entered_by is not None:
            self.state_validation_events[key] = entered_by
        if key in self.validated_states:
            return True
        self.validated_states.add(key)

        obj = self.model.objects.get(object_name)
        if obj is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"unknown object while validating state: {object_name}",
                object_name=object_name,
                state_name=state_name,
            )
            return False

        state = obj.states.get(state_name)
        if state is None:
            self._record(
                DerivationStatus.CONTRADICTION,
                f"unknown state while validating: {object_name}.State::{state_name}",
                obj.decl.span,
                object_name=object_name,
                state_name=state_name,
            )
            return False

        self._collect_deferred(state.decl.deferred, state, "state")
        return self._verify_blocks(
            state.decl.invariants,
            "invariant",
            state=state,
            entered_by=self.state_validation_events.get(key),
        )

    def _verify_blocks(
        self,
        blocks: list[Block],
        kind: str,
        *,
        event: EventDef | None = None,
        state: StateDef | None = None,
        entered_by: EventDef | None = None,
        bindings: dict[str, dict[str, str]] | None = None,
    ) -> bool:
        bindings = bindings or {}
        ok = True
        for block in blocks:
            for entry, entry_span in block.entry_spans:
                canonical_entry = _canonicalize_ref_aliases(entry, bindings)
                if _STATE_EXPR_RE.match(entry):
                    ok = (
                        self._verify_state_expression(
                            entry, entry_span, kind, event, state
                        )
                        and ok
                    )
                elif self._try_prove_event_ensures(
                    entry, entry_span, kind, state, entered_by
                ):
                    continue
                elif self._try_prove_boot_protocol_fact(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif self._try_prove_external_source_fact(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif self._try_prove_kernel_image_linker_fact(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif self._try_prove_platform_cpu_fact(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif self._try_prove_linear_map_layout_fact(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif self._try_prove_trampoline_map_layout_fact(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif self._try_prove_kernel_image_map_layout_fact(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif self._try_prove_fixmap_slot_layout_fact(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif self._try_prove_raw_dtb_memory_fact(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif self._try_prove_stack_layout_fact(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif self._try_prove_prior_fact(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif canonical_entry != entry and self._try_prove_prior_fact(
                    canonical_entry,
                    entry_span,
                    kind,
                    event,
                    state,
                    recorded_expression=entry,
                ):
                    continue
                elif self._try_prove_phase_context(
                    entry, entry_span, kind, event, state
                ):
                    continue
                elif self._try_prove_builtin_predicate(
                    entry, entry_span, kind, event, state
                ):
                    continue
                else:
                    context_object = _context_object(event, state)
                    classification = _classify_obligation(entry, kind, context_object)
                    self._record(
                        DerivationStatus.OBLIGATION,
                        f"unresolved {kind}: {entry}",
                        entry_span,
                        object_name=context_object,
                        event_name=event.name if event is not None else None,
                        state_name=state.name if state is not None else None,
                        expression=entry,
                        source_kind=kind,
                        predicate=classification["predicate"],
                        obligation_category=classification["category"],
                        proof_class=classification["proof_class"],
                        proof_provider=classification["proof_provider"],
                    )
        return ok

    def _verify_state_expression(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        match = _STATE_EXPR_RE.match(expression)
        if match is None:
            return False

        object_name, expected_state = match.group(1), match.group(2)
        actual_state = self.states.get(object_name)
        if actual_state == expected_state:
            self._record(
                DerivationStatus.PROVED,
                f"{kind}: {expression}",
                span,
                object_name=_context_object(event, state),
                event_name=event.name if event is not None else None,
                state_name=state.name if state is not None else None,
                expression=expression,
            )
            return True

        self._record(
            DerivationStatus.BLOCKED,
            f"{kind} requires {expression}, got State::{actual_state}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name if state is not None else None,
            expression=expression,
        )
        return False

    def _try_prove_builtin_predicate(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        stripped = expression.strip()
        if stripped == "readonly(self)" and state is not None:
            obj = self.model.objects.get(state.object_name)
            if obj is not None and obj.decl.properties.get("access") == "Access::ReadOnly":
                self._record_builtin_proof(
                    expression,
                    span,
                    kind,
                    event,
                    state,
                    proof_class="object_attribute",
                    proof_provider="builtin",
                )
                return True

        has_slot = _HAS_SLOT_RE.match(stripped)
        if has_slot is not None:
            slot_name = has_slot.group(1)
            if self._fixmap_config_has_slot(slot_name):
                self._record_builtin_proof(
                    expression,
                    span,
                    kind,
                    event,
                    state,
                    proof_class="config_structure",
                    proof_provider="builtin",
                )
                return True

        no_service = _NO_SERVICE_RE.match(stripped)
        if no_service is not None:
            object_name = no_service.group(1)
            if self.states.get(object_name) == "Destroyed":
                self._record_builtin_proof(
                    expression,
                    span,
                    kind,
                    event,
                    state,
                    proof_class="state_alias",
                    proof_provider="builtin",
                )
                return True

        return False

    def _try_prove_event_ensures(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        state: StateDef | None,
        entered_by: EventDef | None,
    ) -> bool:
        if kind != "invariant" or state is None or entered_by is None:
            return False

        ensure_blocks = list(entered_by.decl.ensures)
        for within in entered_by.decl.within:
            ensure_blocks.extend(within.ensures)
        for block in ensure_blocks:
            expression_key = _fact_key(expression)
            if expression_key not in {_fact_key(entry) for entry in block.entries}:
                continue
            classification = _classify_obligation(
                expression, kind, state.object_name
            )
            self._record(
                DerivationStatus.PROVED,
                f"{kind}: {expression}",
                span,
                object_name=state.object_name,
                event_name=entered_by.name,
                state_name=state.name,
                expression=expression,
                source_kind=kind,
                predicate=classification["predicate"],
                proof_class=classification["proof_class"],
                proof_provider="event_ensures",
            )
            return True
        return False

    def _try_prove_boot_protocol_fact(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        if (
            kind != "invariant"
            or state is None
            or state.object_name != "BootArgs"
            or state.name != "Online"
        ):
            return False

        proof = _BOOT_PROTOCOL_PROOFS.get(expression.strip())
        if proof is None:
            return False
        proof_class, proof_provider = proof
        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class=proof_class,
            proof_provider=proof_provider,
        )
        return True

    def _try_prove_external_source_fact(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        if kind != "invariant" or state is None:
            return False

        obj = self.model.objects.get(state.object_name)
        if obj is None:
            return False
        proof = _EXTERNAL_SOURCE_PROOFS.get(
            (state.object_name, obj.decl.properties.get("source"), expression.strip())
        )
        if proof is None:
            return False
        proof_class, proof_provider = proof
        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class=proof_class,
            proof_provider=proof_provider,
        )
        return True

    def _try_prove_kernel_image_linker_fact(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        if kind != "invariant" or state is None:
            return False
        if state.object_name != "KernelImage":
            return False

        proof = _KERNEL_IMAGE_LINKER_PROOFS.get(expression.strip())
        if proof is None:
            return False

        self._validate_state("Lds", "Online")
        proof_class, proof_provider = proof
        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class=proof_class,
            proof_provider=proof_provider,
        )
        return True

    def _try_prove_platform_cpu_fact(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        if expression.strip() != "platform_hart_id_valid(BootArgs.boot_hartid)":
            return False

        self._validate_state("PlatformCpuInfo", "Online")
        if expression not in self.proved_expressions:
            return False
        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name if state is not None else None,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class="platform_cpu_description",
            proof_provider="prior_derivation_facts",
        )
        return True

    def _try_prove_linear_map_layout_fact(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        if kind != "invariant" or state is None:
            return False
        if state.object_name != "LinearMap" or state.name != "Destroyed":
            return False

        proof = _LINEAR_MAP_LAYOUT_PROOFS.get(expression.strip())
        if proof is None:
            return False
        if not self._validate_state("Config", "Online"):
            return False

        proof_class, proof_provider = proof
        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class=proof_class,
            proof_provider=proof_provider,
        )
        return True

    def _try_prove_stack_layout_fact(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        if kind != "invariant" or state is None:
            return False
        if state.object_name != "BootInitStack":
            return False
        if (
            expression.strip()
            != "Lds.init_stack_end - Lds.init_stack_start >= Config.page_size"
        ):
            return False

        if not (
            self._validate_state("Lds", "Online")
            and self._validate_state("Config", "Online")
            and self._validate_state("KernelImage", "Ready")
        ):
            return False
        required = {
            "boot_stack_size >= page_size",
            "boot_stack_size == Config.boot_stack_size",
            "init_stack_end - init_stack_start == boot_stack_size",
        }
        if not required.issubset(self.proved_expressions):
            return False

        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class="stack_layout",
            proof_provider="config_and_linker",
        )
        return True

    def _try_prove_trampoline_map_layout_fact(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        if expression.strip() != "valid_trampoline_map(TrampolineMap)":
            return False
        if not (
            self._validate_state("Lds", "Online")
            and self._validate_state("Config", "Online")
        ):
            return False

        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name if state is not None else None,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class="address_mapping",
            proof_provider="config_and_linker",
        )
        return True

    def _try_prove_kernel_image_map_layout_fact(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        if expression.strip() != "fits_in_kernel_image_map(KernelImage, KernelImageMap)":
            return False
        if not (
            self._validate_state("KernelImage", "Ready")
            and self._validate_state("Lds", "Online")
            and self._validate_state("Config", "Online")
        ):
            return False

        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name if state is not None else None,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class="address_mapping",
            proof_provider="config_and_linker",
        )
        return True

    def _try_prove_fixmap_slot_layout_fact(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        if (
            expression.strip()
            != "fits_in_fixmap_slot(RawDtb.range, fdt_slot, Config.page_size)"
        ):
            return False
        if not (
            self._validate_state("RawDtb", "Ready")
            and self._validate_state("Config", "Online")
        ):
            return False

        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name if state is not None else None,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class="address_mapping",
            proof_provider="riscv_fixmap_layout",
        )
        return True

    def _try_prove_raw_dtb_memory_fact(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        stripped = expression.strip()
        proof = _CONTAINS_PROOFS.get(stripped)
        if proof is None:
            return False

        if not (
            self._validate_state("OpenSbiFirmware", "Online")
            and self._validate_state("BootArgs", "Online")
            and self._validate_state("PhysicalMemory", "Online")
        ):
            return False

        if (
            "firmware_dtb_blob_in_ram_at_kernel_entry(BootArgs.dtb_pa)"
            not in self.proved_expressions
        ):
            return False

        if stripped == "contains(PhysicalMemory.ram, header_range)":
            required = {
                "header_range.start == BootArgs.dtb_pa",
                "header_range.end == BootArgs.dtb_pa + size_of::<DtbHeader>()",
            }
        elif stripped == "contains(PhysicalMemory.ram, range)":
            required = {
                "valid_dtb_header(header)",
                "range.start == BootArgs.dtb_pa",
                "range.end == BootArgs.dtb_pa + header.total_size",
            }
        else:
            return False

        if not required.issubset(self.proved_expressions):
            return False

        proof_class, proof_provider = proof
        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name if state is not None else None,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class=proof_class,
            proof_provider=proof_provider,
        )
        return True

    def _try_prove_prior_fact(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
        *,
        recorded_expression: str | None = None,
    ) -> bool:
        proof_class = _prior_fact_proof_class(expression, self.proved_expressions)
        if proof_class is None:
            return False
        recorded_expression = recorded_expression or expression
        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {recorded_expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name if state is not None else None,
            expression=recorded_expression,
            source_kind=kind,
            predicate=_predicate_name(recorded_expression),
            proof_class=proof_class,
            proof_provider="prior_derivation_facts",
        )
        return True

    def _try_prove_phase_context(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
    ) -> bool:
        if (
            kind != "invariant"
            or state is None
            or state.object_name not in {"EntryPreludePhase", "EntrySuccessorPhase"}
        ):
            return False

        if expression == "interrupt_concurrency_closed()":
            self._validate_state("OpenSbiFirmware", "Online")
            if "primary_hart_sie_clear_at_kernel_entry()" not in self.proved_expressions:
                return False
            proof_class = "system_exclusive_context"
            proof_provider = "prior_derivation_facts"
        elif expression == "task_concurrency_closed()":
            self._validate_state("SbiSpec", "Online")
            self._validate_state("OpenSbiFirmware", "Online")
            if not {
                "sbi_hsm_available()",
                "ordered_booting_enabled()",
                "primary_hart_only_at_kernel_entry()",
            }.issubset(self.proved_expressions):
                return False
            proof_class = "system_exclusive_context"
            proof_provider = "prior_derivation_facts"
        elif expression == "context_is(SystemExclusive)" and {
            "interrupt_concurrency_closed()",
            "task_concurrency_closed()",
        }.issubset(self.proved_expressions):
            proof_class = "phase_context"
            proof_provider = "prior_derivation_facts"
        else:
            return False

        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class=proof_class,
            proof_provider=proof_provider,
        )
        return True

    def _record_builtin_proof(
        self,
        expression: str,
        span: SourceSpan,
        kind: str,
        event: EventDef | None,
        state: StateDef | None,
        *,
        proof_class: str,
        proof_provider: str,
    ) -> None:
        self._record(
            DerivationStatus.PROVED,
            f"{kind}: {expression}",
            span,
            object_name=_context_object(event, state),
            event_name=event.name if event is not None else None,
            state_name=state.name if state is not None else None,
            expression=expression,
            source_kind=kind,
            predicate=_predicate_name(expression),
            proof_class=proof_class,
            proof_provider=proof_provider,
        )

    def _fixmap_config_has_slot(self, slot_name: str) -> bool:
        fixmap = self.model.types.get("FixMapConfig")
        if fixmap is None:
            return False
        expected = _slot_field_name(slot_name)
        for block in fixmap.blocks:
            if block.kind != "slots":
                continue
            for entry in block.entries:
                match = _SLOT_ENTRY_RE.match(entry)
                if match is not None and match.group(1) == expected:
                    return True
        return False

    def _collect_deferred(
        self,
        blocks: list[Block],
        owner: EventDef | StateDef,
        kind: str,
    ) -> None:
        for block in blocks:
            entries = block.entry_spans or [
                (block.body.strip(), SourceSpan(block.body_start_line or block.span.start_line, block.span.end_line))
            ]
            for entry, entry_span in entries:
                if not entry:
                    continue
                self._record(
                    DerivationStatus.DEFERRED,
                    f"{kind} deferred: {_strip_quotes(entry)}",
                    entry_span,
                    object_name=owner.object_name,
                    event_name=owner.name if isinstance(owner, EventDef) else None,
                    state_name=owner.name if isinstance(owner, StateDef) else None,
                    expression=entry,
                )

    def _record(
        self,
        status: DerivationStatus,
        message: str,
        span: SourceSpan | None = None,
        *,
        object_name: str | None = None,
        event_name: str | None = None,
        state_name: str | None = None,
        expression: str | None = None,
        source_kind: str | None = None,
        predicate: str | None = None,
        obligation_category: str | None = None,
        proof_class: str | None = None,
        proof_provider: str | None = None,
        display_expression: str | None = None,
        process_parent: str | None = None,
    ) -> None:
        if status is DerivationStatus.PROVED and expression is not None:
            self.proved_expressions.add(expression)
        self.records.append(
            DerivationRecord(
                status=status,
                message=message,
                span=span,
                object_name=object_name,
                event_name=event_name,
                state_name=state_name,
                expression=expression,
                display_expression=display_expression,
                source_kind=source_kind,
                predicate=predicate,
                obligation_category=obligation_category,
                proof_class=proof_class,
                proof_provider=proof_provider,
                process_parent=process_parent,
            )
        )
        if status is DerivationStatus.PROVED and expression is not None:
            alias = _derived_alias(expression)
            if alias is not None and alias not in self.proved_expressions:
                self.proved_expressions.add(alias)
                self.records.append(
                    DerivationRecord(
                        status=DerivationStatus.PROVED,
                        message=f"derived alias: {alias}",
                        span=span,
                        object_name=object_name,
                        event_name=event_name,
                        state_name=state_name,
                        expression=alias,
                        source_kind="derived_alias",
                        predicate=_predicate_name(alias),
                        proof_class="derived_alias",
                        proof_provider="type_process_ensures",
                    )
                )

    def _finish_trace(
        self,
        frame: "_TraceFrame",
        status: DerivationStatus,
        message: str | None,
    ) -> None:
        node = DerivationTraceNode(
            object_name=frame.object_name,
            event_name=frame.event_name,
            source_state=frame.source_state,
            target_state=frame.target_state,
            status=status,
            message=message,
            span=frame.span,
            children=tuple(frame.children),
        )
        if self.trace_stack:
            self.trace_stack[-1].children.append(node)
        else:
            self.trace.append(node)


class _TraceFrame:
    def __init__(
        self,
        *,
        object_name: str,
        event_name: str,
        source_state: str,
        target_state: str,
        span: SourceSpan,
    ) -> None:
        self.object_name = object_name
        self.event_name = event_name
        self.source_state = source_state
        self.target_state = target_state
        self.span = span
        self.children: list[DerivationTraceNode] = []


def _find_event(obj: ObjectDef, event_name: str) -> EventDef | None:
    for state in obj.states.values():
        event = state.events.get(event_name)
        if event is not None:
            return event
    return None


def _type_declares_event(model: ObjectModel, obj: ObjectDef, event_name: str) -> bool:
    type_decl = model.types.get(obj.kind)
    if type_decl is None:
        return False
    pattern = re.compile(_TYPE_EVENT_RE_TEMPLATE.format(re.escape(event_name)))
    return any(pattern.search(block.body) for block in type_decl.blocks)


def _ref_process_type(
    model: ObjectModel,
    receiver_type: str | None,
    process_kind: str,
    process_name: str,
) -> str | None:
    if receiver_type is None:
        return None

    self_type = _ref_self_process_type(model, receiver_type, process_kind, process_name)
    if self_type is not None:
        return self_type

    target_type = _REF_TARGET_PROCESS_TYPES.get(receiver_type)
    if target_type is None:
        return None
    if _process_signature(model, target_type, process_kind, process_name) is None:
        return None
    return target_type


def _ref_self_process_type(
    model: ObjectModel,
    receiver_type: str,
    process_kind: str,
    process_name: str,
) -> str | None:
    if receiver_type not in _REF_SELF_ACTION_TYPES or process_kind != "Action":
        return None
    if _process_signature(model, receiver_type, process_kind, process_name) is None:
        return None
    return receiver_type


def _within_parameter_bindings(
    parameters: dict[str, str], inherited_bindings: dict[str, dict[str, str]]
) -> dict[str, dict[str, str]]:
    bindings: dict[str, dict[str, str]] = {}
    for name, value in parameters.items():
        if value in inherited_bindings:
            bindings[name] = dict(inherited_bindings[value])
            bindings[name]["display"] = value
        else:
            value_type = _known_ref_value_type(value)
            if value_type is not None:
                bindings[name] = {"type": value_type, "value": value}
    return bindings


def _process_param_binding(
    model: ObjectModel,
    inherited: dict[str, str],
    display: str,
    param_type: str | None,
) -> dict[str, str]:
    binding = dict(inherited)
    if param_type is None:
        return binding
    target_type = _REF_TARGET_PROCESS_TYPES.get(inherited.get("type", ""))
    if target_type != param_type:
        return binding
    target_object = _ref_value_target_object(model, inherited.get("value"))
    if target_object is not None:
        binding = {"type": param_type, "value": target_object, "display": display}
    return binding


def _canonicalize_ref_aliases(
    expression: str, bindings: dict[str, dict[str, str]]
) -> str:
    canonical = expression
    for name, binding in bindings.items():
        value = binding.get("value")
        if value:
            canonical = re.sub(rf"\b{re.escape(name)}\b", value, canonical)
    return canonical


def _display_ref_aliases(
    expression: str, bindings: dict[str, dict[str, str]]
) -> str | None:
    display = expression
    changed = False
    for name, binding in bindings.items():
        display_name = binding.get("display")
        if not display_name:
            continue
        replaced = re.sub(rf"\b{re.escape(name)}\b", display_name, display)
        if replaced != display:
            changed = True
            display = replaced
    return display if changed else None


def _ref_target_object(
    model: ObjectModel, binding: dict[str, str] | None
) -> str | None:
    if binding is None:
        return None
    value = binding.get("value")
    return _ref_value_target_object(model, value) if value else None


def _ref_process_self_name(
    model: ObjectModel,
    binding: dict[str, str] | None,
    process_type: str,
    fallback: str,
) -> str:
    binding_type = binding.get("type") if binding is not None else None
    if binding_type == process_type:
        return binding.get("value") or fallback
    return _ref_target_object(model, binding) or fallback


def _ref_value_target_object(model: ObjectModel, ref_value: str | None) -> str | None:
    if not ref_value:
        return None
    if ref_value == "CurrentTaskRef":
        return "BootIdleTask"
    if ref_value == "CurrentRunQueueRef":
        return "BootRunQueue"
    if ref_value == "BootRunQueueRef":
        return "BootRunQueue"
    if ref_value == "BootIdleTaskRef":
        return "BootIdleTask"
    if ref_value == "KernelInitTaskRef":
        return "KernelInitTask"
    if ref_value == "KthreaddTaskRef":
        return "KthreaddTask"
    for type_name in ("SchedulerObject", "RunQueue", "Task"):
        type_decl = model.types.get(type_name)
        if type_decl is None:
            continue
        pattern = re.compile(
            r"\b(?:runqueue_ref_targets|task_ref_targets)\(\s*"
            + re.escape(ref_value)
            + r"\s*,\s*([A-Z][A-Za-z0-9_]*)\s*\)"
        )
        for block in type_decl.blocks:
            match = pattern.search(block.body)
            if match is not None:
                return match.group(1)
    return None


def _normalize_ref_process_expression(
    model: ObjectModel,
    receiver_name: str,
    receiver_type: str,
    process_kind: str,
    process_name: str,
    args: str | None,
    fallback: str,
) -> str:
    process_type = _ref_process_type(model, receiver_type, process_kind, process_name)
    if process_type is None:
        return fallback
    return _normalize_process_call(
        model,
        process_type,
        f"{receiver_name}.{process_kind}::{process_name}",
        process_kind,
        process_name,
        args,
        fallback,
    )


def _binding_or_ref_value(
    receiver_name: str, bindings: dict[str, dict[str, str]]
) -> dict[str, str] | None:
    receiver = bindings.get(receiver_name)
    if receiver is not None:
        return receiver
    receiver_type = _known_ref_value_type(receiver_name)
    if receiver_type is None:
        return None
    return {"type": receiver_type, "value": receiver_name}


def _known_ref_value_type(value: str) -> str | None:
    if value.endswith("RunQueueRef"):
        return "RunQueueRef"
    if value.endswith("TaskRef"):
        return "TaskRef"
    return None


def _normalize_process_expression(
    model: ObjectModel,
    object_name: str,
    process_kind: str,
    process_name: str,
    args: str | None,
    fallback: str,
) -> str:
    obj = model.objects.get(object_name)
    if obj is None:
        return fallback
    return _normalize_process_call(
        model,
        obj.kind,
        f"{object_name}.{process_kind}::{process_name}",
        process_kind,
        process_name,
        args,
        fallback,
    )


def _normalize_process_call(
    model: ObjectModel,
    type_name: str,
    callee: str,
    process_kind: str,
    process_name: str,
    args: str | None,
    fallback: str,
) -> str:
    signature = _process_signature(model, type_name, process_kind, process_name)
    if signature is None or args is None:
        return fallback
    args = args.strip()
    if not args or _uses_named_args(args) or len(signature) != 1 or "," in args:
        return fallback
    param_name, _param_type = signature[0]
    return f"{callee}({param_name}: {args})"


def _process_signature(
    model: ObjectModel, type_name: str, process_kind: str, process_name: str
) -> tuple[tuple[str, str], ...] | None:
    type_decl = model.types.get(type_name)
    if type_decl is None:
        return None
    pattern = re.compile(
        r"\b"
        + re.escape(process_kind)
        + r"::"
        + re.escape(process_name)
        + r"\s*(?:\(([^{};]*)\))?(?:\s*->\s*[A-Z][A-Za-z0-9_]*)?\s*\{",
        re.S,
    )
    for block in type_decl.blocks:
        match = pattern.search(block.body)
        if match is not None:
            return _parse_process_parameters(match.group(1) or "")
    return None


def _process_body(
    model: ObjectModel, type_name: str, process_kind: str, process_name: str
) -> str | None:
    type_decl = model.types.get(type_name)
    if type_decl is None:
        return None
    pattern = re.compile(
        r"\b"
        + re.escape(process_kind)
        + r"::"
        + re.escape(process_name)
        + r"\s*(?:\([^{};]*\))?(?:\s*->\s*[A-Z][A-Za-z0-9_]*)?\s*\{",
        re.S,
    )
    for block in type_decl.blocks:
        match = pattern.search(block.body)
        if match is None:
            continue
        body_start = match.end()
        body_end = _matching_brace_index(block.body, body_start - 1)
        if body_end is not None:
            return block.body[body_start:body_end]
    return None


def _process_ensures(
    model: ObjectModel, type_name: str, process_kind: str, process_name: str
) -> tuple[str, ...]:
    body = _process_body(model, type_name, process_kind, process_name)
    if body is None:
        return ()
    ensures = _top_level_named_block_body(body, "ensures")
    if ensures is None:
        return ()
    return tuple(entry for entry, _span in Block("ensures", ensures, SourceSpan(1, 1)).entry_spans)


def _process_drives(
    model: ObjectModel, type_name: str, process_kind: str, process_name: str
) -> tuple[Block, ...]:
    body = _process_body(model, type_name, process_kind, process_name)
    if body is None:
        return ()
    drives = _top_level_named_block_body(body, "drives")
    if drives is None:
        return ()
    return (Block("drives", drives, SourceSpan(1, 1)),)


def _process_withins(
    model: ObjectModel, type_name: str, process_kind: str, process_name: str
) -> tuple[WithinDecl, ...]:
    body = _process_body(model, type_name, process_kind, process_name)
    if body is None:
        return ()
    return tuple(_top_level_withins(body))


def _top_level_withins(body: str) -> list[WithinDecl]:
    withins: list[WithinDecl] = []
    index = 0
    pattern = re.compile(r"\bwithin\s+([A-Za-z_][A-Za-z0-9_]*(?:\s*\([^{}]*\))?)\s*\{", re.S)
    while index < len(body):
        match = pattern.search(body, index)
        if match is None:
            break
        if not _is_top_level_at(body, match.start()):
            index = match.end()
            continue
        block_start = match.end()
        block_end = _matching_brace_index(body, block_start - 1)
        if block_end is None:
            break
        header = match.group(1).strip()
        context, parameters = _parse_within_header(header)
        body_start_line = 1 + body.count("\n", 0, block_start)
        span = SourceSpan(
            1 + body.count("\n", 0, match.start()),
            1 + body.count("\n", 0, block_end),
        )
        withins.append(
            _within_from_body(
                context,
                body[block_start:block_end],
                span,
                parameters=parameters,
                body_start_line=body_start_line,
            )
        )
        index = block_end + 1
    return withins


def _within_from_body(
    context: str,
    body: str,
    span: SourceSpan,
    *,
    parameters: dict[str, str],
    body_start_line: int,
) -> WithinDecl:
    entered_by: list[Block] = []
    depends_on: list[Block] = []
    drives: list[Block] = []
    nested_withins: list[WithinDecl] = []
    exited_by: list[Block] = []
    may_change: list[Block] = []
    ensures: list[Block] = []
    deferred: list[Block] = []
    other_blocks: list[Block] = []

    for kind, header, child_body, child_span, child_body_start_line in _top_level_blocks(
        body, body_start_line
    ):
        block = Block(
            kind,
            child_body,
            child_span,
            header=header,
            body_start_line=child_body_start_line,
        )
        if kind == "entered_by":
            entered_by.append(block)
        elif kind == "depends_on":
            depends_on.append(block)
        elif kind == "drives":
            drives.append(block)
        elif kind == "within":
            child_context, child_parameters = _parse_within_header(header)
            nested_withins.append(
                _within_from_body(
                    child_context,
                    child_body,
                    child_span,
                    parameters=child_parameters,
                    body_start_line=child_body_start_line,
                )
            )
        elif kind == "exited_by":
            exited_by.append(block)
        elif kind == "may_change":
            may_change.append(block)
        elif kind == "ensures":
            ensures.append(block)
        elif kind == "deferred":
            deferred.append(block)
        else:
            other_blocks.append(block)

    return WithinDecl(
        context=context,
        span=span,
        parameters=parameters,
        entered_by=entered_by,
        depends_on=depends_on,
        drives=drives,
        within=nested_withins,
        exited_by=exited_by,
        may_change=may_change,
        ensures=ensures,
        deferred=deferred,
        other_blocks=other_blocks,
    )


def _top_level_blocks(
    body: str, body_start_line: int
) -> list[tuple[str, str, str, SourceSpan, int]]:
    blocks: list[tuple[str, str, str, SourceSpan, int]] = []
    pattern = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*([^{};]*)\{", re.S)
    index = 0
    while index < len(body):
        match = pattern.search(body, index)
        if match is None:
            break
        if not _is_top_level_at(body, match.start()):
            index = match.end()
            continue
        block_start = match.end()
        block_end = _matching_brace_index(body, block_start - 1)
        if block_end is None:
            break
        kind = match.group(1)
        header = match.group(2).strip()
        span = SourceSpan(
            body_start_line + body.count("\n", 0, match.start()),
            body_start_line + body.count("\n", 0, block_end),
        )
        child_body_start_line = body_start_line + body.count("\n", 0, block_start)
        blocks.append((kind, header, body[block_start:block_end], span, child_body_start_line))
        index = block_end + 1
    return blocks


def _is_top_level_at(text: str, offset: int) -> bool:
    depth = 0
    for char in text[:offset]:
        if char == "{":
            depth += 1
        elif char == "}":
            depth = max(0, depth - 1)
    return depth == 0


def _parse_within_header(header: str) -> tuple[str, dict[str, str]]:
    if "(" not in header:
        return header.strip(), {}
    context, args = header.split("(", 1)
    args = args.rsplit(")", 1)[0].strip()
    parameters: dict[str, str] = {}
    for raw_arg in args.split(","):
        arg = raw_arg.strip()
        if not arg or ":" not in arg:
            continue
        name, value = arg.split(":", 1)
        parameters[name.strip()] = value.strip()
    return context.strip(), parameters


def _substitute_within_bindings(
    within: WithinDecl, replacements: dict[str, str]
) -> WithinDecl:
    return WithinDecl(
        context=within.context,
        span=within.span,
        parameters={
            name: _substitute_process_bindings(value, replacements)
            for name, value in within.parameters.items()
        },
        entered_by=_substitute_blocks(within.entered_by, replacements),
        depends_on=_substitute_blocks(within.depends_on, replacements),
        drives=_substitute_blocks(within.drives, replacements),
        within=[
            _substitute_within_bindings(child, replacements)
            for child in within.within
        ],
        exited_by=_substitute_blocks(within.exited_by, replacements),
        may_change=_substitute_blocks(within.may_change, replacements),
        ensures=_substitute_blocks(within.ensures, replacements),
        deferred=_substitute_blocks(within.deferred, replacements),
        other_blocks=_substitute_blocks(within.other_blocks, replacements),
    )


def _substitute_blocks(blocks: list[Block], replacements: dict[str, str]) -> list[Block]:
    return [
        Block(
            block.kind,
            _substitute_process_bindings(block.body, replacements),
            block.span,
            header=_substitute_process_bindings(block.header, replacements),
            body_start_line=block.body_start_line,
        )
        for block in blocks
    ]


def _owned_child_type(model: ObjectModel, parent_name: str, child_name: str) -> str | None:
    parent = model.objects.get(parent_name)
    if parent is None:
        return None

    candidates: list[Block] = []
    candidates.extend(
        block for block in parent.decl.other_blocks if block.kind == "owned"
    )
    type_decl = model.types.get(parent.kind)
    if type_decl is not None:
        candidates.extend(block for block in type_decl.blocks if block.kind == "owned")

    pattern = re.compile(
        r"\A" + re.escape(child_name) + r"\s*:\s*([A-Z][A-Za-z0-9_]*)\Z"
    )
    for block in candidates:
        for entry in block.entries:
            match = pattern.match(entry)
            if match is not None:
                return match.group(1)
    return None


def _process_argument_bindings(
    model: ObjectModel,
    type_name: str,
    process_kind: str,
    process_name: str,
    args: str | None,
) -> dict[str, str]:
    signature = _process_signature(model, type_name, process_kind, process_name)
    if signature is None or args is None:
        return {}
    raw_args = [item.strip() for item in args.split(",") if item.strip()]
    if _uses_named_args(args):
        bindings: dict[str, str] = {}
        for item in raw_args:
            name, sep, value = item.partition(":")
            if sep:
                bindings[name.strip()] = value.strip()
        return bindings
    if len(signature) != len(raw_args):
        return {}
    return {
        param_name: value
        for (param_name, _param_type), value in zip(signature, raw_args, strict=True)
    }


def _substitute_process_bindings(expression: str, bindings: dict[str, str]) -> str:
    substituted = expression
    for name, value in bindings.items():
        substituted = re.sub(rf"\b{re.escape(name)}\b", value, substituted)
    return substituted


def _named_block_body(body: str, block_name: str) -> str | None:
    pattern = re.compile(r"\b" + re.escape(block_name) + r"\s*\{", re.S)
    match = pattern.search(body)
    if match is None:
        return None
    block_start = match.end()
    block_end = _matching_brace_index(body, block_start - 1)
    if block_end is None:
        return None
    return body[block_start:block_end]


def _top_level_named_block_body(body: str, block_name: str) -> str | None:
    pattern = re.compile(r"\b" + re.escape(block_name) + r"\s*\{", re.S)
    index = 0
    while index < len(body):
        match = pattern.search(body, index)
        if match is None:
            return None
        if not _is_top_level_at(body, match.start()):
            index = match.end()
            continue
        block_start = match.end()
        block_end = _matching_brace_index(body, block_start - 1)
        if block_end is None:
            return None
        return body[block_start:block_end]
    return None


def _matching_brace_index(text: str, open_index: int) -> int | None:
    depth = 0
    for index in range(open_index, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index
    return None


def _parse_process_parameters(params: str) -> tuple[tuple[str, str], ...]:
    params = params.strip()
    if not params:
        return ()
    parsed: list[tuple[str, str]] = []
    for item in params.split(","):
        name, sep, type_name = item.strip().partition(":")
        if sep:
            parsed.append((name.strip(), type_name.strip()))
    return tuple(parsed)


def _uses_named_args(args: str) -> bool:
    return any(
        re.match(r"\s*[a-z][A-Za-z0-9_]*\s*:(?!:)", item) is not None
        for item in _split_args(args)
    )


def _process_call_expression(callee: str, args: str | None) -> str:
    if args is None:
        return callee
    return f"{callee}({args.strip()})"


def _action_result_value(
    model: ObjectModel, object_name: str, action_name: str, type_name: str
) -> str | None:
    if type_name == "RunQueueRef" and action_name == "SelectRunQueue":
        obj = model.objects.get(object_name)
        if obj is None or obj.kind not in model.types:
            return None
        type_decl = model.types[obj.kind]
        pattern = re.compile(
            r"\bAction::"
            + re.escape(action_name)
            + r"\b.*?scheduler_select_runqueue_returns\([^,]+,\s*[^,]+,\s*([A-Z][A-Za-z0-9_]*)\)",
            re.S,
        )
        for block in type_decl.blocks:
            match = pattern.search(block.body)
            if match is not None:
                return match.group(1)
    if type_name == "TaskRef" and action_name == "PickNextTask":
        type_decl = model.types.get("RunQueue")
        if type_decl is None:
            return None
        pattern = re.compile(
            r"\bAction::"
            + re.escape(action_name)
            + r"\b.*?runqueue_pick_next_task_returns\([^,]+,\s*[^,]+,\s*([A-Z][A-Za-z0-9_]*)\)",
            re.S,
        )
        for block in type_decl.blocks:
            match = pattern.search(block.body)
            if match is not None:
                return match.group(1)
    return None


def _action_result_value_from_hints(
    model: ObjectModel,
    object_name: str,
    action_name: str,
    type_name: str,
    args: str | None,
    process_type: str | None,
    hints: tuple[str, ...],
    bindings: dict[str, dict[str, str]],
) -> str | None:
    signature = (
        _process_signature(model, process_type, "Action", action_name)
        if process_type is not None
        else None
    )
    call_args = _canonical_call_args(args, signature, bindings)
    receiver = _canonical_ref_value(object_name, bindings)
    if type_name == "RunQueueRef" and action_name == "SelectRunQueue":
        task_ref = call_args.get("task_ref")
        if task_ref is None:
            return None
        return _result_value_from_predicate(
            hints,
            "scheduler_select_runqueue_returns",
            (receiver, task_ref),
        )
    if type_name == "TaskRef" and action_name == "PickNextTask":
        prev_ref = call_args.get("prev_ref")
        if prev_ref is None:
            return None
        return _result_value_from_predicate(
            hints,
            "runqueue_pick_next_task_returns",
            (receiver, prev_ref),
        )
    return None


def _canonical_call_args(
    args: str | None,
    signature: tuple[tuple[str, str], ...] | None,
    bindings: dict[str, dict[str, str]],
) -> dict[str, str]:
    if args is None or signature is None:
        return {}
    raw_args = _split_args(args)
    if _uses_named_args(args):
        by_name: dict[str, str] = {}
        for item in raw_args:
            name, sep, value = item.partition(":")
            if sep:
                by_name[name.strip()] = _canonical_ref_value(value.strip(), bindings)
        return by_name
    if len(raw_args) != len(signature):
        return {}
    return {
        name: _canonical_ref_value(value, bindings)
        for (name, _type_name), value in zip(signature, raw_args, strict=True)
    }


def _canonical_ref_value(
    value: str, bindings: dict[str, dict[str, str]]
) -> str:
    binding = bindings.get(value)
    if binding is not None:
        return binding.get("value", value)
    return value


def _canonical_process_args(
    model: ObjectModel,
    type_name: str,
    process_kind: str,
    process_name: str,
    args: str | None,
    bindings: dict[str, dict[str, str]],
) -> str | None:
    signature = _process_signature(model, type_name, process_kind, process_name)
    if args is None or signature is None:
        return args
    raw_args = _split_args(args)
    if _uses_named_args(args):
        rendered: list[str] = []
        signature_by_name = dict(signature)
        for item in raw_args:
            name, sep, value = item.partition(":")
            if not sep:
                rendered.append(item)
                continue
            arg_name = name.strip()
            rendered.append(
                f"{arg_name}: "
                + _canonical_process_arg_value(
                    model,
                    value.strip(),
                    signature_by_name.get(arg_name),
                    bindings,
                )
            )
        return ", ".join(rendered)
    if len(raw_args) != len(signature):
        return args
    return ", ".join(
        _canonical_process_arg_value(model, value, param_type, bindings)
        for (_param_name, param_type), value in zip(signature, raw_args, strict=True)
    )


def _canonical_process_arg_value(
    model: ObjectModel,
    value: str,
    param_type: str | None,
    bindings: dict[str, dict[str, str]],
) -> str:
    binding = bindings.get(value)
    if binding is None:
        return value
    return _process_param_binding(model, binding, value, param_type).get("value", value)


def _result_value_from_predicate(
    hints: tuple[str, ...],
    predicate: str,
    expected_prefix: tuple[str, ...],
) -> str | None:
    for hint in reversed(hints):
        parsed = _predicate_args(hint)
        if parsed is None:
            continue
        hint_predicate, hint_args = parsed
        if (
            hint_predicate == predicate
            and len(hint_args) == len(expected_prefix) + 1
            and tuple(hint_args[: len(expected_prefix)]) == expected_prefix
        ):
            return hint_args[-1]
    return None


def _predicate_args(expression: str) -> tuple[str, tuple[str, ...]] | None:
    match = re.match(r"\A([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\Z", expression.strip(), re.S)
    if match is None:
        return None
    return match.group(1), tuple(_split_args(match.group(2)))


def _split_args(args: str) -> list[str]:
    parts: list[str] = []
    current: list[str] = []
    depth = 0
    in_string = False
    escaped = False
    for char in args:
        if in_string:
            current.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
            current.append(char)
        elif char == "(":
            depth += 1
            current.append(char)
        elif char == ")":
            depth -= 1
            current.append(char)
        elif char == "," and depth == 0:
            part = "".join(current).strip()
            if part:
                parts.append(part)
            current = []
        else:
            current.append(char)
    part = "".join(current).strip()
    if part:
        parts.append(part)
    return parts


def _block_entries(blocks: list[Block]) -> tuple[str, ...]:
    return tuple(entry for block in blocks for entry in block.entries)


def _context_object(event: EventDef | None, state: StateDef | None) -> str | None:
    if event is not None:
        return event.object_name
    if state is not None:
        return state.object_name
    return None


def _event_label(object_name: str, event_name: str) -> str:
    return f"{object_name}.Event::{event_name}"


def _derived_alias(expression: str) -> str | None:
    match = re.match(
        r"\Atask_runtime_state_is\(\s*([A-Z][A-Za-z0-9_]*)\s*,\s*TaskRuntimeState::Running\s*\)\Z",
        expression,
    )
    if match is None:
        return None
    return f"task_state_running({match.group(1)})"


def _strip_quotes(value: str) -> str:
    stripped = value.strip()
    if len(stripped) >= 2 and stripped[0] == '"' and stripped[-1] == '"':
        return stripped[1:-1]
    return stripped


def _record_counts(records: tuple[DerivationRecord, ...]) -> dict[DerivationStatus, int]:
    counts: dict[DerivationStatus, int] = {}
    for record in records:
        counts[record.status] = counts.get(record.status, 0) + 1
    return counts


def _format_obligation_category_summary(records: list[DerivationRecord]) -> list[str]:
    counts: dict[str, int] = {}
    for record in records:
        category = record.obligation_category or "unknown"
        counts[category] = counts.get(category, 0) + 1
    if not counts:
        return []
    summary = ", ".join(f"{name}={count}" for name, count in sorted(counts.items()))
    return [f"  categories: {summary}"]


def _format_obligation_provider_summary(records: list[DerivationRecord]) -> list[str]:
    counts: dict[tuple[str, str], int] = {}
    for record in records:
        proof_provider = record.proof_provider or "unknown"
        proof_class = record.proof_class or "unknown"
        key = (proof_provider, proof_class)
        counts[key] = counts.get(key, 0) + 1
    if not counts:
        return []

    lines = ["  providers:"]
    for (proof_provider, proof_class), count in sorted(
        counts.items(), key=lambda item: (-item[1], item[0][0], item[0][1])
    ):
        lines.append(f"    {proof_provider}/{proof_class}: {count}")
    return lines


_PRIOR_FACT_PROOFS = {
    "firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa)": (
        "firmware_entry_state",
        {
            "firmware_dtb_blob_complete_at_kernel_entry(BootArgs.dtb_pa)",
        },
    ),
    "firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa)": (
        "firmware_entry_state",
        {
            "firmware_dtb_blob_accessible_at_kernel_entry(BootArgs.dtb_pa)",
        },
    ),
    "boot_cpu_hartid_ready(BootCPU, BootArgs.boot_hartid)": (
        "boot_hart_identity",
        {
            "boot_cpu_hartid_ready(BootCPU, BootArgs.boot_hartid)",
        },
    ),
    "boot_cpu_present(BootCPU)": (
        "cpu_state",
        {
            "boot_cpu_present(BootCPU)",
        },
    ),
    "boot_cpu_active(BootCPU)": (
        "cpu_state",
        {
            "boot_cpu_active(BootCPU)",
        },
    ),
    "slot_contains(FixMap.fdt_slot, RawDtb)": (
        "fixmap_slot_content",
        {
            "slot_contains(FixMap.fdt_slot, RawDtb)",
        },
    ),
    "trampoline_mapping_ready(StaticObjects.trampoline_pg_dir, TrampolineMap)": (
        "address_mapping",
        {
            "trampoline_mapping_ready(StaticObjects.trampoline_pg_dir, TrampolineMap)",
        },
    ),
    "valid_task_ref(Riscv64.tp)": (
        "object_storage",
        {
            "Riscv64.tp == phys_addr(StaticObjects.init_task)",
            "Riscv64.tp == virt_addr(StaticObjects.init_task, EarlyVm, KernelImageMap)",
        },
    ),
    "valid_task_storage(StaticObjects.init_task)": (
        "object_storage",
        {
            "valid_object_storage(init_task)",
        },
    ),
    "valid_stack_pointer(Riscv64.sp)": (
        "architecture_state",
        {
            "Riscv64.sp == phys_addr(Lds.init_stack_end - Config.pt_size_on_stack)",
            "Riscv64.sp == virt_addr(Lds.init_stack_end - Config.pt_size_on_stack, EarlyVm, KernelImageMap)",
        },
    ),
    "inside(Riscv64.sp, Lds.init_stack_end, Lds.init_stack_start, Lds.init_stack_end)": (
        "stack_layout",
        {
            "Riscv64.sp == phys_addr(Lds.init_stack_end - Config.pt_size_on_stack)",
        },
    ),
    "inside(Riscv64.sp, virt_addr(Lds.init_stack_end, EarlyVm, KernelImageMap), virt_addr(Lds.init_stack_start, EarlyVm, KernelImageMap), virt_addr(Lds.init_stack_end, EarlyVm, KernelImageMap))": (
        "stack_layout",
        {
            "Riscv64.sp == virt_addr(Lds.init_stack_end - Config.pt_size_on_stack, EarlyVm, KernelImageMap)",
        },
    ),
}


def _prior_fact_proof_class(
    expression: str, proved_expressions: set[str]
) -> str | None:
    proof = _PRIOR_FACT_PROOFS.get(expression)
    if proof is not None:
        proof_class, required = proof
        if proved_expressions.intersection(required):
            return proof_class
    if expression in proved_expressions:
        return "derived_fact"
    return None


def _classify_obligation(
    expression: str, source_kind: str, context_object: str | None
) -> dict[str, str | None]:
    predicate = _predicate_name(expression)
    if predicate == "attrs_accessible":
        proof_class, proof_provider = _attrs_accessible_proof(
            expression, context_object
        )
        return {
            "predicate": predicate,
            "category": "derived_candidate",
            "proof_class": proof_class,
            "proof_provider": proof_provider,
        }
    if expression in _CONTAINS_PROOFS:
        proof_class, proof_provider = _CONTAINS_PROOFS[expression]
        return {
            "predicate": predicate,
            "category": "derived_candidate",
            "proof_class": proof_class,
            "proof_provider": proof_provider,
        }
    if expression in _RELATION_PROOFS:
        proof_class, proof_provider = _RELATION_PROOFS[expression]
        return {
            "predicate": predicate,
            "category": "derived_candidate",
            "proof_class": proof_class,
            "proof_provider": proof_provider,
        }
    if predicate in _AUTO_PREDICATES:
        return {
            "predicate": predicate,
            "category": "auto_candidate",
            "proof_class": _AUTO_PREDICATES[predicate],
            "proof_provider": "builtin_candidate",
        }
    if predicate in _ASSUMPTION_PREDICATES:
        return {
            "predicate": predicate,
            "category": "assumption_candidate",
            "proof_class": _ASSUMPTION_PREDICATES[predicate],
            "proof_provider": "assumption_candidate",
        }
    if predicate in _EXTERNAL_PREDICATES:
        return {
            "predicate": predicate,
            "category": "derived_candidate",
            "proof_class": _EXTERNAL_PREDICATES[predicate],
            "proof_provider": _DERIVED_PROVIDERS.get(predicate, "derived_candidate"),
        }

    if _is_relation_expression(expression):
        return {
            "predicate": predicate,
            "category": "auto_candidate",
            "proof_class": "relation",
            "proof_provider": "builtin_candidate",
        }

    if source_kind == "depends_on":
        category = "spec_gap"
    else:
        category = "unknown"
    return {
        "predicate": predicate,
        "category": category,
        "proof_class": "unknown",
        "proof_provider": "unknown",
    }


def _predicate_name(expression: str) -> str | None:
    match = _PREDICATE_CALL_RE.match(expression.strip())
    if match is None:
        return None
    return match.group(1)


def _fact_key(expression: str) -> str:
    stripped = expression.strip()
    if _predicate_name(stripped) is None:
        return stripped
    return _remove_unquoted_whitespace(stripped)


def _remove_unquoted_whitespace(text: str) -> str:
    chars: list[str] = []
    in_string = False
    escaped = False
    for char in text:
        if in_string:
            chars.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
            chars.append(char)
        elif not char.isspace():
            chars.append(char)
    return "".join(chars)


def _attrs_accessible_proof(
    expression: str, context_object: str | None
) -> tuple[str, str]:
    match = re.match(r"\Aattrs_accessible\(\s*([A-Z][A-Za-z0-9_]*)\s*\)\Z", expression)
    object_name = match.group(1) if match is not None else context_object
    if object_name in _ATTRS_ACCESSIBLE_PROOFS:
        return _ATTRS_ACCESSIBLE_PROOFS[object_name]
    return ("object_attributes", "derived_candidate")


def _is_relation_expression(expression: str) -> bool:
    return bool(_RELATION_RE.search(expression))


def _slot_field_name(slot_name: str) -> str:
    return slot_name[:1].lower() + slot_name[1:]


def _append_trace_node(
    lines: list[str], node: DerivationTraceNode, depth: int
) -> None:
    indent = "  " * depth
    lines.append(f"{indent}> {node.label} State::{node.source_state}")
    for child in node.children:
        _append_trace_node(lines, child, depth + 1)

    suffix = ""
    if node.status in (DerivationStatus.BLOCKED, DerivationStatus.CONTRADICTION):
        suffix = f" {node.status.value}"
        if node.message:
            suffix += f": {node.message}"
    lines.append(f"{indent}< {node.label} State::{node.target_state}{suffix}")


def _format_record(record: DerivationRecord) -> str:
    location = ""
    if record.span is not None:
        location = f"line {record.span.start_line}: "
    proof = ""
    if record.proof_provider or record.proof_class:
        proof_provider = record.proof_provider or "unknown"
        proof_class = record.proof_class or "unknown"
        proof = f" [{proof_provider}/{proof_class}]"
    return f"{location}{record.message}{proof}"


__all__ = [
    "DEFAULT_TARGET",
    "DerivationRecord",
    "DerivationResult",
    "DerivationStatus",
    "DerivationTraceNode",
    "EventTransition",
    "derive",
    "render_derivation_text",
    "summarize_derivation",
]
