use crate::{
    mmio::MmioMemoryHandle,
    paging::PagePermissions,
    unsafe_impl::memory_token::{MemoryToken, PhysicalMmioToken},
};

pub struct Hpet {
    mmio_handle: MmioMemoryHandle,

    frequency: u64,
}

pub const HPET_MMIO_SIZE: usize = 1024;

const HPET_CAPABILITIES_OFFSET: usize = 0x00;
const HPET_GENERAL_CONFIGURATION_OFFSET: usize = 0x10;
const HPET_GENERAL_INTERRUPT_STATUS_OFFSET: usize = 0x20;
const HPET_MAIN_COUNTER_VALUE_OFFSET: usize = 0xF0;

impl Hpet {
    /// # Safety
    /// The physical address must point to a valid HPET MMIO region.
    pub unsafe fn new(physical_address: PhysicalMmioToken) -> Self {
        assert_eq!(physical_address.size(), HPET_MMIO_SIZE);
        let mmio_handle =
            MmioMemoryHandle::new(physical_address, PagePermissions::KERNEL_READ_WRITE);

        let frequency;
        unsafe {
            let capabilities = mmio_handle
                .at_offset::<u64>(HPET_CAPABILITIES_OFFSET)
                .read();
            let counter_clock_period = (capabilities >> 32) as u32; // In femtoseconds
            frequency = 1_000_000_000_000_000 / counter_clock_period as u64;

            // Here we disable the counter, as well as the legacy mode.
            let general_configuration = mmio_handle
                .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
                .read();
            mmio_handle
                .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
                .write(general_configuration & !(1 | 1 << 1));
        }

        Self {
            mmio_handle,
            frequency,
        }
    }

    pub fn frequency(&self) -> u64 {
        self.frequency
    }

    pub fn counter_value(&self) -> u64 {
        unsafe {
            self.mmio_handle
                .at_offset::<u64>(HPET_MAIN_COUNTER_VALUE_OFFSET)
                .read()
        }
    }

    pub fn enable(&self) {
        unsafe {
            let mut general_configuration = self
                .mmio_handle
                .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
                .read();
            general_configuration |= 1;
            self.mmio_handle
                .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
                .write(general_configuration);
        }
    }

    pub fn disable(&self) {
        unsafe {
            let mut general_configuration = self
                .mmio_handle
                .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
                .read();
            general_configuration &= !1;
            self.mmio_handle
                .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
                .write(general_configuration);
        }
    }

    /// Restart the counter at 0.
    pub fn reset(&self) {
        unsafe {
            self.disable();
            self.mmio_handle
                .at_offset::<u64>(HPET_MAIN_COUNTER_VALUE_OFFSET)
                .write(0);
            self.enable();
        }
    }
}
