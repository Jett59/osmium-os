use core::arch::asm;

#[inline(always)]
pub fn isb() {
    unsafe { asm!("isb", options(nomem, nostack)) }
}

#[inline(always)]
pub fn dsb_ish() {
    unsafe { asm!("dsb ish", options(nomem, nostack)) }
}
