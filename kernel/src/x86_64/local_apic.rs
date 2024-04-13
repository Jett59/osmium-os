use core::sync::atomic::{AtomicU64, Ordering};

use bitflags::bitflags;

use crate::{mmio::MmioMemoryHandle, paging::PagePermissions};

use super::interrupts::{SPURIOUS_INTERRUPT_VECTOR, TIMER_INTERRUPT};

static mut APIC_HANDLE: Option<MmioMemoryHandle> = None;

const LOCAL_APIC_MEMORY_RANGE_SIZE: usize = 0x1000;

const LOCAL_APIC_ID_OFFSET: usize = 0x20;
const LOCAL_APIC_VERSION_OFFSET: usize = 0x30;
const LOCAL_APIC_TASK_PRIORITY_OFFSET: usize = 0x80;
const LOCAL_APIC_ARBITRATION_PRIORITY_OFFSET: usize = 0x90;
const LOCAL_APIC_PROCESSOR_PRIORITY_OFFSET: usize = 0xA0;
const LOCAL_APIC_EOI_OFFSET: usize = 0xB0;
const LOCAL_APIC_REMOTE_READ_OFFSET: usize = 0xC0;
const LOCAL_APIC_LOGICAL_DESTINATION_OFFSET: usize = 0xD0;
const LOCAL_APIC_DESTINATION_FORMAT_OFFSET: usize = 0xE0;
const LOCAL_APIC_SPURIOUS_INTERRUPT_VECTOR_OFFSET: usize = 0xF0;

const LOCAL_APIC_IN_SERVICE_BASE_OFFSET: usize = 0x100;
const LOCAL_APIC_TRIGGER_MODE_BASE_OFFSET: usize = 0x180;
const LOCAL_APIC_INTERRUPT_REQUEST_BASE_OFFSET: usize = 0x200;

const LOCAL_APIC_ERROR_STATUS_OFFSET: usize = 0x280;
const LOCAL_APIC_INTERRUPT_COMMAND_OFFSET: usize = 0x300;
const LOCAL_APIC_LVT_TIMER_OFFSET: usize = 0x320;
const LOCAL_APIC_LVT_THERMAL_SENSOR_OFFSET: usize = 0x330;
const LOCAL_APIC_LVT_PERFORMANCE_MONITORING_COUNTERS_OFFSET: usize = 0x340;
const LOCAL_APIC_LVT_LINT0_OFFSET: usize = 0x350;
const LOCAL_APIC_LVT_LINT1_OFFSET: usize = 0x360;
const LOCAL_APIC_LVT_ERROR_OFFSET: usize = 0x370;
const LOCAL_APIC_TIMER_INITIAL_COUNT_OFFSET: usize = 0x380;
const LOCAL_APIC_TIMER_CURRENT_COUNT_OFFSET: usize = 0x390;
const LOCAL_APIC_TIMER_DIVIDE_CONFIGURATION_OFFSET: usize = 0x3E0;

/// # Safety
/// The physical address must both point to a APIC, and also not be in use by another instance of the APIC driver or be mapped anywhere else.
/// Additionally, there will be massive confusion if the legacy PIC is not disabled by now, so callers must ensure that it is disabled.
pub unsafe fn initialize(address: usize) {
    APIC_HANDLE = Some(MmioMemoryHandle::new(
        address,
        LOCAL_APIC_MEMORY_RANGE_SIZE,
        PagePermissions::KERNEL_READ_WRITE,
    ));

    let Some(apic_handle) = APIC_HANDLE.as_mut() else {
        panic!("APIC handle not initialized");
    };

    // To enable the APIC, we have to set the spurious interrupt vector with bit 8 set to 1.
    apic_handle
        .at_offset::<u32>(LOCAL_APIC_SPURIOUS_INTERRUPT_VECTOR_OFFSET)
        .write(SPURIOUS_INTERRUPT_VECTOR as u32 | 0x100);
}

/// # Safety
/// The APIC must be initialized properly (see above).
pub unsafe fn end_of_interrupt() {
    let Some(apic_handle) = APIC_HANDLE.as_mut() else {
        panic!("APIC handle not initialized");
    };

    apic_handle.at_offset::<u32>(LOCAL_APIC_EOI_OFFSET).write(0);
}

bitflags! {
    pub struct LvtFlags: u32 {
        const TIMER_MODE_PERIODIC = 1 << 17;
        const MASKED = 1 << 16;
        const TRIGGER_MODE_LEVEL = 1 << 15;
        const INTERRUPT_ACTIVE = 1 << 14;
        const INTERRUPT_PENDING = 1 << 12;
        // const MESSAGE_TYPE_FIXED = 0b000 << 8; // The default
        const MESSAGE_TYPE_SMI = 0b010 << 8;
        const MESSAGE_TYPE_NMI = 0b100 << 8;
        const MESSAGE_TYPE_EXTINT = 0b111 << 8;
    }
}

/// # Safety
/// The APIC must be initialized properly (see above).
pub unsafe fn initialize_timer() {
    let Some(apic_handle) = APIC_HANDLE.as_mut() else {
        panic!("APIC handle not initialized");
    };

    // We set the timer to be one-shot, with an initial count of 0 and a divisor of 64.
    apic_handle
        .at_offset::<u32>(LOCAL_APIC_LVT_TIMER_OFFSET)
        .write(TIMER_INTERRUPT as u32);

    apic_handle
        .at_offset::<u32>(LOCAL_APIC_TIMER_INITIAL_COUNT_OFFSET)
        .write(0);

    apic_handle
        .at_offset::<u32>(LOCAL_APIC_TIMER_DIVIDE_CONFIGURATION_OFFSET)
        .write(0b1001);
}

static TIMER_FREQUENCY: AtomicU64 = AtomicU64::new(0);

pub fn set_timer_frequency(frequency: u64) {
    TIMER_FREQUENCY.store(frequency, Ordering::SeqCst);
}

pub fn get_timer_frequency() -> u64 {
    TIMER_FREQUENCY.load(Ordering::SeqCst)
}

/// Read the raw count from the timer.
/// Callers will generally want to convert this to some normal unit of time, which would generally involve multiplying by some value (e.g. 1000 for milliseconds) and dividing by the frequency.
///
/// # Safety
/// The APIC must be initialized properly (see above).
pub unsafe fn read_timer() -> u64 {
    let Some(apic_handle) = APIC_HANDLE.as_mut() else {
        panic!("APIC handle not initialized");
    };

    apic_handle
        .at_offset::<u32>(LOCAL_APIC_TIMER_CURRENT_COUNT_OFFSET)
        .read() as u64
}

/// Set the timer to fire after the given number of ticks.
///
/// # Safety
/// The APIC must be initialized properly (see above).
pub unsafe fn set_timer(ticks: u64) {
    let Some(apic_handle) = APIC_HANDLE.as_mut() else {
        panic!("APIC handle not initialized");
    };

    apic_handle
        .at_offset::<u32>(LOCAL_APIC_TIMER_INITIAL_COUNT_OFFSET)
        .write(ticks as u32);
}
