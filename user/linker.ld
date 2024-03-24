ENTRY(_start)

SECTIONS {
    . = 0x4000000 + SIZEOF_HEADERS;

    .text : ALIGN(64k) {
        *(.text*)
    }
    .rodata : ALIGN(64k) {
        *(.rodata*)
    }
    .data : ALIGN(64k) {
        *(.data*)
    }
    .bss : ALIGN(64k) {
        *(.bss*)
    }
}
