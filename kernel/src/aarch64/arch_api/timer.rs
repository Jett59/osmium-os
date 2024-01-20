use crate::arch::registers::{get_cntfrq, get_cntvct};

use super::acpi::AcpiInfo;

pub fn initialize(acpi_info: &AcpiInfo) {
    let timer_frequency = get_cntfrq();
    loop {
        let start = get_cntvct();
        while get_cntvct() - start < timer_frequency {}
    }
}
