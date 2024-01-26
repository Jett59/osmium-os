use crate::mmio::MmioMemoryHandle;

pub struct Hpet {
    mmio_handle: MmioMemoryHandle,

    frequency: u64,
}

const HPET_MMIO_SIZE: usize = 1024;

const HPET_CAPABILITIES_OFFSET: usize = 0x00;
const HPET_GENERAL_CONFIGURATION_OFFSET: usize = 0x10;
const HPET_GENERAL_INTERRUPT_STATUS_OFFSET: usize = 0x20;
const HPET_MAIN_COUNTER_VALUE_OFFSET: usize = 0xF0;

impl Hpet {
    /// # Safety
    /// The physical address must both point to a HPET, and also not be in use by another instance of the HPET driver or anything else.
    pub unsafe fn new(physical_address: usize) -> Self {
        let mmio_handle = MmioMemoryHandle::new(physical_address, HPET_MMIO_SIZE);

        let capabilities = mmio_handle
            .at_offset::<u64>(HPET_CAPABILITIES_OFFSET)
            .read();
        let counter_clock_period = (capabilities >> 32) as u32; // In femtoseconds
        let frequency = 1_000_000_000_000_000 / counter_clock_period as u64;

        // Here we disable the counter, as well as the legacy mode.
        let general_configuration = mmio_handle
            .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
            .read();
        mmio_handle
            .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
            .write(general_configuration & !(1 | 1 << 1));

        Self {
            mmio_handle,
            frequency,
        }
    }

    pub fn frequency(&self) -> u64 {
        self.frequency
    }

    pub unsafe fn counter_value(&self) -> u64 {
        self.mmio_handle
            .at_offset::<u64>(HPET_MAIN_COUNTER_VALUE_OFFSET)
            .read()
    }

    pub unsafe fn enable(&self) {
        let mut general_configuration = self
            .mmio_handle
            .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
            .read();
        general_configuration |= 1;
        self.mmio_handle
            .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
            .write(general_configuration);
    }

    pub unsafe fn disable(&self) {
        let mut general_configuration = self
            .mmio_handle
            .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
            .read();
        general_configuration &= !1;
        self.mmio_handle
            .at_offset::<u64>(HPET_GENERAL_CONFIGURATION_OFFSET)
            .write(general_configuration);
    }

    /// Restart the counter at 0.
    pub unsafe fn reset(&self) {
        self.disable();
        self.mmio_handle
            .at_offset::<u64>(HPET_MAIN_COUNTER_VALUE_OFFSET)
            .write(0);
        self.enable();
    }
}
