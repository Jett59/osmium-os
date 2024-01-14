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
const MADT_ENTRY_TYPE_IO_SAPIC: u8 = 6;
const MADT_ENTRY_TYPE_LOCAL_SAPIC: u8 = 7;
const MADT_TYPE_INTERRUPT_SOURCES: u8 = 8;
const MADT_TYPE_LOCAL_X2APIC: u8 = 9;
const MADT_TYPE_LOCAL_X2APIC_NMI: u8 = 0xa;
const MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_CPU_INTERFACE: u8 = 0xb;
const MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_DISTRIBUTOR: u8 = 0xc;
const MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_MSI_FRAME: u8 = 0xd;
const MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_REDISTRIBUTOR: u8 = 0xe;
const MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_TRANSLATION_SERVICE: u8 = 0xf;

macro_rules! default_operator {
    ($left:expr, $default:tt|$given:tt, $right:expr) => {
        $left $given $right
    };
    ($left:expr, $default:tt, $right:expr) => {
        $left $default $right
    };
}

macro_rules! madt_entry {
    (
        $(#[size_comparison_operator($size_comparison_operator:tt)])?
        struct $Name:ident ($id:expr) {
        $(
            $field:ident: $Type:ty => $validation:expr
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
            #[allow(unused_variables)]
            let $Name {header, $($field,)*} = *self;
            header.validate() && header.entry_type == $id && default_operator!(header.length, ==$(|$size_comparison_operator)?, size_of::<Self>() as u8) && $(
                $validation
            )&&*
        }
    }
}
}

madt_entry! {
    struct LocalApicEntry(MADT_ENTRY_TYPE_LOCAL_APIC) {
        acpi_id: u8 => true,
        apic_id: u8 => true,
        flags: u32 => true,
    }
}

madt_entry! {
    struct IoApicEntry(MADT_ENTRY_TYPE_IO_APIC) {
        io_apic_id: u8 => true,
        reserved: u8 => true,
        io_apic_address: u32 => true,
        global_system_interrupt_base: u32 => true,
    }
}

madt_entry! {
    struct InterruptSourceOverrideEntry(MADT_ENTRY_TYPE_INTERRUPT_SOURCE_OVERRIDE) {
        bus_source: u8 => true,
        irq_source: u8 => true,
        global_system_interrupt: u32 => true,
        flags: u16 => true,
    }
}

madt_entry! {
    struct NonMaskableInterruptSourceEntry(MADT_ENTRY_TYPE_NON_MASKABLE_INTERRUPT_SOURCE) {
        flags: u16 => true,
        global_system_interrupt: u32 => true,
    }
}

madt_entry! {
    struct LocalApicNmiEntry(MADT_ENTRY_TYPE_LOCAL_APIC_NMI) {
        acpi_processor_id: u8 => true,
        flags: u16 => true,
        local_apic_lint: u8 => true,
    }
}

madt_entry! {
    struct LocalApicAddressOverrideEntry(MADT_ENTRY_TYPE_LOCAL_APIC_ADDRESS_OVERRIDE) {
        reserved: u16 => true,
        local_apic_address: u64 => true,
    }
}

madt_entry! {
    struct IoSapicEntry(MADT_ENTRY_TYPE_IO_SAPIC) {
        io_sapic_id: u8 => true,
        reserved: u8 => true,
        global_system_interrupt_base: u32 => true,
        io_sapic_address: u64 => true,
    }
}

madt_entry! {
    #[size_comparison_operator(>=)]
    struct LocalSapicEntry(MADT_ENTRY_TYPE_LOCAL_SAPIC) {
        acpi_processor_id: u8 => true,
        local_sapic_id: u8 => true,
        local_sapic_eid: u8 => true,
        reserved: [u8; 3] => true,
        flags: u32 => true,
        acpi_processor_uid_string: u8 => true,
    }
}

madt_entry! {
    struct InterruptSourceEntry(MADT_TYPE_INTERRUPT_SOURCES) {
        flags: u16 => true,
        interrupt_type: u8 => true,
        destination_processor_id: u8 => true,
        destination_processor_eid: u8 => true,
        sapic_vector: u8 => true,
        global_system_interrupt: u32 => true,
        platform_flags: u32 => true,
    }
}

madt_entry! {
    struct LocalX2ApicEntry(MADT_TYPE_LOCAL_X2APIC) {
        reserved: u16 => true,
        x2apic_id: u32 => true,
        flags: u32 => true,
        acpi_processor_uid: u32 => true,
    }
}

madt_entry! {
    struct LocalX2ApicNmiEntry(MADT_TYPE_LOCAL_X2APIC_NMI) {
        flags: u16 => true,
        acpi_processor_uid: u32 => true,
        local_x2apic_lint: u8 => true,
        reserved: [u8; 3] => true,
    }
}

// The GICC entries can be different sizes for different versions of ACPI.
// On some, it is 76, but on others it is 80.
// There may be more that I don't know of but I know for sure that these both exist.
madt_entry! {
    struct GenericInterruptControllerCpuInterfaceEntry76(MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_CPU_INTERFACE) {
        reserved: u16 => true,
        cpu_interface_number: u32 => true,
        uid: u32 => true,
        flags: u32 => true,
        parking_protocol_version: u32 => true,
        performance_interrupt: u32 => true,
        parked_address: u64 => true,
        base_address: u64 => true,
        gicv_base_address: u64 => true,
        gich_base_address: u64 => true,
        vgic_maintenance_interrupt: u32 => true,
        gicr_base_address: u64 => true,
        multiprocessing_id: u64 => true,
    }
}
madt_entry! {
    struct GenericInterruptControllerCpuInterfaceEntry80(MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_CPU_INTERFACE) {
        reserved: u16 => true,
        cpu_interface_number: u32 => true,
        uid: u32 => true,
        flags: u32 => true,
        parking_protocol_version: u32 => true,
        performance_interrupt: u32 => true,
        parked_address: u64 => true,
        base_address: u64 => true,
        gicv_base_address: u64 => true,
        gich_base_address: u64 => true,
        vgic_maintenance_interrupt: u32 => true,
        gicr_base_address: u64 => true,
        multiprocessing_id: u64 => true,
        processor_efficiency: u8 => true,
        reserved2: u8 => true,
        statistical_profiling_interrupt: u16 => true,
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
        reserved: u16 => true,
        gic_id: u32 => true,
        base_address: u64 => true,
        global_system_interrupt_base: u32 => true, // Always 0
        version: u8 => true,
        reserved2: [u8; 3] => true,
    }
}

madt_entry! {
    struct GenericInterruptControllerMsiFrameEntry(MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_MSI_FRAME) {
        reserved: u16 => true,
        msi_frame_id: u32 => true,
        base_address: u64 => true,
        flags: u32 => true,
        spi_count: u16 => true,
        spi_base: u16 => true,
    }
}

madt_entry! {
    struct GenericInterruptControllerRedistributorEntry(MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_REDISTRIBUTOR) {
        reserved: u16 => true,
        discovery_range_base_address: u64 => true,
        discovery_range_length: u32 => true,
    }
}

madt_entry! {
    struct GenericInterruptControllerTranslationServiceEntry(MADT_TYPE_GENERIC_INTERRUPT_CONTROLLER_TRANSLATION_SERVICE) {
        reserved: u16 => true,
        translation_service_id: u32 => true,
        base_address: u64 => true,
        reserved2: u32 => true,
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
    pub io_sapic_entries: Vec<IoSapicEntry>,
    pub local_sapic_entries: Vec<LocalSapicEntry>,
    pub interrupt_source_entries: Vec<InterruptSourceEntry>,
    pub local_x2apic_entries: Vec<LocalX2ApicEntry>,
    pub local_x2apic_nmi_entries: Vec<LocalX2ApicNmiEntry>,
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
                MADT_ENTRY_TYPE_IO_SAPIC => add_entry!(io_sapic_entries),
                MADT_ENTRY_TYPE_LOCAL_SAPIC => add_entry!(local_sapic_entries),
                MADT_TYPE_INTERRUPT_SOURCES => add_entry!(interrupt_source_entries),
                MADT_TYPE_LOCAL_X2APIC => add_entry!(local_x2apic_entries),
                MADT_TYPE_LOCAL_X2APIC_NMI => add_entry!(local_x2apic_nmi_entries),
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
