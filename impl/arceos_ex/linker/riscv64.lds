OUTPUT_ARCH(riscv)
ENTRY(_start)

KERNEL_LINK_ADDR = 0xffffffff80000000;
LOAD_OFFSET = KERNEL_LINK_ADDR;

SECTIONS
{
    . = LOAD_OFFSET;
    kernel_start = .;
    _stext = .;

    .head.text : AT(ADDR(.head.text) - LOAD_OFFSET) ALIGN(4K) {
        __head_text_start = .;
        KEEP(*(.head.text.entry))
        KEEP(*(.head.text .head.text.*))
        __head_text_end = .;
    }

    .text : AT(ADDR(.text) - LOAD_OFFSET) ALIGN(4K) {
        *(.text .text.*)
    }
    _etext = .;

    .rodata : AT(ADDR(.rodata) - LOAD_OFFSET) ALIGN(4K) {
        _srodata = .;
        *(.rodata .rodata.*)
        _erodata = .;
    }

    . = ALIGN(4);
    __ex_table : AT(ADDR(__ex_table) - LOAD_OFFSET) {
        __start___ex_table = .;
        KEEP(*(__ex_table))
        __stop___ex_table = .;
    }

    .data : AT(ADDR(.data) - LOAD_OFFSET) ALIGN(4K) {
        _sdata = .;
        PROVIDE(__global_pointer$ = . + 0x800);
        KEEP(*(.head.handoff))
        *(.sdata .sdata.*)
        *(.data)
    }
    _edata = .;

    . = ALIGN(4K);
    .data..percpu : AT(ADDR(.data..percpu) - LOAD_OFFSET) {
        __per_cpu_load = .;
        __per_cpu_start = .;
        KEEP(*(.data..percpu..first))
        . = ALIGN(4K);
        *(.data..percpu..page_aligned)
        . = ALIGN(64);
        *(.data..percpu..read_mostly)
        . = ALIGN(64);
        *(.data..percpu)
        *(.data..percpu.*)
        . = ALIGN(64);
        __per_cpu_end = .;
    }

    .data.tail : AT(ADDR(.data.tail) - LOAD_OFFSET) ALIGN(4K) {
        *(.data.*)
    }

    .bss : AT(ADDR(.bss) - LOAD_OFFSET) ALIGN(4K) {
        _sbss = .;
        __bss_start = .;
        KEEP(*(.bss.objects))
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        _ebss = .;
        __bss_stop = .;
    }

    .boot.stack (NOLOAD) : AT(ADDR(.boot.stack) - LOAD_OFFSET) ALIGN(4K) {
        init_stack_start = .;
        KEEP(*(.boot.stack))
        . += 16K;
        . = ALIGN(4K);
        init_stack_end = .;
    }

    _end = .;
    kernel_end = .;

    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
    }
}
