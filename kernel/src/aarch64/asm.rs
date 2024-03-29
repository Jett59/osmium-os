use core::arch::asm;

#[inline(always)]
pub fn isb() {
    unsafe { asm!("isb", options(nomem, nostack)) }
}

#[inline(always)]
pub fn dsb_ish() {
    unsafe { asm!("dsb ish", options(nomem, nostack)) }
}

#[inline(always)]
pub fn yield_instruction() {
    unsafe { asm!("yield", options(nomem, nostack)) }
}

pub fn enable_interrupts() {
    unsafe { asm!("msr daifclr, #15", options(nomem, nostack)) }
}
