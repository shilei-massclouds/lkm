OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;
    PROVIDE(kernel_start = .);
    PROVIDE(_stext = .);

    .head.text : ALIGN(4K) {
        KEEP(*(.head.text))
    }

    .text : ALIGN(4K) {
        *(.text .text.*)
    }
    PROVIDE(_etext = .);

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K) {
        PROVIDE(__global_pointer$ = . + 0x800);
        *(.sdata .sdata.*)
        *(.data .data.*)
    }
    PROVIDE(_edata = .);

    .bss : ALIGN(4K) {
        PROVIDE(_sbss = .);
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        PROVIDE(_ebss = .);
    }

    PROVIDE(_end = .);
    PROVIDE(kernel_end = .);

    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
    }
}
