use core::arch::asm;

pub fn memory_barrier() {
    unsafe {
        asm!("dmb sy");
    }
}
