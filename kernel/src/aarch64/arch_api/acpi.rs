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

pub fn get_root_table_address() -> Option<usize> {
    // # Safety
    // It's safe to access the tag structures since, although they are technically mutable, they only get changed by the bootloader.
    unsafe {
        if ACPI_TAG.rsdt == 0 {
            None
        } else {
            Some(ACPI_TAG.rsdt)
        }
    }
}
