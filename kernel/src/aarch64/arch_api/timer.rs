use crate::arch::{
    gtdt::TimerFlags,
    registers::{get_cntfrq, get_cntvct, set_cntv_ctl, set_cntv_cval},
};

use super::{
    acpi::AcpiInfo,
    irq::{configure_interrupt, enable_interrupt},
};

pub fn initialize(acpi_info: &AcpiInfo) {
    let timer_frequency = get_cntfrq();
    set_cntv_ctl(0x1); // Enable the timer, unmask the interrupt
    set_cntv_cval(get_cntvct() + timer_frequency); // Set the timer compare value to go off in 1 second
    configure_interrupt(
        acpi_info.gtdt.timer_interrupt,
        acpi_info
            .gtdt
            .timer_flags
            .contains(TimerFlags::EDGE_TRIGGERED),
        0xf0,
    );
    enable_interrupt(acpi_info.gtdt.timer_interrupt);
}
