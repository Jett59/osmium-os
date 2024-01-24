pub mod arch_api;
mod asm;
mod exceptions;
mod gicv2;
mod registers;

#[path = "acpi/gtdt.rs"]
mod gtdt;
