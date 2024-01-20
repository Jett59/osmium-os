use crate::{arch::hpet::Hpet, mmio::MmioMemoryHandle, print};

use super::acpi::AcpiInfo;

pub fn initialize(acpi_info: &AcpiInfo) {
    // SAFETY: The ACPI tables are required to give us a good HPET.
    // Additionally, this function is only called once, and then before anything else has had a chance to use the HPET.
    let hpet = unsafe { Hpet::new(acpi_info.hpet.address as usize) };
    unsafe { hpet.reset() };

    loop {
        let start = unsafe { hpet.counter_value() };
        while unsafe { hpet.counter_value() } < start + hpet.frequency() {}
        print!(".");
    }
}
