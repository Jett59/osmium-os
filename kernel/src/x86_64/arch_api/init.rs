use crate::{memory, phyical_memory_manager};

use super::{super::multiboot, paging};

#[cfg(not(test))]
extern "C" {
    // The physical end of the kernel.
    // Note that this is not a pointer, it is actually the first thing after the kernel (in physical addressing), and therefore uses the unit type.
    #[allow(improper_ctypes)]
    static KERNEL_PHYSICAL_END: ();
}

#[cfg(test)]
static mut KERNEL_PHYSICAL_END: () = (); // Mutable to make an unsafe block necessary.

pub fn arch_init() {
    multiboot::parse_multiboot_structures();
    // Unless we really want to have difficulties in the near future (possibly as soon as the very next function), we must tell people not to use the kernel's memory as a heap.]
    phyical_memory_manager::mark_range_as_used(
        0,
        memory::align_address_up(
            unsafe { &KERNEL_PHYSICAL_END as *const () as usize },
            phyical_memory_manager::BLOCK_SIZE,
        ),
    );
    paging::initialize_paging();
}
