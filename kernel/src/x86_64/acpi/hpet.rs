//! High Precision Event Timer ([`HPET`]) table handling.
//!
//! [`HPET`]: https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/software-developers-hpet-spec-1-0a.pdf

use crate::{
    acpi::AcpiTableHandle,
    memory::{reinterpret_memory, Validateable},
};

#[repr(C, packed)]
pub struct HpetTableBody {
    hardware_revision: u8,
    counter_info: u8,
    pci_vendor_id: u16,
    address_space: u8,
    bit_width: u8,
    bit_offset: u8,
    access_width: u8,
    address: u64,
    hpet_number: u8,
    minimum_tick: u16,
    page_protection: u8,
}

impl Validateable for HpetTableBody {
    fn validate(&self) -> bool {
        true // TODO: Actually validate this
    }
}

const COUNTER_INFO_COMPARATOR_COUNT_MASK: u8 = 0b0001_1111;
const COUNTER_INFO_64_BIT: u8 = 0b0010_0000;
const COUNTER_INFO_LEGACY_REPLACEMENT: u8 = 0b1000_0000;

#[derive(Debug)]
pub struct HpetInfo {
    pub address: u64,
    pub comparator_count: u8,
    pub is_64_bit: bool,
    pub legacy_replacement: bool,
}

impl HpetInfo {
    pub fn new(table: &AcpiTableHandle) -> HpetInfo {
        assert_eq!(table.identifier(), b"HPET");
        let body = unsafe { reinterpret_memory::<HpetTableBody>(table.body()) }
            .expect("Invalid HPET table");
        let counter_info = body.counter_info;
        Self {
            address: body.address,
            comparator_count: (counter_info & COUNTER_INFO_COMPARATOR_COUNT_MASK) + 1,
            is_64_bit: counter_info & COUNTER_INFO_64_BIT != 0,
            legacy_replacement: counter_info & COUNTER_INFO_LEGACY_REPLACEMENT != 0,
        }
    }
}
