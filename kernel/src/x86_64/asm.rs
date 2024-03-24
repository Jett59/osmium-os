use core::arch::asm;

/// # Safety
/// This function could cause undefined behavior if the port does something strange.
/// For example, a port-mapped DMA controller could overwrite parts of the kernel.
pub unsafe fn write_port8(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack));
}

pub fn io_wait() {
    unsafe {
        asm!("out dx, al", in("dx") 0x80, in("al") 0u8, options(nomem, nostack));
    }
}

/// # Safety
/// This could break code in the surrounding scope which relies on interrupts being disabled.
pub unsafe fn enable_interrupts() {
    asm!("sti", options(nomem, nostack));
}

pub unsafe fn iret(
    stack_segment: u64,
    stack_pointer: u64,
    flags: u64,
    code_segment: u64,
    instruction_pointer: u64,
) -> ! {
    asm!(
        "push {}",
        "push {}",
        "push {}",
        "push {}",
        "push {}",
        "iretq",
        in(reg) stack_segment, in(reg) stack_pointer, in(reg) flags, in(reg) code_segment, in(reg) instruction_pointer, options(nomem, nostack, noreturn));
}
