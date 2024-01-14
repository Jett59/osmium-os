use core::mem::size_of;

use common::beryllium::{AcpiTag, BootRequestTagType};

#[link_section = ".beryllium"]
#[no_mangle]
pub static mut ACPI_TAG: AcpiTag = AcpiTag {
    tag_type: BootRequestTagType::Acpi,
    size: size_of::<AcpiTag>() as u16,
    flags: 0,
    rsdt: 0,
};

pub fn get_rsdt_address() -> Option<usize> {
    // # Safety
    // Only the bootloader touches this value, so we should be safe.
    // Additionally, this code only runs when the kernel is first loaded, so there are no threads to worry about.
    unsafe {
        if ACPI_TAG.rsdt == 0 {
            None
        } else {
            Some(ACPI_TAG.rsdt)
        }
    }
}
