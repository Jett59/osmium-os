use crate::{arch::interrupts, memory, physical_memory_manager};

use super::{super::multiboot, paging};

#[cfg(not(test))]
extern "C" {
    // The physical end of the kernel.
    // Note that this is not a pointer, it is actually the first thing after the kernel (in physical addressing), and therefore uses the unit type.
    #[allow(improper_ctypes)]
    static KERNEL_PHYSICAL_END: ();
}

#[cfg(test)]
static KERNEL_PHYSICAL_END: () = (); // Mutable to make an unsafe block necessary.

#[allow(unused_unsafe)] // It isn't actually unused, but I think there is a bug in the compiler since removing it causes an error.
pub fn arch_init() {
    interrupts::init();

    multiboot::parse_multiboot_structures();
    // Unless we really want to have difficulties in the near future (possibly as soon as the very next function), we must tell people not to use the kernel's memory as a heap.]
    physical_memory_manager::mark_range_as_used(
        0,
        memory::align_address_up(
            unsafe { &KERNEL_PHYSICAL_END as *const () as usize },
            physical_memory_manager::BLOCK_SIZE,
        ),
    );
    paging::initialize_paging();

    // Trigger an exception
    unsafe {
        core::arch::asm!("mov rax, 0xdeadbeef", "int 0x80");
    }
}
