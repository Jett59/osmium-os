#[derive(Debug)]
pub enum ExceptionLevel {
    EL0,
    EL1,
    EL2,
    EL3,
}

#[inline(always)]
pub fn current_el() -> ExceptionLevel {
    let el: u64;
    unsafe {
        asm!("mrs {:x}, CurrentEL", out(reg) el, options(nomem, nostack));
    }
    match ((el >> 2) & 0b11) as u8 {
        0 => ExceptionLevel::EL0,
        1 => ExceptionLevel::EL1,
        2 => ExceptionLevel::EL2,
        3 => ExceptionLevel::EL3,
        _ => panic!("Unknown exception level"),
    }
}
