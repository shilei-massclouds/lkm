OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;
    kernel_start = .;
    _stext = .;

    .head.text : ALIGN(4K) {
        __head_text_start = .;
        KEEP(*(.head.text))
        __head_text_end = .;
    }

    .text : ALIGN(4K) {
        *(.text .text.*)
    }
    _etext = .;

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K) {
        PROVIDE(__global_pointer$ = . + 0x800);
        *(.sdata .sdata.*)
        *(.data .data.*)
    }
    _edata = .;

    .bss : ALIGN(4K) {
        _sbss = .;
        __bss_start = .;
        KEEP(*(.bss.objects))
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        _ebss = .;
        __bss_stop = .;
    }

    .boot.stack : ALIGN(4K) {
        init_stack_start = .;
        KEEP(*(.boot.stack))
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
