//! Multiple APIC Description Table (MADT) handling.
//! 
//! [`MADT`]: https://uefi.org/htmlspecs/ACPI_Spec_6_4_html/05_ACPI_Software_Programming_Model/ACPI_Software_Programming_Model.html#multiple-apic-description-table-madt

use alloc::vec::Vec;

use crate::{
    memory::{
        reinterpret_memory, DynamicallySized, DynamicallySizedItem, DynamicallySizedObjectIterator,
        Validateable,
    },
    println,
};
use core::mem::size_of;

use super::AcpiTableHandle;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct MadtEntryHeader {
    entry_type: u8,
    length: u8,
}

impl DynamicallySized for MadtEntryHeader {
    fn size(&self) -> usize {
        self.length as usize
    }
}

const MAX_ENTRY_SIZE: u8 = 128;

impl Validateable for MadtEntryHeader {
    fn validate(&self) -> bool {
        self.length >= 2 && self.length <= MAX_ENTRY_SIZE
    }
}

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

macro_rules! madt_entry {
    (
        struct $Name:ident ($id:expr) {
        $(
            $field:ident: $Type:ty
    ),*$(,)?
}
) => {
    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct $Name {
        header: MadtEntryHeader,
        $(
            $field: $Type,
        )*
    }

impl $Name {
    $(
        pub fn $field(&self) -> $Type {
            self.$field
        }
    )*
}

    impl Validateable for $Name {
        fn validate(&self) -> bool {
            let header = self.header;
            header.validate() && header.entry_type == $id && header.length >= size_of::<Self>() as u8
        }
    }
}
}

madt_entry! {
    struct LocalApicEntry(MADT_ENTRY_TYPE_LOCAL_APIC) {
        acpi_id: u8,
        apic_id: u8,
        flags: u32,
    }
}

madt_entry! {
    struct IoApicEntry(MADT_ENTRY_TYPE_IO_APIC) {
        io_apic_id: u8,
        reserved: u8,
        io_apic_address: u32,
        global_system_interrupt_base: u32,
    }
}

madt_entry! {
    struct InterruptSourceOverrideEntry(MADT_ENTRY_TYPE_INTERRUPT_SOURCE_OVERRIDE) {
        bus_source: u8,
        irq_source: u8,
        global_system_interrupt: u32,
        flags: u16,
    }
}

madt_entry! {
    struct NonMaskableInterruptSourceEntry(MADT_ENTRY_TYPE_NON_MASKABLE_INTERRUPT_SOURCE) {
        flags: u16,
        global_system_interrupt: u32,
    }
}

madt_entry! {
    struct LocalApicNmiEntry(MADT_ENTRY_TYPE_LOCAL_APIC_NMI) {
        acpi_processor_id: u8,
        flags: u16,
        local_apic_lint: u8,
    }
}

madt_entry! {
    struct LocalApicAddressOverrideEntry(MADT_ENTRY_TYPE_LOCAL_APIC_ADDRESS_OVERRIDE) {
        reserved: u16,
        local_apic_address: u64,
    }
}

// The GICC entries can be different sizes for different versions of ACPI.
// On some, it is 76, but on others it is 80.
// There may be more that I don't know of but I know for sure that these both exist.
madt_entry! {
    struct GenericInterruptControllerCpuInterfaceEntry76(MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_CPU_INTERFACE) {
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
madt_entry! {
    struct GenericInterruptControllerCpuInterfaceEntry80(MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_CPU_INTERFACE) {
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

impl From<GenericInterruptControllerCpuInterfaceEntry76>
    for GenericInterruptControllerCpuInterfaceEntry80
{
    fn from(entry: GenericInterruptControllerCpuInterfaceEntry76) -> Self {
        let GenericInterruptControllerCpuInterfaceEntry76 {
            header,
            reserved,
            cpu_interface_number,
            uid,
            flags,
            parking_protocol_version,
            performance_interrupt,
            parked_address,
            base_address,
            gicv_base_address,
            gich_base_address,
            vgic_maintenance_interrupt,
            gicr_base_address,
            multiprocessing_id,
        } = entry;
        GenericInterruptControllerCpuInterfaceEntry80 {
            header: MadtEntryHeader {
                entry_type: header.entry_type,
                length: 80,
            },
            reserved,
            cpu_interface_number,
            uid,
            flags,
            parking_protocol_version,
            performance_interrupt,
            parked_address,
            base_address,
            gicv_base_address,
            gich_base_address,
            vgic_maintenance_interrupt,
            gicr_base_address,
            multiprocessing_id,
            processor_efficiency: 0,
            reserved2: 0,
            statistical_profiling_interrupt: 0,
        }
    }
}

madt_entry! {
    struct GenericInterruptControllerDistributorEntry(MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_DISTRIBUTOR) {
        reserved: u16,
        gic_id: u32,
        base_address: u64,
        global_system_interrupt_base: u32, // Always 0
        version: u8,
        reserved2: [u8; 3],
    }
}

madt_entry! {
    struct GenericInterruptControllerMsiFrameEntry(MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_MSI_FRAME) {
        reserved: u16,
        msi_frame_id: u32,
        base_address: u64,
        flags: u32,
        spi_count: u16,
        spi_base: u16,
    }
}

madt_entry! {
    struct GenericInterruptControllerRedistributorEntry(MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_REDISTRIBUTOR) {
        reserved: u16,
        discovery_range_base_address: u64,
        discovery_range_length: u32,
    }
}

madt_entry! {
    struct GenericInterruptControllerTranslationServiceEntry(MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_TRANSLATION_SERVICE) {
        reserved: u16,
        translation_service_id: u32,
        base_address: u64,
        reserved2: u32,
    }
}

#[derive(Debug, Default)]
pub struct MadtInfo {
    pub local_apic_address: u32,
    pub flags: u32,

    // We essentially have a Vec of each of the types defined above.
    pub local_apic_entries: Vec<LocalApicEntry>,
    pub io_apic_entries: Vec<IoApicEntry>,
    pub interrupt_source_override_entries: Vec<InterruptSourceOverrideEntry>,
    pub non_maskable_interrupt_source_entries: Vec<NonMaskableInterruptSourceEntry>,
    pub local_apic_nmi_entries: Vec<LocalApicNmiEntry>,
    pub local_apic_address_override_entries: Vec<LocalApicAddressOverrideEntry>,
    pub generic_interrupt_controller_cpu_interface_entries:
        Vec<GenericInterruptControllerCpuInterfaceEntry80>, // The 76s are expanded to 80s.
    pub generic_interrupt_controller_distributor_entries:
        Vec<GenericInterruptControllerDistributorEntry>,
    pub generic_interrupt_controller_msi_frame_entries:
        Vec<GenericInterruptControllerMsiFrameEntry>,
    pub generic_interrupt_controller_redistributor_entries:
        Vec<GenericInterruptControllerRedistributorEntry>,
    pub generic_interrupt_controller_translation_service_entries:
        Vec<GenericInterruptControllerTranslationServiceEntry>,
}

impl MadtInfo {
    pub fn new(table: &AcpiTableHandle) -> Self {
        assert_eq!(table.identifier(), b"APIC");
        let body = table.body();
        // The first eight bytes of the body are the local APIC address and the flags.
        let local_apic_address = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let flags = u32::from_le_bytes(body[4..8].try_into().unwrap());

        let mut result = MadtInfo {
            local_apic_address,
            flags,
            ..Default::default()
        };

        let entries_data = &body[8..];
        for DynamicallySizedItem {
            value,
            value_memory,
        } in DynamicallySizedObjectIterator::<MadtEntryHeader>::new(entries_data)
        {
            macro_rules! add_entry {
                ($list_name:ident) => {
                    result.$list_name.push(*unsafe {
                        reinterpret_memory(value_memory).expect("Invalid MADT entry")
                    })
                };
            }

            match value.entry_type {
                MADT_ENTRY_TYPE_LOCAL_APIC => add_entry!(local_apic_entries),
                MADT_ENTRY_TYPE_IO_APIC => add_entry!(io_apic_entries),
                MADT_ENTRY_TYPE_INTERRUPT_SOURCE_OVERRIDE => {
                    add_entry!(interrupt_source_override_entries)
                }
                MADT_ENTRY_TYPE_NON_MASKABLE_INTERRUPT_SOURCE => {
                    add_entry!(non_maskable_interrupt_source_entries)
                }
                MADT_ENTRY_TYPE_LOCAL_APIC_NMI => add_entry!(local_apic_nmi_entries),
                MADT_ENTRY_TYPE_LOCAL_APIC_ADDRESS_OVERRIDE => {
                    add_entry!(local_apic_address_override_entries)
                }
                MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_CPU_INTERFACE => {
                    if value.length == 80 {
                        add_entry!(generic_interrupt_controller_cpu_interface_entries)
                    } else {
                        result
                            .generic_interrupt_controller_cpu_interface_entries
                            .push(GenericInterruptControllerCpuInterfaceEntry80::from(
                                *unsafe {
                                    reinterpret_memory::<
                                        GenericInterruptControllerCpuInterfaceEntry76,
                                    >(value_memory)
                                    .expect("Invalid MADT entry")
                                },
                            ))
                    }
                }
                MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_DISTRIBUTOR => {
                    add_entry!(generic_interrupt_controller_distributor_entries)
                }
                MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_MSI_FRAME => {
                    add_entry!(generic_interrupt_controller_msi_frame_entries)
                }
                MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_REDISTRIBUTOR => {
                    add_entry!(generic_interrupt_controller_redistributor_entries)
                }
                MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_TRANSLATION_SERVICE => {
                    add_entry!(generic_interrupt_controller_translation_service_entries)
                }
                _ => println!("Unknown MADT entry type: {}", value.entry_type),
            }
        }
        result
    }
}
