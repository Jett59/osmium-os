//! Multiple APIC Description Table (MADT) handling.
//!
//! [`MADT`]: https://uefi.org/htmlspecs/ACPI_Spec_6_4_html/05_ACPI_Software_Programming_Model/ACPI_Software_Programming_Model.html#multiple-apic-description-table-madt

use alloc::vec::Vec;
use bitflags::bitflags;

use crate::{
    memory::{
        DynamicallySized, DynamicallySizedItem, DynamicallySizedObjectIterator, Endianness,
        FromBytes, ReservedMemory,
    },
    memory_struct, println,
};

use super::AcpiTableHandle;

memory_struct! {
    struct MadtEntryHeader<'lifetime> {
        entry_type: u8,
        length: u8,
    }
}

impl DynamicallySized for MadtEntryHeader<'_> {
    fn size(&self) -> usize {
        self.length() as usize
    }
}

const MAX_ENTRY_SIZE: u8 = 128;

// It doesn't really make sense to use an enum for the type, since we don't know all the possible values (since the spec may change).
const MADT_ENTRY_TYPE_LOCAL_APIC: u8 = 0;
const MADT_ENTRY_TYPE_IO_APIC: u8 = 1;
const MADT_ENTRY_TYPE_INTERRUPT_SOURCE_OVERRIDE: u8 = 2;
const MADT_ENTRY_TYPE_NON_MASKABLE_INTERRUPT_SOURCE: u8 = 3;
const MADT_ENTRY_TYPE_LOCAL_APIC_NMI: u8 = 4;
const MADT_ENTRY_TYPE_LOCAL_APIC_ADDRESS_OVERRIDE: u8 = 5;

const MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_CPU_INTERFACE: u8 = 0xb;
const MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_DISTRIBUTOR: u8 = 0xc;
const MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_MSI_FRAME: u8 = 0xd;
const MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_REDISTRIBUTOR: u8 = 0xe;
const MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_TRANSLATION_SERVICE: u8 = 0xf;

memory_struct! {
    struct LocalApicEntry<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        acpi_id: u8,
        apic_id: u8,
        flags: u32,
    }
}

memory_struct! {
    struct IoApicEntry<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        io_apic_id: u8,
        reserved: u8,
        io_apic_address: u32,
        global_system_interrupt_base: u32,
    }
}

memory_struct! {
    struct InterruptSourceOverrideEntry<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        bus_source: u8,
        irq_source: u8,
        global_system_interrupt: u32,
        flags: u16,
    }
}

memory_struct! {
    struct NonMaskableInterruptSourceEntry<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        flags: u16,
        global_system_interrupt: u32,
    }
}

memory_struct! {
    struct LocalApicNmiEntry<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        acpi_processor_id: u8,
        flags: u16,
        local_apic_lint: u8,
    }
}

memory_struct! {
    struct LocalApicAddressOverrideEntry<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        reserved: u16,
        local_apic_address: u64,
    }
}

// The GICC entries can be different sizes for different versions of ACPI.
// On some, it is 76, but on others it is 80.
// There may be more that I don't know of but I know for sure that these both exist.
memory_struct! {
    struct GenericInterruptControllerCpuInterfaceEntry76<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        reserved: u16,
        cpu_interface_number: u32,
        uid: u32,
        flags: u32,
        parking_protocol_version: u32,
        performance_interrupt: u32,
        parked_address: u64,
        base_address: u64,
        gicv_base_address: u64,
        gich_base_address: u64,
        vgic_maintenance_interrupt: u32,
        gicr_base_address: u64,
        multiprocessing_id: u64,
    }
}
memory_struct! {
    struct GenericInterruptControllerCpuInterfaceEntry80<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        reserved: u16,
        cpu_interface_number: u32,
        uid: u32,
        flags: u32,
        parking_protocol_version: u32,
        performance_interrupt: u32,
        parked_address: u64,
        base_address: u64,
        gicv_base_address: u64,
        gich_base_address: u64,
        vgic_maintenance_interrupt: u32,
        gicr_base_address: u64,
    multiprocessing_id: u64,
        processor_efficiency: u8,
        reserved2: u8,
        statistical_profiling_interrupt: u16,
    }
}

memory_struct! {
    struct GenericInterruptControllerDistributorEntry<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        reserved: u16,
        gic_id: u32,
        base_address: u64,
        global_system_interrupt_base: u32, // Always 0
        version: u8,
        reserved2: ReservedMemory<3>,
    }
}

memory_struct! {
    struct GenericInterruptControllerMsiFrameEntry<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        reserved: u16,
        msi_frame_id: u32,
        base_address: u64,
        flags: u32,
        spi_count: u16,
        spi_base: u16,
    }
}

memory_struct! {
    struct GenericInterruptControllerRedistributorEntry<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        reserved: u16,
        discovery_range_base_address: u64,
        discovery_range_length: u32,
    }
}

memory_struct! {
    struct GenericInterruptControllerTranslationServiceEntry<'lifetime> {
        madt_header: MadtEntryHeader<'lifetime>,
        reserved: u16,
        translation_service_id: u32,
        base_address: u64,
        reserved2: u32,
    }
}

#[derive(Debug)]
pub struct IoApicInfo {
    pub address: u32,
    pub global_system_interrupt_base: u32,
}

bitflags! {
    #[derive(Debug)]
    pub struct GeneralAPICInterruptFlags: u16 {
        const ACTIVE_HIGH = 0b01;
        const ACTIVE_LOW = 0b11;
        const EDGE_TRIGGERED = 0b01 << 2;
        const LEVEL_TRIGGERED = 0b11 << 2;
    }
}

#[derive(Debug)]
pub struct InterruptSourceOverrideInfo {
    pub bus_source: u8,
    pub irq_source: u8,
    pub global_system_interrupt: u32,
    pub flags: GeneralAPICInterruptFlags,
}

#[derive(Debug)]
pub struct LocalApicNmiInfo {
    pub flags: GeneralAPICInterruptFlags,
    pub local_apic_lint: u8,
}

#[derive(Debug)]
pub struct GenericInterruptControllerCpuInterfaceInfo {
    pub cpu_interface_number: u32,
    pub base_address: u64,
    pub mp_id_register: u64,
    pub efficiency_class: u8,
}

#[derive(Debug)]
pub struct GenericInterruptControllerDistributorInfo {
    pub base_address: u64,
    pub gic_version: u8,
}

#[derive(Debug)]
pub struct GenericInterruptControllerRedistributorInfo {
    pub discovery_range_base_address: u64,
    pub discovery_range_length: u32,
}

#[derive(Debug, Default)]
pub struct MadtInfo {
    pub local_interrupt_controller_address: u64,
    pub flags: u32,

    pub local_apic_ids: Vec<u8>,
    pub io_apic_entries: Vec<IoApicInfo>,
    pub interrupt_source_override_entries: Vec<InterruptSourceOverrideInfo>,
    pub local_apic_nmi_entries: Vec<LocalApicNmiInfo>,
    pub generic_interrupt_controller_cpu_interface_entries:
        Vec<GenericInterruptControllerCpuInterfaceInfo>,
    pub generic_interrupt_controller_distributor_entries:
        Vec<GenericInterruptControllerDistributorInfo>,
    pub generic_interrupt_controller_redistributor_entries:
        Vec<GenericInterruptControllerRedistributorInfo>,
}

impl MadtInfo {
    pub fn new(table: &AcpiTableHandle) -> Self {
        assert_eq!(table.identifier(), b"APIC");
        let body = table.body();
        // The first eight bytes of the body are the local APIC address and the flags.
        let local_interrupt_controller_address = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let flags = u32::from_le_bytes(body[4..8].try_into().unwrap());

        let mut result = MadtInfo {
            local_interrupt_controller_address: local_interrupt_controller_address as u64,
            flags,
            ..Default::default()
        };

        let entries_data = &body[8..];
        for DynamicallySizedItem {
            value,
            value_memory,
        } in
            DynamicallySizedObjectIterator::<MadtEntryHeader>::new(Endianness::Little, entries_data)
        {
            match value.entry_type() {
                MADT_ENTRY_TYPE_LOCAL_APIC => {
                    let entry = LocalApicEntry::from_bytes(Endianness::Little, value_memory)
                        .expect("Invalid MADT entry");
                    result.local_apic_ids.push(entry.apic_id());
                }
                MADT_ENTRY_TYPE_IO_APIC => {
                    let entry = IoApicEntry::from_bytes(Endianness::Little, value_memory)
                        .expect("Invalid MADT entry");
                    result.io_apic_entries.push(IoApicInfo {
                        address: entry.io_apic_address(),
                        global_system_interrupt_base: entry.global_system_interrupt_base(),
                    });
                }
                MADT_ENTRY_TYPE_INTERRUPT_SOURCE_OVERRIDE => {
                    let entry =
                        InterruptSourceOverrideEntry::from_bytes(Endianness::Little, value_memory)
                            .expect("Invalid MADT entry");
                    result
                        .interrupt_source_override_entries
                        .push(InterruptSourceOverrideInfo {
                            bus_source: entry.bus_source(),
                            irq_source: entry.irq_source(),
                            global_system_interrupt: entry.global_system_interrupt(),
                            flags: GeneralAPICInterruptFlags::from_bits_retain(entry.flags()),
                        });
                }
                MADT_ENTRY_TYPE_NON_MASKABLE_INTERRUPT_SOURCE => {
                    println!("Ignoring NMI source entry");
                }
                MADT_ENTRY_TYPE_LOCAL_APIC_NMI => {
                    let entry = LocalApicNmiEntry::from_bytes(Endianness::Little, value_memory)
                        .expect("Invalid MADT entry");
                    result.local_apic_nmi_entries.push(LocalApicNmiInfo {
                        flags: GeneralAPICInterruptFlags::from_bits_retain(entry.flags()),
                        local_apic_lint: entry.local_apic_lint(),
                    });
                }
                MADT_ENTRY_TYPE_LOCAL_APIC_ADDRESS_OVERRIDE => {
                    let entry =
                        LocalApicAddressOverrideEntry::from_bytes(Endianness::Little, value_memory)
                            .expect("Invalid MADT entry");
                    result.local_interrupt_controller_address = entry.local_apic_address();
                }
                MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_CPU_INTERFACE => {
                    if value.length() == 80 {
                        let entry = GenericInterruptControllerCpuInterfaceEntry80::from_bytes(
                            Endianness::Little,
                            value_memory,
                        )
                        .expect("Invalid MADT entry");
                        result
                            .generic_interrupt_controller_cpu_interface_entries
                            .push(GenericInterruptControllerCpuInterfaceInfo {
                                cpu_interface_number: entry.cpu_interface_number(),
                                base_address: entry.base_address(),
                                mp_id_register: entry.multiprocessing_id(),
                                efficiency_class: entry.processor_efficiency(),
                            });
                    } else {
                        let entry = GenericInterruptControllerCpuInterfaceEntry76::from_bytes(
                            Endianness::Little,
                            value_memory,
                        )
                        .expect("Invalid MADT entry");
                        result
                            .generic_interrupt_controller_cpu_interface_entries
                            .push(GenericInterruptControllerCpuInterfaceInfo {
                                cpu_interface_number: entry.cpu_interface_number(),
                                base_address: entry.base_address(),
                                mp_id_register: entry.multiprocessing_id(),
                                efficiency_class: 0,
                            });
                    }
                }
                MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_DISTRIBUTOR => {
                    let entry = GenericInterruptControllerDistributorEntry::from_bytes(
                        Endianness::Little,
                        value_memory,
                    )
                    .expect("Invalid MADT entry");
                    result
                        .generic_interrupt_controller_distributor_entries
                        .push(GenericInterruptControllerDistributorInfo {
                            base_address: entry.base_address(),
                            gic_version: entry.version(),
                        });
                }
                MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_MSI_FRAME => {
                    println!("Ignoring MSI frame entry");
                }
                MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_REDISTRIBUTOR => {
                    let entry = GenericInterruptControllerRedistributorEntry::from_bytes(
                        Endianness::Little,
                        value_memory,
                    )
                    .expect("Invalid MADT entry");
                    result
                        .generic_interrupt_controller_redistributor_entries
                        .push(GenericInterruptControllerRedistributorInfo {
                            discovery_range_base_address: entry.discovery_range_base_address(),
                            discovery_range_length: entry.discovery_range_length(),
                        });
                }
                MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_TRANSLATION_SERVICE => {
                    println!("Ignoring translation service entry");
                }
                _ => println!("Unknown MADT entry type: {}", value.entry_type()),
            }
        }
        result
    }
}
