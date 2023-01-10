use crate::{memory, pmm, x86_64::multiboot};

use super::paging;

extern "C" {
    // The physical end of the kernel.
    // Note that this is not a pointer, it is actually the first thing after the kernel (in physical addressing), and therefore uses the unit type.
    #[allow(improper_ctypes)]
    static KERNEL_PHYSICAL_END: ();
}

pub fn arch_init() {
    multiboot::parse_multiboot_structures();
    // Unless we really want to have difficulties in the near future (possibly as soon as the very next function), we must tell people not to use the kernel's memory as a heap.]
    pmm::mark_range_as_used(
        0,
        memory::align_address_up(
            unsafe { &KERNEL_PHYSICAL_END as *const () as usize },
            pmm::BLOCK_SIZE,
        ),
    );
    paging::initialize_paging();
}
