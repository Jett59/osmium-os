use core::arch::asm;

use crate::arch::registers::{set_sctlr_el1, set_ttbr0_el1, SCTLR};

use super::{
    paging::{PageTableFlags, PageTables},
    registers::{
        current_el, mask_exceptions, set_hcr_el2, set_mair_el1, set_tcr_el1, set_ttbr1_el1,
        ExceptionLevel, HCR, MAIR, TCR,
    },
};

pub fn enter_kernel(entrypoint: usize, stack_pointer: usize, page_tables: &mut PageTables) -> ! {
    // We need to set up the recursive mapping for the kernel.
    // This is the only place we can do it since it is architecture-specific, but we need the page tables to be created first.
    let page_table_address = page_tables.upper() as *const _ as u64;
    page_tables.upper_mut()[0] = page_table_address
        | (PageTableFlags::VALID
            | PageTableFlags::NOT_BLOCK
            | PageTableFlags::NORMAL_MEMORY
            | PageTableFlags::ACCESS)
            .bits();

    unsafe {
        mask_exceptions();
        set_ttbr1_el1(page_tables.upper() as *const _ as u64);
        set_tcr_el1(
            TCR::FORTY_EIGHT_BIT_VIRTUAL_ADDRESSES
                | TCR::FOUR_K_PAGES
                | TCR::FORTY_EIGHT_BIT_PHYSICAL_ADDRESSES,
        );
        let mut mair = [MAIR::DEVICE; 8];
        mair[1] = MAIR::NORMAL_WRITE_BACK;
        set_mair_el1(mair);
        set_sctlr_el1(SCTLR::RESERVED | SCTLR::MMU | SCTLR::CACHE_ENABLE);
        if current_el() == ExceptionLevel::EL2 {
            set_hcr_el2(HCR::RW | HCR::SWIO);
            set_ttbr0_el1(0);
            // We set SP_EL1 to the stack, ELR_El2 to the entrypoint, SPSR_EL2 to 0x3c5 (all exceptions masked, return to EL1 using SP_EL1), and then eret into the kernel.
            asm!("
            msr sp_el1, {stack_pointer}
            msr elr_el2, {entrypoint}
            mov x0, #0x3c5
            msr spsr_el2, x0
            dsb sy
            isb
            eret
            ",
                stack_pointer = in(reg) stack_pointer,
                entrypoint = in(reg) entrypoint,
                options(nomem, nostack, noreturn)
            );
        } else {
            asm!(
                "
            mov sp, {stack_pointer}
            dsb sy
            isb
            br {entrypoint}
            ",
                stack_pointer = in(reg) stack_pointer,
                entrypoint = in(reg) entrypoint,
                options(nomem, nostack, noreturn)
            );
        }
    }
}
