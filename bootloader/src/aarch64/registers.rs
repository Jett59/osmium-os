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
        #[allow(clippy::identity_op)]
        const FORTY_EIGHT_BIT_VIRTUAL_ADDRESSES = 16 << 0 | 16 << 16;
        #[allow(clippy::identity_op)]
        const FOUR_K_PAGES = 0 << 14 | 2 << 30;
        const FORTY_EIGHT_BIT_PHYSICAL_ADDRESSES = 0b101 << 32;
        // TODO: Add the rest
    }
}

#[inline(always)]
pub unsafe fn set_tcr_el1(tcr: TCR) {
    asm!("msr tcr_el1, {:x}", in(reg) tcr.bits(), options(nomem, nostack));
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct MAIR: u8 {
        const DEVICE = 0b00000000;
        const NORMAL_WRITE_BACK = 0b11111111;
    }
}

#[inline(always)]
pub unsafe fn set_mair_el1(mair: [MAIR; 8]) {
    let mut mair_el1: u64 = 0;
    for (i, mair) in mair.iter().enumerate() {
        mair_el1 |= (mair.bits() as u64) << (i * 8);
    }
    asm!("msr mair_el1, {:x}", in(reg) mair_el1, options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn set_ttbr1_el1(ttbr1: u64) {
    asm!("msr ttbr1_el1, {:x}", in(reg) ttbr1, options(nomem, nostack));
}

bitflags! {
    pub struct HCR: u64 {
        const RW = 1 << 31;
        const SWIO = 1 << 1;
    }
}

#[inline(always)]
pub unsafe fn set_hcr_el2(hcr: HCR) {
    asm!("msr hcr_el2, {:x}", in(reg) hcr.bits(), options(nomem, nostack));
}

#[inline(always)]
pub unsafe fn mask_exceptions() {
    asm!("msr daifset, #1", options(nomem, nostack));
    asm!("msr daifset, #2", options(nomem, nostack));
    asm!("msr daifset, #4", options(nomem, nostack));
    asm!("msr daifset, #8", options(nomem, nostack));
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct SCTLR: u64 {
        const MMU = 1 << 0;
        const RESERVED = (1 << 29) | (1 << 28) | (1 << 23) | (1 << 22) | (1 << 20) | (1 << 11) | (1 << 8) | (1 << 7);
    }
}

pub unsafe fn set_sctlr_el1(sctlr_el1: SCTLR) {
    asm!("msr sctlr_el1, {:x}", in(reg) sctlr_el1.bits(), options(nomem, nostack));
}
