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

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct SystemControl: u64 {
        const MMU = 1 << 0;
        const ALIGNMENT_CHECK = 1 << 1;
        const DATA_CACHE = 1 << 2;
        const KERNEL_STACK_ALIGNMENT_CHECK = 1 << 3;
        const USER_STACK_ALIGNMENT_CHECK = 1 << 4;
        const INSTRUCTION_CACHE = 1 << 12;
    }
}

#[inline(always)]
pub fn set_system_control_el1(system_control: SystemControl) {
    unsafe {
        asm!("msr sctlr_el1, {:x}", in(reg) system_control.bits(), options(nomem, nostack));
    }
}

#[inline(always)]
pub fn set_elr_el2(elr: u64) {
    unsafe {
        asm!("msr elr_el2, {:x}", in(reg) elr, options(nomem, nostack));
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct SavedProgramState: u64 {
        const EL_SPECIFIC_STACK = 1 << 0;
        const EL0 = 0b00 << 2;
        const EL1 = 0b01 << 2;
        const EL2 = 0b10 << 2;
        const EL3 = 0b11 << 2;

        const MASK_EXCEPTIONS = 0b1111 << 6;
    }
}

#[inline(always)]
pub fn set_saved_program_state_el2(saved_program_state: SavedProgramState) {
    unsafe {
        asm!("msr spsr_el2, {:x}", in(reg) saved_program_state.bits(), options(nomem, nostack));
    }
}

#[inline(always)]
pub fn set_stack_pointer_el1(stack_pointer: u64) {
    unsafe {
        asm!("msr sp_el1, {:x}", in(reg) stack_pointer, options(nomem, nostack));
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct HypervisorControl: u64 {
        const AARCH64 = 1 << 31;
        // That is the only one I care about just now.
    }
}

#[inline(always)]
pub fn set_hypervisor_control_el2(hypervisor_control: HypervisorControl) {
    unsafe {
        asm!("msr hcr_el2, {:x}", in(reg) hypervisor_control.bits(), options(nomem, nostack));
    }
}
