use bitflags::bitflags;

use crate::memory::{reinterpret_memory, Validateable};

use super::AcpiTableHandle;

#[repr(C, packed)]
pub struct FadtTableBody {
    // TODO: fill out the rest of this structure when we need it (which may well be never, since we are only dealing with early initialization code).
    unknown: [u8; 73],
    x86_boot_flags: u16,
    reserved: u8,
    fixed_flags: u32,
    _reset_register: [u8; 12],
    _reset_value: u8,
    arm_boot_flags: u16,
}

impl Validateable for FadtTableBody {
    fn validate(&self) -> bool {
        true // TODO: actually validate this structure
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct X86BootFlags: u16 {
        const LEGACY_DEVICES = 1 << 0;
        const PS2_KEYBOARD = 1 << 1;
        const VGA_NOT_PRESENT = 1 << 2;
        const MSI_NOT_AVAILABLE = 1 << 3;
        const ASPM_DISABLE = 1 << 4;
        const CMOS_RTC_NOT_PRESENT = 1 << 5;
    }

    #[derive(Debug)]
    pub struct FixedFlags: u32 {
        const WBINVD_WORKS = 1 << 0;
        const WBINVD_FLUSH = 1 << 1;
        const C1_SUPPORTED = 1 << 2;
        const C2_MULTIPROCESSORS = 1 << 3;
        const POWER_BUTTON_IS_CONTROL_METHOD = 1 << 4;
        const SLEEP_BUTTON_IS_CONTROL_METHOD = 1 << 5;
        const RTC_WAKE_IS_FIXED = 1 << 6;
        const RTC_WAKE_FROM_S4 = 1 << 7;
        const TIMER_VALUE_32BIT = 1 << 8;
        const CAN_BE_DOCKED = 1 << 9;
        const RESET_REG_SUPPORTED = 1 << 10;
        const SEALED_CASE = 1 << 11;
        const HEADLESS = 1 << 12;
        const REQUIRES_INSTRUCTION_AFTER_SLEEP_TYPE = 1 << 13;
        const PCIE_WAKE = 1 << 14;
        const USE_PLATFORM_CLOCK = 1 << 15;
        const RTC_STATUS_VALID_FROM_S4 = 1 << 16;
        const REMOTE_POWER_ON_CAPABLE = 1 << 17;
        const FORCE_APIC_CLUSTER_MODEL = 1 << 18;
        const FORCE_APIC_PHYSICAL_DESTINATION_MODE = 1 << 19;
        const HW_REDUCED_ACPI = 1 << 20;
        const S3_USELESS = 1 << 21;
    }

    #[derive(Debug)]
    pub struct ArmBootFlags: u16 {
        const SUPPORTS_PSCI = 1 << 0;
        const MUST_USE_HVC = 1 << 1;
    }
}

#[derive(Debug)]
pub struct FadtInfo {
    pub x86_boot_flags: X86BootFlags,
    pub fixed_flags: FixedFlags,
    pub arm_boot_flags: ArmBootFlags,
}

impl FadtInfo {
    pub fn new(fadt_table: &AcpiTableHandle) -> Self {
        assert_eq!(fadt_table.identifier(), b"FACP");
        let fadt_table_body = unsafe { reinterpret_memory::<FadtTableBody>(fadt_table.body()) }
            .expect("FADT table is invalid");

        Self {
            x86_boot_flags: X86BootFlags::from_bits_retain(fadt_table_body.x86_boot_flags),
            fixed_flags: FixedFlags::from_bits_retain(fadt_table_body.fixed_flags),
            arm_boot_flags: ArmBootFlags::from_bits_retain(fadt_table_body.arm_boot_flags),
        }
    }
}
