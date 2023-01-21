ENTRY(kmain) /* The bootloader goes straight into Rust code. */

KERNEL_VIRTUAL_ADDRESS = 0xffffffff80000000;

SECTIONS {
    . = KERNEL_VIRTUAL_ADDRESS;
    .text : {
        *(.text*)
    }
    .rodata : {
        *(.rodata*)
    }
    .data : {
        *(.data*)
    }
    .bss : {
        *(.bss*)
    }
    /DISCARD/ : {
        *(.eh_frame*)
        *(.note*)
        *(.comment)
        *(.got)
    }
}
