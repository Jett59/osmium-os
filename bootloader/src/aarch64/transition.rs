use core::arch::asm;

use super::{
    paging::PageTables,
    registers::{set_mair, set_tcr, set_ttbr1, MAIR, TCR},
};

pub fn enter_kernel(entrypoint: usize, stack_pointer: usize, page_tables: &PageTables) -> ! {
    set_ttbr1(page_tables.get_upper() as *const _ as u64);
    set_tcr(TCR::FOURTY_EIGHT_BIT_ADDRESSES | TCR::FOUR_K_PAGES);
    let mut mair = [MAIR::DEVICE; 8];
    mair[1] = MAIR::NORMAL_WRITE_BACK;
    set_mair(mair);
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
