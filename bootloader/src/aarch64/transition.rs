use core::arch::asm;

use super::{
    paging::PageTables,
    registers::{
        current_el, get_ttbr0_el2, mask_exceptions, set_hcr_el2, set_mair_el1, set_tcr_el1,
        set_ttbr0_el1, set_ttbr1_el1, ExceptionLevel, HCR, MAIR, TCR,
    },
};

pub fn enter_kernel(entrypoint: usize, stack_pointer: usize, page_tables: &PageTables) -> ! {
    mask_exceptions();
    set_ttbr1_el1(page_tables.get_upper() as *const _ as u64);
    set_tcr_el1(TCR::FOURTY_EIGHT_BIT_ADDRESSES | TCR::FOUR_K_PAGES);
    let mut mair = [MAIR::DEVICE; 8];
    mair[1] = MAIR::NORMAL_WRITE_BACK;
    set_mair_el1(mair);
    if current_el() == ExceptionLevel::EL2 {
        set_ttbr0_el1(get_ttbr0_el2());
        set_hcr_el2(HCR::SWIO | HCR::RW);
        unsafe {
            // We set the SCTLR_EL1 to 0x1 (enable MMU), SP_EL1 to the stack, ELR_El2 to the entrypoint, SPSR_EL2 to 0x3c5 (all exceptions masked, return to EL1 using SP_EL1), and then eret into the kernel.
            asm!("
            msr sp_el1, {stack_pointer}
            msr elr_el2, {entrypoint}
            mov x0, #0x3c5
            msr spsr_el2, x0
            mov x0, #0x1
            msr sctlr_el1, x0
            eret
            ",
                stack_pointer = in(reg) stack_pointer,
                entrypoint = in(reg) entrypoint,
                options(nomem, nostack)
            );
            unreachable!();
        }
    } else {
        unsafe {
            asm!(
                "
            mov sp, {stack_pointer}
            mov x0, {entrypoint}
            br x0
            ",
                stack_pointer = in(reg) stack_pointer,
                entrypoint = in(reg) entrypoint,
                options(nomem, nostack)
            );
            unreachable!();
        }
    }
}
