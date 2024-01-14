use alloc::vec::Vec;

use crate::acpi::{madt::MadtInfo, AcpiTableHandle};

static mut ROOT_TABLE_ADDRESS: usize = 0;

pub(in crate::arch) fn init(rsdt_address: usize) {
    // # Safety
    // It is safe to assign to ROOT_TABLE_ADDRESS because this function is only called once, and then before threading is initialized.
    unsafe {
        ROOT_TABLE_ADDRESS = rsdt_address;
    }
}

pub fn get_root_table_address() -> Option<usize> {
    // # Safety
    // Se above for init.
    unsafe {
        if ROOT_TABLE_ADDRESS == 0 {
            None
        } else {
            Some(ROOT_TABLE_ADDRESS)
        }
    }
}

pub fn handle_acpi_info(acpi_tables: Vec<AcpiTableHandle>) {
    let mut madt = None;
    for table in acpi_tables {
        match table.identifier() {
            b"APIC" => {
                madt = Some(MadtInfo::new(&table));
            }
            _ => {}
        }
    }

    crate::println!("MADT: {:?}", madt);
}
