#![no_std]

use core::arch::naked_asm;

#[cfg_attr(not(test), panic_handler)]
pub fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// The loader doesn't set up a stack for us, but it is simple enough to do that for ourselves:
const STACK_SIZE: usize = 65536;
static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

unsafe extern "C" {
    fn main();
}

#[unsafe(naked)]
#[cfg_attr(not(test), unsafe(no_mangle))]
pub extern "C" fn _start() -> ! {
    // Set up the stack pointer
    #[cfg(target_arch = "aarch64")]
    naked_asm!(
        "adr x0, {} + {}",
        "mov sp, x0",
        "bl {}",
        "b .",
        sym STACK,
        const STACK_SIZE,
        sym main,
    );
    #[cfg(target_arch = "x86_64")]
    naked_asm!(
        "lea {} + {}(%rip), %rsp",
        "call {}",
        "jmp .",
        sym STACK,
        const STACK_SIZE,
        sym main,
        options(att_syntax)
    );
}
