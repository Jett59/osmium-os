use core::sync::atomic::{AtomicU32, Ordering};

use crate::{
    acpi::gtdt::TimerFlags,
    unsafe_impl::arch::asm::{get_cntfrq, get_cntvct, set_cntv_ctl, set_cntv_cval},
};

use super::{
    acpi::AcpiInfo,
    irq::{Priority, configure_interrupt, enable_interrupt},
};

static TIMER_INTERRUPT: AtomicU32 = AtomicU32::new(0);

pub fn initialize(acpi_info: &AcpiInfo) {
    let timer_frequency = get_cntfrq();
    set_cntv_ctl(false, true); // Enable the timer, unmask the interrupt
    set_cntv_cval(get_cntvct() + timer_frequency); // Set the timer compare value to go off in 1 second
    configure_interrupt(
        acpi_info.gtdt.timer_interrupt,
        acpi_info
            .gtdt
            .timer_flags
            .contains(TimerFlags::EDGE_TRIGGERED),
        Priority::High,
    );
    enable_interrupt(acpi_info.gtdt.timer_interrupt);

    TIMER_INTERRUPT.store(acpi_info.gtdt.timer_interrupt, Ordering::SeqCst);
}

pub fn get_timer_interrupt() -> u32 {
    TIMER_INTERRUPT.load(Ordering::SeqCst)
}
