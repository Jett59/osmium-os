use crate::user_memory::UserAddressSpaceHandle;
use core::{
    arch::asm,
    ptr::null_mut,
    sync::atomic::{self, AtomicPtr},
};
use syscall_interface::{decode_syscall, encode_syscall_result};

use crate::arch::asm::{LSTAR_MSR, SFMASK_MSR, STAR_MSR, USER_DATA_SELECTOR};

use super::asm::{write_msr, ALTERNATE_GS_BASE_MSR};

#[naked]
extern "C" fn syscall_entrypoint() {
    unsafe {
        // essentially we want to extract the rsp value from the alternate GS register, but without messing with any other registers.
        // This means we have to xchg the value at gs:0 with the value in rsp to get a valid stack.
        // Then we have to save the value of the old rsp on the new stack, and restore the value of gs:0.
        asm! (
            "swapgs",
            "xchg gs:0, rsp",
            "push rbp",
            "lea rbp, [rsp + 8]",
            "xchg gs:0, rbp",
            "swapgs",
            "push rcx",
            "push r11",
            "push r9",
            "push r8",
            "push r10",
            "push rdx",
            "push rsi",
            "push rdi",
            "push rax",
            "mov rdi, rax",
            "lea rsi, [rsp + 8]",
            "call {syscall_handler}",
            "pop rax",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop r10",
            "pop r8",
            "pop r9",
            "pop r11",
            "pop rcx",
            "xchg [rsp], rbp",
            "pop rsp",
            "sysretq",
            syscall_handler = sym syscall_handler,
            options(noreturn),
        );
    }
}

extern "C" fn syscall_handler(number: u16, arguments: &mut syscall_interface::RegisterValues) {
    // SAFETY: We are the syscall handler, and therefore know that there is no handle to the address space.
    let address_space = unsafe { UserAddressSpaceHandle::new() };

    let result =
        crate::syscall::handle_syscall(decode_syscall(number, *arguments).unwrap(), address_space);
    *arguments = encode_syscall_result(result);
}

static SYSCALL_STACK_POINTER: AtomicPtr<u8> = AtomicPtr::new(null_mut());

pub fn initialize(stack_pointer: *mut u8) {
    // The plan is to store a pointer to the stack pointer in the alternate GS register.
    // This way, the syscall handler can do a swapgs, read the stack pointer, and swapgs back.
    SYSCALL_STACK_POINTER.store(stack_pointer, atomic::Ordering::SeqCst);
    unsafe {
        write_msr(
            ALTERNATE_GS_BASE_MSR,
            &SYSCALL_STACK_POINTER as *const _ as u64,
        );
    }

    // To set up the syscall handler, we have to initialize three MSRs: STAR, LSTAR and SFMASK.
    // STAR holds the CS and SS segment selectors for kernel and user mode:
    const USER_SELECTORS_BASE: u16 = USER_DATA_SELECTOR - 8;
    const KERNEL_SELECTORS_BASE: u16 = 0x08;
    const STAR_VALUE: u64 =
        ((USER_SELECTORS_BASE as u64) << 48) | ((KERNEL_SELECTORS_BASE as u64) << 32);
    unsafe {
        write_msr(STAR_MSR, STAR_VALUE);
        write_msr(LSTAR_MSR, syscall_entrypoint as *const () as u64);
        write_msr(SFMASK_MSR, 0x200); // bit 9 masks the interrupt flag.
    }
}
