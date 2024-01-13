use core::arch::{asm, global_asm};

global_asm!(include_str!("exceptions.s"));

extern "C" {
    static exception_vector_table: u8;
}

pub fn load_exceptions() {
    unsafe {
        asm!("msr vbar_el1, {}", in(reg) &exception_vector_table);
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub static sp0_synch: &str = "sp0_synch";
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub static sp0_irq: &str = "sp0_irq";
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub static sp0_fiq: &str = "sp0_fiq";
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub static sp0_serror: &str = "sp0_serror";

#[no_mangle]
#[allow(improper_ctypes)]
pub static user32_synch: &str = "user32_synch";
#[no_mangle]
#[allow(improper_ctypes)]
pub static user32_irq: &str = "user32_irq";
#[no_mangle]
#[allow(improper_ctypes)]
pub static user32_fiq: &str = "user32_fiq";
#[no_mangle]
#[allow(improper_ctypes)]
pub static user32_serror: &str = "user32_serror";

#[no_mangle]
pub extern "C" fn invalid_vector(vector: *const &str) {
    // # Safety
    // This function is only called by the assembly code, which guarantees that the pointer is valid.
    unsafe {
        panic!("Invalid vector: {}", *vector);
    }
}

#[derive(Debug)]
pub struct SavedRegisters {
    x: [u64; 31],
    zero: u64,
}

#[no_mangle]
pub extern "C" fn synchronous_vector(registers: &SavedRegisters) {
    panic!("Synchronous exception\n{:?}", registers);
}
#[no_mangle]
pub extern "C" fn irq_vector(registers: &SavedRegisters) {
    panic!("IRQ exception\n{:?}", registers);
}
#[no_mangle]
pub extern "C" fn fiq_vector(registers: &SavedRegisters) {
    panic!("FIQ exception\n{:?}", registers);
}
#[no_mangle]
pub extern "C" fn serror_vector(registers: &SavedRegisters) {
    panic!("SError exception\n{:?}", registers);
}

#[no_mangle]
pub extern "C" fn synchronous_vector_user(registers: &SavedRegisters) {
    panic!("synchronous exception in user code\n{:?}", registers);
}
#[no_mangle]
pub extern "C" fn irq_vector_user(registers: &SavedRegisters) {
    panic!("IRQ exception in user code\n{:?}", registers);
}
#[no_mangle]
pub extern "C" fn fiq_vector_user(registers: &SavedRegisters) {
    panic!("FIQ exception in user code\n{:?}", registers);
}
#[no_mangle]
pub extern "C" fn serror_vector_user(registers: &SavedRegisters) {
    panic!("SError exception in user code\n{:?}", registers);
}
