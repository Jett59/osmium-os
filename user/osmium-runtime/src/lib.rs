#![no_std]
#![feature(naked_functions, asm_const)]

use core::arch::asm;

#[cfg_attr(not(test), panic_handler)]
pub fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// The loader doesn't set up a stack for us, but it is simple enough to do that for ourselves:
const STACK_SIZE: usize = 65536;
static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

extern "C" {
    fn main();
}

#[naked]
#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe {
        // Set up the stack pointer
        #[cfg(target_arch = "aarch64")]
        asm!(
            "adr x0, {} + {}",
            "mov sp, x0",
            "bl {}",
            "b .",
            sym STACK,
            const STACK_SIZE,
            sym main,
            options(noreturn)
        );
        #[cfg(target_arch = "x86_64")]
        asm!(
            "lea {} + {}(%rip), %rsp",
            "call {}",
            "jmp .",
            sym STACK,
            const STACK_SIZE,
            sym main,
            options(noreturn, att_syntax)
        );
    }
}
