SECTIONS {
    . = 0xffffffff80000000;
    PROVIDE(_kernel_start = .);

    .text : { *(.text*) }

    . = ALIGN(4096);
    .rodata : { *(.rodata*) }

    . = ALIGN(4096);
    .data : { *(.data*) }
    .bss : { *(.bss*) }

    PROVIDE(_kernel_end = .);
}
