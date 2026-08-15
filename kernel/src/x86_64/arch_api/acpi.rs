use alloc::vec::Vec;

use crate::{
    acpi::{AcpiTableHandle, madt::MadtInfo},
    arch::acpi::hpet::HpetInfo,
    unsafe_impl::init_cell::{InitCell, NoConcurrency},
};

static ROOT_TABLE_ADDRESS: InitCell<usize> = InitCell::new();

pub fn init(rsdt_address: usize, no_concurrency: &NoConcurrency) {
    ROOT_TABLE_ADDRESS.set(rsdt_address, no_concurrency);
}

pub fn get_root_table_address() -> Option<usize> {
    ROOT_TABLE_ADDRESS.get().copied()
}

pub struct AcpiInfo {
    pub madt: MadtInfo,
    pub hpet: HpetInfo,
}

pub fn handle_acpi_info(acpi_tables: Vec<AcpiTableHandle>) -> AcpiInfo {
    let mut madt = None;
    let mut hpet = None;
    for table in acpi_tables {
        match table.identifier() {
            b"APIC" => {
                madt = Some(MadtInfo::new(&table));
            }
            b"HPET" => {
                hpet = Some(HpetInfo::new(&table));
            }
            _ => {}
        }
    }

    crate::println!("MADT: {:?}", madt);
    crate::println!("HPET: {:?}", hpet);

    AcpiInfo {
        madt: madt.expect("MADT not found"),
        hpet: hpet.expect("HPET not found"),
    }
}
