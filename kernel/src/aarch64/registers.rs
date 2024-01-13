use core::arch::asm;

pub fn get_esr() -> u64 {
    let mut esr: u64;
    unsafe {
        asm!("mrs {}, esr_el1", out(reg) esr, options(nomem, nostack));
    }
    esr
}
