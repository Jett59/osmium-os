use core::ptr::addr_of;

use crate::{
    arch::{syscall, task_state_segment},
    unsafe_impl::{arch::interrupts, init_cell::NoConcurrency, paging::init::initialize_paging},
};

use super::super::multiboot;

#[cfg(not(test))]
unsafe extern "C" {

    #[allow(improper_ctypes)]
    static stack_end: ();
}

#[cfg(test)]
#[allow(non_upper_case_globals)]
static stack_end: () = ();

pub fn arch_init(no_concurrency: &NoConcurrency) {
    interrupts::init();
    multiboot::parse_multiboot_structures(no_concurrency);
    initialize_paging();

    task_state_segment::initialize(addr_of!(stack_end) as u64);
    syscall::initialize(addr_of!(stack_end) as *mut u8);
}
