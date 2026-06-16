use crate::arch::asm::{self, USER_CODE_SELECTOR, USER_DATA_SELECTOR};

/// Enter user mode at the specified address (ideally in user memory).
///
/// # Safety
/// This could be unsafe for all the same reasons why FFI is unsafe.
/// If entrypoint is invalid, or it does something nasty, it could be unsafe.
pub unsafe fn enter_user_mode(entrypoint: usize) -> ! {
    asm::iret(
        USER_DATA_SELECTOR as u64,
        0,
        0x200,
        USER_CODE_SELECTOR as u64,
        entrypoint as u64,
    );
}
