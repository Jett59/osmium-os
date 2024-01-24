use alloc::boxed::Box;

use crate::arch::gicv2::Gicv2;

use super::acpi::AcpiInfo;

pub(in crate::arch) struct InterruptInfo {
    pub interrupt_number: u32,

    pub acknowledge_register_value: u32,
}

pub(in crate::arch) trait GenericInterruptController {
    /// Acknowledge that an interrupt was received, getting the interrupt number at the same time.
    /// On GICv2, this is done by reading the `IAR` register in the CPU interface.
    fn acknowledge_interrupt(&mut self) -> Option<InterruptInfo>;

    /// Signal the end of interrupt handling, providing the interrupt number.
    fn end_of_interrupt(&mut self, interrupt_info: InterruptInfo);

    fn enable_interrupt(&mut self, interrupt_number: u32);
    fn disable_interrupt(&mut self, interrupt_number: u32);

    fn interrupt_is_usable(&self, interrupt_number: u32) -> bool;

    /// Enables the interrupts for this CPU.
    /// On GICV2, this means enabling the CPU interface.
    fn enable_interrupts_for_this_cpu(&mut self);
}

static mut GIC: Option<Box<dyn GenericInterruptController>> = None;

pub fn initialize(acpi_info: &AcpiInfo) {
    assert!(
        !acpi_info
            .madt
            .generic_interrupt_controller_distributor_entries
            .is_empty(),
        "No GICD entries found in MADT"
    );
    // Since there should only be one distributor, we just get it here.
    let gic_distributor = &acpi_info
        .madt
        .generic_interrupt_controller_distributor_entries[0];

    if gic_distributor.gic_version == 2 {
        assert!(
            !acpi_info
                .madt
                .generic_interrupt_controller_cpu_interface_entries
                .is_empty(),
            "No GICC entries found in MADT"
        );

        let distributor_address = gic_distributor.base_address;
        let cpu_interface_address = acpi_info
            .madt
            .generic_interrupt_controller_cpu_interface_entries[0]
            .base_address;

        // The GICV2 spec "strongly recommends" that the CPU interface address is common, so we
        // assert that here.
        for cpu_interface_entry in &acpi_info
            .madt
            .generic_interrupt_controller_cpu_interface_entries
        {
            assert_eq!(
                cpu_interface_entry.base_address, cpu_interface_address,
                "GICV2 CPU interface address is not common"
            );
        }

        // SAFETY: The provided addresses are from ACPI, so they are correct.
        // Also, there are no other GIC drivers running at this point, so there will be no conflicts.
        unsafe {
            GIC = Some(Box::new(Gicv2::new(
                distributor_address as usize,
                cpu_interface_address as usize,
            )));
        }
    } else {
        panic!("GICv{} not supported yet", gic_distributor.gic_version);
    }
}
