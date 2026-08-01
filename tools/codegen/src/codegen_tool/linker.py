"""Generate linker scripts from model Lds/Config facts."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

@dataclass(frozen=True)
class LinkerProfile:
    """Concrete values for the current implementation profile."""

    kernel_link_addr: str
    page_size: str
    boot_stack_size: str


_REQUIRED_CONFIG_ATTRS = {
    "boot_stack_size",
    "kernel_link_addr",
    "page_size",
}

_REQUIRED_LDS_ATTRS = {
    "boot_stack_size",
    "bss_end",
    "bss_start",
    "global_pointer",
    "head_text_range",
    "init_stack_end",
    "init_stack_start",
    "kernel_end",
    "kernel_start",
    "per_cpu_end",
    "per_cpu_load",
    "per_cpu_start",
    "data_end",
    "data_start",
    "rodata_end",
    "rodata_start",
    "text_end",
    "text_start",
}

_REQUIRED_LDS_INVARIANTS = {
    "boot_stack_size == Config.boot_stack_size",
    "init_stack_end - init_stack_start == boot_stack_size",
    "text_start == kernel_start",
    "inside(text_start, text_end, kernel_start, kernel_end)",
    "inside(rodata_start, rodata_end, kernel_start, kernel_end)",
    "inside(data_start, data_end, kernel_start, kernel_end)",
    "entry_head_text_layout_ready(Lds)",
    "per_cpu_static_image_layout_ready(Lds)",
}


def generate_riscv64_linker_script(model: dict[str, Any], profile: LinkerProfile) -> str:
    """Generate the current arceos_ex RISC-V64 linker script."""

    _validate_model_contract(model)
    return _render(profile)


def _validate_model_contract(model: dict[str, Any]) -> None:
    _validate_tools2_model_contract(model)


def _validate_tools2_model_contract(envelope: dict[str, Any]) -> None:
    if envelope.get("schema") != "lkm.spec.model" or envelope.get("version") != 10:
        raise ValueError("linker codegen requires tools2 model protocol v10")
    if envelope.get("producer") != "tools2":
        raise ValueError("linker codegen requires producer=tools2")
    model = envelope.get("model")
    if not isinstance(model, dict):
        raise ValueError("tools2 model envelope is missing model")
    systems = model.get("systems")
    if not isinstance(systems, dict):
        raise ValueError("tools2 model is missing systems")

    config = systems.get("Config")
    if not isinstance(config, dict):
        raise ValueError("model does not define Config")
    config_attrs = _tools2_attr_names(config)
    missing_config = sorted(_REQUIRED_CONFIG_ATTRS.difference(config_attrs))
    if missing_config:
        raise ValueError(f"Config is missing attrs: {', '.join(missing_config)}")

    lds = systems.get("Lds")
    if not isinstance(lds, dict):
        raise ValueError("model does not define Lds")
    lds_attrs = _tools2_attr_names(lds)
    missing_lds = sorted(_REQUIRED_LDS_ATTRS.difference(lds_attrs))
    if missing_lds:
        raise ValueError(f"Lds is missing attrs: {', '.join(missing_lds)}")

    states = lds.get("states")
    online = states.get("Online") if isinstance(states, dict) else None
    if not isinstance(online, dict):
        raise ValueError("Lds does not define State::Online")
    invariants = online.get("invariant")
    invariant_text = {
        item.get("text")
        for item in invariants
        if isinstance(invariants, list) and isinstance(item, dict)
    } if isinstance(invariants, list) else set()
    missing_invariants = sorted(_REQUIRED_LDS_INVARIANTS.difference(invariant_text))
    if missing_invariants:
        raise ValueError(
            "Lds.Online is missing invariants: " + ", ".join(missing_invariants)
        )


def _tools2_attr_names(system: dict[str, Any]) -> set[str]:
    fields = system.get("fields")
    attrs = fields.get("attrs") if isinstance(fields, dict) else None
    if not isinstance(attrs, list):
        return set()
    return {
        item["name"]
        for item in attrs
        if isinstance(item, dict) and isinstance(item.get("name"), str)
    }


def _render(profile: LinkerProfile) -> str:
    return f"""OUTPUT_ARCH(riscv)
ENTRY(_start)

KERNEL_LINK_ADDR = {profile.kernel_link_addr};
LOAD_OFFSET = KERNEL_LINK_ADDR;

SECTIONS
{{
    . = LOAD_OFFSET;
    kernel_start = .;
    _stext = .;

    .head.text : AT(ADDR(.head.text) - LOAD_OFFSET) ALIGN({profile.page_size}) {{
        __head_text_start = .;
        KEEP(*(.head.text.entry))
        KEEP(*(.head.text .head.text.*))
        __head_text_end = .;
    }}

    .text : AT(ADDR(.text) - LOAD_OFFSET) ALIGN({profile.page_size}) {{
        KEEP(*(.init.text .init.text.*))
        *(.text .text.*)
    }}
    _etext = .;

    .rodata : AT(ADDR(.rodata) - LOAD_OFFSET) ALIGN({profile.page_size}) {{
        _srodata = .;
        *(.rodata .rodata.*)
        _erodata = .;
    }}

    . = ALIGN(4);
    __ex_table : AT(ADDR(__ex_table) - LOAD_OFFSET) {{
        __start___ex_table = .;
        KEEP(*(__ex_table))
        __stop___ex_table = .;
    }}

    . = ALIGN(8);
    .initcall : AT(ADDR(.initcall) - LOAD_OFFSET) {{
        __initcall_pure_start = .;
        KEEP(*(.initcall.pure))
        __initcall_pure_end = .;
        __initcall_core_start = .;
        KEEP(*(.initcall.core))
        __initcall_core_end = .;
        __initcall_postcore_start = .;
        KEEP(*(.initcall.postcore))
        __initcall_postcore_end = .;
        __initcall_arch_start = .;
        KEEP(*(.initcall.arch))
        __initcall_arch_end = .;
        __initcall_subsys_start = .;
        KEEP(*(.initcall.subsys))
        __initcall_subsys_end = .;
        __initcall_fs_start = .;
        KEEP(*(.initcall.fs))
        __initcall_fs_end = .;
        __initcall_device_start = .;
        KEEP(*(.initcall.device))
        __initcall_device_end = .;
        __initcall_late_start = .;
        KEEP(*(.initcall.late))
        __initcall_late_end = .;
    }}

    . = ALIGN(8);
    .linux_initcall : AT(ADDR(.linux_initcall) - LOAD_OFFSET) {{
        __linux_initcall6_start = .;
        KEEP(*(.initcall6.init))
        __linux_initcall6_end = .;
    }}

    . = ALIGN(8);
    __irqchip_of_table : AT(ADDR(__irqchip_of_table) - LOAD_OFFSET) {{
        __linux_irqchip_of_table_start = .;
        KEEP(*(__irqchip_of_table))
        __linux_irqchip_of_table_end = .;
    }}

    . = ALIGN(8);
    .alternative : AT(ADDR(.alternative) - LOAD_OFFSET) {{
        __linux_alternative_start = .;
        KEEP(*(.alternative))
        __linux_alternative_end = .;
    }}

    . = ALIGN(8);
    __bug_table : AT(ADDR(__bug_table) - LOAD_OFFSET) {{
        __linux_bug_table_start = .;
        KEEP(*(__bug_table))
        __linux_bug_table_end = .;
    }}

    . = ALIGN(8);
    .irqchip_init : AT(ADDR(.irqchip_init) - LOAD_OFFSET) {{
        __irqchip_init_start = .;
        KEEP(*(.irqchip.init))
        __irqchip_init_end = .;
    }}

    .data : AT(ADDR(.data) - LOAD_OFFSET) ALIGN({profile.page_size}) {{
        _sdata = .;
        PROVIDE(__global_pointer$ = . + 0x800);
        KEEP(*(.head.handoff))
        *(.sdata .sdata.*)
        *(.data)
    }}
    _edata = .;

    . = ALIGN({profile.page_size});
    .data..percpu : AT(ADDR(.data..percpu) - LOAD_OFFSET) {{
        __per_cpu_load = .;
        __per_cpu_start = .;
        KEEP(*(.data..percpu..first))
        . = ALIGN({profile.page_size});
        KEEP(*(.data..percpu..page_aligned))
        . = ALIGN(64);
        KEEP(*(.data..percpu..read_mostly))
        . = ALIGN(64);
        KEEP(*(.data..percpu))
        KEEP(*(.data..percpu.*))
        . = ALIGN(64);
        __per_cpu_end = .;
    }}

    .data.tail : AT(ADDR(.data.tail) - LOAD_OFFSET) ALIGN({profile.page_size}) {{
        KEEP(*(.data..ro_after_init))
        *(.data.*)
    }}

    .bss : AT(ADDR(.bss) - LOAD_OFFSET) ALIGN({profile.page_size}) {{
        _sbss = .;
        __bss_start = .;
        KEEP(*(.bss.objects))
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        _ebss = .;
        __bss_stop = .;
    }}

    .boot.stack (NOLOAD) : AT(ADDR(.boot.stack) - LOAD_OFFSET) ALIGN({profile.page_size}) {{
        init_stack_start = .;
        KEEP(*(.boot.stack))
        . += {profile.boot_stack_size};
        . = ALIGN({profile.page_size});
        init_stack_end = .;
    }}

    _end = .;
    kernel_end = .;

    /DISCARD/ : {{
        *(.eh_frame)
        *(.comment)
    }}
}}
"""
