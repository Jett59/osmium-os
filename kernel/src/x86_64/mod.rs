pub mod arch_api;
mod multiboot;
mod syscall;
mod task_state_segment;

mod acpi {
    pub(in crate::arch) mod hpet;
}
