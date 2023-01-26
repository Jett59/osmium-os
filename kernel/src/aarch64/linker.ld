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
    .beryllium : {
        BYTE(66) 
        BYTE(101)
        BYTE(114)
        BYTE(121)
        BYTE(108)
        BYTE(108)
        BYTE(105)
        BYTE(117)
        BYTE(109)
        BYTE(32)
        BYTE(82)
        BYTE(101)
        BYTE(97)
        BYTE(100)
        BYTE(121)
        BYTE(33)
    }
    /DISCARD/ : {
        *(.eh_frame*)
        *(.note*)
        *(.comment)
        *(.got)
    }
}
