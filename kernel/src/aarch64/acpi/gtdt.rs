//! Generic Timer Description Table ([`GTDT`]) handling.
//!
//! [`GTDT`]: https://uefi.org/htmlspecs/ACPI_Spec_6_4_html/05_ACPI_Software_Programming_Model/ACPI_Software_Programming_Model.html#generic-timer-description-table-gtdt

use bitflags::bitflags;

use crate::{
    acpi::AcpiTableHandle,
    memory::{Endianness, FromBytes},
    memory_struct,
};

memory_struct! {
struct GtdtTableBody<'lifetime> {
    count_control_physical_address: u64,
    reserved: u32,
    _secure_el1_interrupt: u32,
    _secure_el1_flags: u32,
    el1_interrupt: u32,
    el1_flags: u32,
    _virtual_el1_interrupt: u32,
    _virtual_el1_flags: u32,
    _el2_interrupt: u32,
    _el2_flags: u32,
    counter_read_physical_address: u64,
    platform_timers_count: u32,
    platform_timers_offset: u32,
}
}

bitflags! {
    #[derive(Debug)]
    pub struct TimerFlags : u32 {
        const EDGE_TRIGGERED = 1 << 0;
        const ACTIVE_LOW = 1 << 1;
        const ALWAYS_ON = 1 << 2;
    }
}

#[derive(Debug)]
pub struct GtdtInfo {
    pub timer_interrupt: u32,
    pub timer_flags: TimerFlags,
}

impl GtdtInfo {
    pub fn new(table: &AcpiTableHandle) -> Self {
        assert_eq!(table.identifier(), b"GTDT");
        let body = GtdtTableBody::from_bytes(Endianness::Little, table.body())
            .expect("Invalid GTDT table body");
        Self {
            timer_interrupt: body.el1_interrupt(),
            timer_flags: TimerFlags::from_bits_retain(body.el1_flags()),
        }
    }
}
