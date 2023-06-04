use core::arch::asm;

use bitflags::bitflags;

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
    match (el >> 2) & 0b11 {
        0 => ExceptionLevel::EL0,
        1 => ExceptionLevel::EL1,
        2 => ExceptionLevel::EL2,
        3 => ExceptionLevel::EL3,
        _ => panic!("Unknown exception level"),
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct TCR: u64 {
        const FOURTY_EIGHT_BIT_ADDRESSES = 16 << 0 | 16 << 16;
        const FOUR_K_PAGES = 0 << 14 | 2 << 30;
        // TODO: Add the rest
    }
}

#[inline(always)]
pub fn set_tcr_el1(tcr: TCR) {
    unsafe {
        asm!("msr tcr_el1, {:x}", in(reg) tcr.bits(), options(nomem, nostack));
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct MAIR: u8 {
        const DEVICE = 0b0000;
        const NORMAL_WRITE_BACK = 0b1111;
    }
}

#[inline(always)]
pub fn set_mair_el1(mair: [MAIR; 8]) {
    let mut mair_el1: u64 = 0;
    for (i, mair) in mair.iter().enumerate() {
        mair_el1 |= (mair.bits() as u64) << (i * 8);
    }
    unsafe {
        asm!("msr mair_el1, {:x}", in(reg) mair_el1, options(nomem, nostack));
    }
}

#[inline(always)]
pub fn get_ttbr0_el2() -> u64 {
    let ttbr0: u64;
    unsafe {
        asm!("mrs {:x}, ttbr0_el2", out(reg) ttbr0, options(nomem, nostack));
    }
    ttbr0
}

#[inline(always)]
pub fn set_ttbr0_el1(ttbr0: u64) {
    unsafe {
        asm!("msr ttbr0_el1, {:x}", in(reg) ttbr0, options(nomem, nostack));
    }
}

#[inline(always)]
pub fn set_ttbr1_el1(ttbr1: u64) {
    unsafe {
        asm!("msr ttbr1_el1, {:x}", in(reg) ttbr1, options(nomem, nostack));
    }
}

#[inline(always)]
pub fn mask_exceptions() {
    unsafe {
        asm!("msr daifset, #1", options(nomem, nostack));
        asm!("msr daifset, #2", options(nomem, nostack));
        asm!("msr daifset, #4", options(nomem, nostack));
        asm!("msr daifset, #8", options(nomem, nostack));
    }
}
