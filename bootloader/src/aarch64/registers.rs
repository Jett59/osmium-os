use core::arch::asm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionLevel {
    EL0,
    EL1,
    EL2,
    EL3,
}

#[inline(always)]
pub fn current_el() -> ExceptionLevel {
    let el: u64;
    unsafe {
        asm!("mrs {:x}, CurrentEL", out(reg) el, options(nomem, nostack));
    }
    match ((el >> 2) & 0b11) {
        0 => ExceptionLevel::EL0,
        1 => ExceptionLevel::EL1,
        2 => ExceptionLevel::EL2,
        3 => ExceptionLevel::EL3,
        _ => panic!("Unknown exception level"),
    }
}

#[inline(always)]
pub unsafe fn switch_to_el1() {
    if current_el() == ExceptionLevel::EL1 {
        return;
    }
    if current_el() != ExceptionLevel::EL2 {
        panic!("Cannot switch to EL1 from EL{}", current_el() as u8);
    }
    asm!(
        "
        // 1. Setup the system registers for EL1...
        // (This is system-specific and is omitted here for simplicity)

        // 2. Set up the SCTLR_EL1 register.
        // This would typically involve setting various bits to enable caches, set endianness, etc.
        // The exact value depends on your system.
        mov x0, {sctlr_el1_val}
        msr SCTLR_EL1, x0

        // 3. Set the SP_EL1 register to the stack pointer for EL1.
        // This stack pointer must be correctly initialized and aligned.
        mov x0, {sp_el1_val}
        msr SP_EL1, x0

        // 4. Perform an exception return to switch to EL1.
        // The exact value of x0 depends on your system and should be calculated based on
        // the ARMv8 Architecture Reference Manual.
        mov x0, {elr_el1_val}
        msr ELR_EL1, x0
        eret
        ",
        sctlr_el1_val = in(reg) 0x00C50078u64,  // example SCTLR_EL1 value, replace with your own
        sp_el1_val = in(reg) 0x00008000u64,     // example SP_EL1 value, replace with your own
        elr_el1_val = in(reg) 0x00000000u64,    // example ELR_EL1 value, replace with your own
        options(nostack)
    );
}
