pub mod arch_api;
mod asm;
mod hpet;
mod interrupts;
mod local_apic;
mod multiboot;

mod acpi {
    pub(in crate::arch) mod hpet;
}
