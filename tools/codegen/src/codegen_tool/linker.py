"""Generate linker scripts from model Lds/Config facts."""

from __future__ import annotations

from dataclasses import dataclass

from common.model_types import ObjectModel


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


def generate_riscv64_linker_script(model: ObjectModel, profile: LinkerProfile) -> str:
    """Generate the current arceos_ex RISC-V64 linker script."""

    _validate_model_contract(model)
    return _render(profile)


def _validate_model_contract(model: ObjectModel) -> None:
    config = model.objects.get("Config")
    if config is None:
        raise ValueError("model does not define Config")
    missing_config = sorted(_REQUIRED_CONFIG_ATTRS.difference(config.attrs))
    if missing_config:
        raise ValueError(f"Config is missing attrs: {', '.join(missing_config)}")

    lds = model.objects.get("Lds")
    if lds is None:
        raise ValueError("model does not define Lds")
    missing_lds = sorted(_REQUIRED_LDS_ATTRS.difference(lds.attrs))
    if missing_lds:
        raise ValueError(f"Lds is missing attrs: {', '.join(missing_lds)}")

    online = lds.states.get("Online")
    if online is None:
        raise ValueError("Lds does not define State::Online")
    invariants = {
        entry
        for block in online.decl.invariants
        for entry in block.entries
    }
    missing_invariants = sorted(_REQUIRED_LDS_INVARIANTS.difference(invariants))
    if missing_invariants:
        raise ValueError(
            "Lds.Online is missing invariants: " + ", ".join(missing_invariants)
        )


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
        *(.text .text.*)
    }}
    _etext = .;

    .rodata : AT(ADDR(.rodata) - LOAD_OFFSET) ALIGN({profile.page_size}) {{
        _srodata = .;
        *(.rodata .rodata.*)
        _erodata = .;
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
        *(.data..percpu..page_aligned)
        . = ALIGN(64);
        *(.data..percpu..read_mostly)
        . = ALIGN(64);
        *(.data..percpu)
        *(.data..percpu.*)
        . = ALIGN(64);
        __per_cpu_end = .;
    }}

    .data.tail : AT(ADDR(.data.tail) - LOAD_OFFSET) ALIGN({profile.page_size}) {{
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
