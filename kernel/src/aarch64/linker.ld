ENTRY(kmain) /* The bootloader goes straight into Rust code. */

KERNEL_VIRTUAL_ADDRESS = 0xffffffff80000000;

SECTIONS {
    . = KERNEL_VIRTUAL_ADDRESS + SIZEOF_HEADERS;
    .beryllium : ALIGN(4k) {
        /* "Berylliu" (note little endian) */
        QUAD(0x75696c6c79726542)
        /* "m Ready!" */
        QUAD(0x217964616552206d)

        /* Stack pointer tag */
        LONG(0) /* BootRequestTagType::StackPointer */
        SHORT(24) /* size */
        SHORT(0) /* flags */
        QUAD(kernel_stack_start) /* base */
        QUAD(kernel_stack_end - kernel_stack_start) /* memory_size */

        KEEP(*(.beryllium))
    }
    .text : ALIGN(4k) {
        *(.text*)
    }
    .rodata : ALIGN(4k) {
        *(.rodata*)
    }
    .data : ALIGN(4k) {
        *(.data*)
    }
    .bss : ALIGN(4k) {
        kernel_stack_start = .;
        . += 65536;
        kernel_stack_end = .;
        *(.bss*)
    }
    /DISCARD/ : {
        *(.eh_frame*)
        *(.note*)
        *(.comment)
        *(.got)
    }
}
