SECTIONS {
    . = 0xffffffff80000000;
    PROVIDE(_kernel_start = .);

    .text : { *(.text*) }
    .rodata : { *(.rodata*) }
    .data : { *(.data*) }
    .bss : { *(.bss*) }

    PROVIDE(_kernel_end = .);
}
