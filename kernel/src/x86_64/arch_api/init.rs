use crate::unsafe_impl::{
    arch::{interrupts, multiboot, syscall, task_state_segment},
    init_cell::NoConcurrency,
    paging::init::initialize_paging,
};

pub fn arch_init(no_concurrency: &NoConcurrency) {
    interrupts::init();
    multiboot::parse_multiboot_structures(no_concurrency);
    initialize_paging();

    task_state_segment::initialize();
    syscall::initialize();
}
