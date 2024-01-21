use crate::arch::{
    asm::{enable_interrupts, io_wait, write_port8},
    local_apic,
};

use super::acpi::AcpiInfo;

pub fn initialize(acpi_info: &AcpiInfo) {
    // SAFETY: The MADT is required to contain the correct address for the local APIC, and this is the first place it is ever used.
    unsafe { local_apic::initialize(acpi_info.madt.local_interrupt_controller_address as usize) };

    if acpi_info.madt.flags & 0b1 != 0 {
        // Legacy PIC present
        unsafe {
            // We must disable the legacy PIC so that we don't get interrupts through it in addition to the IO APICs.
            // Unfortunately, there is no official way to do this.
            // The generally accepted method is to map it to some reserved range of interrupts, and then mask all interrupts on the primary PIC (including the cascade line to the secondary PIC).
            write_port8(0x20, 0x11); // Initialize primary PIC
            io_wait();
            write_port8(0x21, 0xf8); // Primary PIC starts at interrupt 0xf8, so that the spurious interrupt is 0xff
            io_wait();
            write_port8(0x20, 0x01); // Enable primary PIC
            io_wait();
            write_port8(0x21, 0x04); // Enable cascade line to secondary PIC
            io_wait();
            write_port8(0x21, 0x01); // Put it in 8086 mode (whatever that does)

            // Mask all interrupts on the primary PIC (including the cascade line to the secondary PIC)
            write_port8(0x21, 0xff);
        }
    }

    // TODO: Initialize IO APICs.

    // SAFETY: This is called from main, which doesn't expect interrupts to be disabled.
    unsafe { enable_interrupts() };
}
