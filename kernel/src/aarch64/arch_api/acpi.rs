use core::mem::size_of;

use alloc::vec::Vec;
use common::beryllium::{AcpiTag, BootRequestTagType};

use crate::{
    acpi::{fadt::FadtInfo, madt::MadtInfo, AcpiTableHandle},
    arch::gtdt::GtdtInfo,
};

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

pub struct AcpiInfo {
    madt: MadtInfo,
    fadt: FadtInfo,
    gtdt: GtdtInfo,
}

pub fn handle_acpi_info(acpi_tables: Vec<AcpiTableHandle>) -> AcpiInfo {
    let mut madt = None;
    let mut fadt = None;
    let mut gtdt = None;
    for table in acpi_tables {
        match table.identifier() {
            b"APIC" => {
                madt = Some(MadtInfo::new(&table));
            }
            b"FACP" => {
                fadt = Some(FadtInfo::new(&table));
            }
            b"GTDT" => {
                gtdt = Some(GtdtInfo::new(&table));
            }
            _ => {}
        }
    }

    crate::println!("MADT: {:?}", madt);
    crate::println!("FADT: {:?}", fadt);
    crate::println!("GTDT: {:?}", gtdt);

    AcpiInfo {
        madt: madt.expect("MADT not found"),
        fadt: fadt.expect("FADT not found"),
        gtdt: gtdt.expect("GTDT not found"),
    }
}
