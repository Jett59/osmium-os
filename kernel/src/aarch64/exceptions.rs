#[cfg_attr(test, allow(unused_imports))]
use core::{
    arch::{asm, global_asm},
    fmt::Debug,
};

use syscall_interface::{decode_syscall, encode_syscall_result};

use crate::{
    arch::registers::{get_cntfrq, get_cntvct, get_esr, set_cntv_cval},
    arch_api::{
        irq::{acknowledge_interrupt, end_of_interrupt},
        timer,
    },
    print,
    syscall::handle_syscall,
    user_memory::UserAddressSpaceHandle,
};

// The vector table itself is defined in assembly language, since it requires low-level manipulation of registers and system instructions.
#[cfg(not(test))]
global_asm!(include_str!("exceptions.s"));

#[cfg(not(test))]
extern "C" {
    static exception_vector_table: u8;
}

#[cfg(test)]
#[allow(non_upper_case_globals)]
static exception_vector_table: u8 = 0;

/// Loads vbar_el1 with the address of the exception vector table, allowing us to handle them properly.
pub fn load_exceptions() {
    unsafe {
        asm!("msr vbar_el1, {}", in(reg) &exception_vector_table);
    }
}

// Below are the string constants referenced in the assembly:
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

/// The registers saved by the assembly code, which are passed to the handlers.
#[derive(Debug)]
pub struct SavedRegisters {
    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
    x8: u64,
    x9: u64,
    x10: u64,
    x11: u64,
    x12: u64,
    x13: u64,
    x14: u64,
    x15: u64,
    x16: u64,
    x17: u64,
    x18: u64,
    x19: u64,
    x20: u64,
    x21: u64,
    x22: u64,
    x23: u64,
    x24: u64,
    x25: u64,
    x26: u64,
    x27: u64,
    x28: u64,
    x29: u64,
    x30: u64,
    sp: u64,
    elr: u64, // Exception Link Register, giving the address of the interrupted instruction.
    spsr: u64,
}

#[no_mangle]
pub extern "C" fn synchronous_vector(registers: &SavedRegisters) {
    panic!(
        "Synchronous exception at {:p}: {:x}\n{:x?}",
        registers.elr as *const (),
        get_esr(),
        registers
    );
}
#[no_mangle]
pub extern "C" fn irq_vector(registers: &SavedRegisters) {
    let Some(irq_info) = acknowledge_interrupt() else {
        return;
    };
    let interrupt_number = irq_info.interrupt_number;
    if interrupt_number == timer::get_timer_interrupt() {
        // Test code for the timer. Remove when we know it works.
        print!(".");
        // Set the timer to go off again in 1 second.
        set_cntv_cval(get_cntfrq() + get_cntvct());
    } else {
        panic!("IRQ {}\n{:x?}", irq_info.interrupt_number, registers);
    }
    end_of_interrupt(irq_info);
}
#[no_mangle]
pub extern "C" fn fiq_vector(registers: &SavedRegisters) {
    panic!("FIQ exception\n{:x?}", registers);
}
#[no_mangle]
pub extern "C" fn serror_vector(registers: &SavedRegisters) {
    panic!("SError exception\n{:x?}", registers);
}

const ESR_CLASS_SVC: u64 = 0b010101;

#[no_mangle]
pub extern "C" fn synchronous_vector_user(registers: &mut SavedRegisters) {
    let esr_value = get_esr();
    let esr_class = (esr_value >> 26) & 0b111111;
    if esr_class == ESR_CLASS_SVC {
        // It was a syscall instruction, with the syscall number in ESR[15:0]
        let syscall_number = (esr_value & 0xffff) as u16;

        // SAFETY: As an entrypoint of the kernel, this function knows that there is no handle to the address space.
        let address_space = unsafe { UserAddressSpaceHandle::new() };

        let result = handle_syscall(
            decode_syscall(
                syscall_number,
                syscall_interface::RegisterValues {
                    x0: registers.x0,
                    x1: registers.x1,
                    x2: registers.x2,
                    x3: registers.x3,
                    x4: registers.x4,
                    x5: registers.x5,
                },
            )
            .unwrap(),
            address_space,
        );
        let result_registers = encode_syscall_result(result);
        registers.x0 = result_registers.x0;
        registers.x1 = result_registers.x1;
        registers.x2 = result_registers.x2;
        registers.x3 = result_registers.x3;
        registers.x4 = result_registers.x4;
        registers.x5 = result_registers.x5;
    } else {
        panic!(
            "synchronous exception in user code at {:p}: {:x}\n{:x?}",
            registers.elr as *const (),
            get_esr(),
            registers
        );
    }
}

#[no_mangle]
pub extern "C" fn irq_vector_user(registers: &SavedRegisters) {
    irq_vector(registers);
}
#[no_mangle]
pub extern "C" fn fiq_vector_user(registers: &SavedRegisters) {
    panic!("FIQ exception in user code\n{:x?}", registers);
}
#[no_mangle]
pub extern "C" fn serror_vector_user(registers: &SavedRegisters) {
    panic!("SError exception in user code\n{:x?}", registers);
}
