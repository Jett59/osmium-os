use crate::{
    arch::{hpet::Hpet, local_apic},
    mmio::MmioMemoryHandle,
    print, println,
};

use super::acpi::AcpiInfo;

pub fn initialize(acpi_info: &AcpiInfo) {
    // The prefered timer is the APIC timer, which is specific to each CPU and has a very nice frequency.
    // The only drawback is that the frequency is specific to the CPU, so we have to synchronize it somehow with another timer.
    // We use the HPET for this purpose, since it is easy to discover through ACPI, has a good frequency and a very simple interface.

    // What we do specifically is:
    // 1. Set the APIC timer to count down from 0xffffffff (the highest possible value).
    // 2. (using the HPET) wait for a specific period of time (we'll go with 100ms).
    // 3. Read the APIC timer count and calculate the frequency based on the difference from 0xffffffff.

    // SAFETY: The ACPI tables are required to give us a good HPET.
    // Additionally, this function is only called once, and then before anything else has had a chance to use the HPET.
    let hpet = unsafe { Hpet::new(acpi_info.hpet.address as usize) };

    unsafe { local_apic::initialize_timer() };
    unsafe { local_apic::set_timer(0xffffffff) };

    unsafe { hpet.reset() };

    let end = unsafe { hpet.counter_value() + hpet.frequency() / 10 };

    while unsafe { hpet.counter_value() } < end {}

    let frequency = 10 * (0xffffffff - unsafe { local_apic::read_timer() });
    local_apic::set_timer_frequency(frequency);

    println!("APIC timer frequency: {}Hz", frequency);

    unsafe { local_apic::set_timer(frequency) };
}
