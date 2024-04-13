use crate::arch::asm;

/// Enter user mode at the specified address (ideally in user memory).
///
/// # Safety
/// This could be unsafe for all the same reasons why FFI is unsafe.
/// If entrypoint is invalid, or it does something nasty, it could be unsafe.
pub unsafe fn enter_user_mode(entrypoint: usize) -> ! {
    asm::iret(0x23, 0, 0x200, 0x1b, entrypoint as u64);
}
