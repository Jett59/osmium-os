use crate::{
    arch::{
        asm::yield_instruction,
        registers::{get_cntfrq, get_cntvct},
    },
    print,
};

use super::acpi::AcpiInfo;

pub fn initialize(acpi_info: &AcpiInfo) {
    let timer_frequency = get_cntfrq();
    loop {
        let start = get_cntvct();
        while get_cntvct() - start < timer_frequency {
            yield_instruction(); // ?
        }
        print!(".");
    }
}
