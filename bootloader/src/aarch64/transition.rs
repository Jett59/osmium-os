use core::arch::asm;

use super::{
    paging::PageTables,
    registers::{
        current_el, get_ttbr0_el2, set_elr_el2, set_hypervisor_control_el2, set_mair_el1,
        set_saved_program_state_el2, set_stack_pointer_el1, set_system_control_el1, set_tcr_el1,
        set_ttbr0_el1, set_ttbr1_el1, ExceptionLevel, HypervisorControl, SavedProgramState,
        SystemControl, MAIR, TCR,
    },
};

pub fn enter_kernel(entrypoint: usize, stack_pointer: usize, page_tables: &PageTables) -> ! {
    set_ttbr1_el1(page_tables.get_upper() as *const _ as u64);
    set_tcr_el1(TCR::FOURTY_EIGHT_BIT_ADDRESSES | TCR::FOUR_K_PAGES);
    let mut mair = [MAIR::DEVICE; 8];
    mair[1] = MAIR::NORMAL_WRITE_BACK;
    set_mair_el1(mair);
    if current_el() == ExceptionLevel::EL2 {
        jump_from_el2(entrypoint, stack_pointer);
    } else if current_el() == ExceptionLevel::EL1 {
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
    } else {
        panic!("Not running at EL1 or EL2");
    }
}

pub fn jump_from_el2(entrypoint: usize, stack_pointer: usize) -> ! {
    set_ttbr0_el1(get_ttbr0_el2());
    set_system_control_el1(
        SystemControl::MMU | SystemControl::DATA_CACHE | SystemControl::INSTRUCTION_CACHE,
    );
    set_hypervisor_control_el2(HypervisorControl::AARCH64);
    set_saved_program_state_el2(
        SavedProgramState::EL_SPECIFIC_STACK
            | SavedProgramState::EL1
            | SavedProgramState::MASK_EXCEPTIONS,
    );
    set_elr_el2(&doit as *const _ as u64);
    set_stack_pointer_el1(stack_pointer as u64);
    unsafe {
        crate::println!("Transitioning exception levels");
        asm!(
            "
        mov x0, sp
        msr sp_el1, x0
        ",
            options(nomem, nostack)
        );
        asm!("eret", options(nomem, nostack));
        unreachable!();
    }
}

fn doit() {
    crate::println!("Transitioned to {:?}", current_el());
    loop {}
}
