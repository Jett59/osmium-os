pub mod arch_api;
mod hpet;
mod interrupts;
mod multiboot;

mod acpi {
    pub(in crate::arch) mod hpet;
}
