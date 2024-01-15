use core::mem::size_of;

use alloc::vec::Vec;
use common::beryllium::{AcpiTag, BootRequestTagType};

use crate::acpi::{fadt::FadtInfo, madt::MadtInfo, AcpiTableHandle};

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

pub fn handle_acpi_info(acpi_tables: Vec<AcpiTableHandle>) {
    let mut madt = None;
    let mut fadt = None;
    for table in acpi_tables {
        match table.identifier() {
            b"APIC" => {
                madt = Some(MadtInfo::new(&table));
            }
            b"FACP" => {
                fadt = Some(FadtInfo::new(&table));
            }
            _ => {}
        }
    }

    crate::println!("MADT: {:?}", madt);
    crate::println!("FADT: {:?}", fadt);
}
