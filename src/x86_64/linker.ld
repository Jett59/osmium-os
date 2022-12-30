ENTRY(_start)

/* If this is changed, other things (mainly in the startup code) will break. Please change those first unless you want the code to break*/
KERNEL_VIRTUAL_OFFSET = 0xffffffff80000000;

SECTIONS {
    . = 0x200000 + SIZEOF_HEADERS;

    .multiboot : ALIGN(4k) {
        KEEP(*(.multiboot*))
        *(.bootstrap*)
    }

. += KERNEL_VIRTUAL_OFFSET;

        .text : AT(ADDR(.text) - KERNEL_VIRTUAL_OFFSET) {
        *(.text*)
    }
    .data : AT(ADDR(.data) - KERNEL_VIRTUAL_OFFSET) {
        *(.data*)
    }
    .rodata : AT(ADDR(.rodata) - KERNEL_VIRTUAL_OFFSET) {
        *(.rodata*)
    }
    .bss : AT(ADDR(.bss) - KERNEL_VIRTUAL_OFFSET)  {
        *(.bss)
        *(.bss*)
        *(common)
        *(common*)
    }
}
