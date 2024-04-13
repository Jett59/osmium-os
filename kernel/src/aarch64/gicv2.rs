use core::ops::Range;

use alloc::{boxed::Box, vec::Vec};

use crate::{
    arch_api::irq::{GenericInterruptController, InterruptInfo, Priority},
    mmio::MmioMemoryHandle,
    paging::PagePermissions,
};

pub struct Gicv2 {
    distributor_registers: MmioMemoryHandle,
    cpu_interface_registers: MmioMemoryHandle,

    cpu_interface_count: u32,
    available_interrupt_ranges: Box<[Range<u32>]>,
}

const DISTRIBUTOR_RANGE_LENGTH: usize = 0x1000;
const CPU_INTERFACE_RANGE_LENGTH: usize = 0x100;

const DISTRIBUTOR_CONTROL_OFFSET: usize = 0x000;
const DISTRIBUTOR_IDENTIFICATION_OFFSET: usize = 0x004;
const DISTRIBUTOR_SET_ENABLE_OFFSET: usize = 0x100;
const DISTRIBUTOR_CLEAR_ENABLE_OFFSET: usize = 0x180;
const DISTRIBUTOR_PRIORITY_OFFSET: usize = 0x400;
const DISTRIBUTOR_INTERRUPT_CONFIGURATION_OFFSET: usize = 0xC00;

const CPU_INTERFACE_CONTROL_OFFSET: usize = 0x00;
const CPU_INTERFACE_PRIORITY_MASK_OFFSET: usize = 0x04;
const CPU_INTERFACE_ACKNOWLEDGE_REGISTER: usize = 0x0C;
const CPU_INTERFACE_END_OF_INTERRUPT_REGISTER: usize = 0x10;

const LOW_PRIORITY: u8 = 0xd0;
const NORMAL_PRIORITY: u8 = 0xc0;
const HIGH_PRIORITY: u8 = 0xb0;

impl Gicv2 {
    /// # Safety
    /// There must be no other active drivers, and the provided addresses must point to valid GICs.
    pub unsafe fn new(distributor_address: usize, cpu_interface_address: usize) -> Self {
        let distributor_registers = MmioMemoryHandle::new(
            distributor_address,
            DISTRIBUTOR_RANGE_LENGTH,
            PagePermissions::KERNEL_READ_WRITE,
        );
        let cpu_interface_registers = MmioMemoryHandle::new(
            cpu_interface_address,
            CPU_INTERFACE_RANGE_LENGTH,
            PagePermissions::KERNEL_READ_WRITE,
        );

        // The GIC spec recommends that we disable the GIC distributor before doing any discovery.
        distributor_registers
            .at_offset::<u32>(DISTRIBUTOR_CONTROL_OFFSET)
            .write(0x0);

        // The identification register (or interrupt controller type register, according to the spec) has these useful fields:
        // - bits 4:0: 32(n+1) gives the number of interrupt lines supported by the GIC.
        // - bits 7:5: n+1 gives the number of CPU interfaces connected to the distributor.
        let identification_register = distributor_registers
            .at_offset::<u32>(DISTRIBUTOR_IDENTIFICATION_OFFSET)
            .read();
        let interrupt_line_count = 32 * ((identification_register & 0b11111) + 1);
        let cpu_interface_count = ((identification_register >> 5) & 0b111) + 1;

        // To discover which interrupt lines are usable, we have to do the following:
        // - Write 0xFFFFFFFF to the set-enable register.
        // - Read the set-enable register. Any bits set to 0 are unusable.
        // - Write 0xFFFFFFFF to the clear-enable register.
        // - Read the clear-enable register. Any bits set to 1 are unusable.
        // This is necessary since there may be interrupt lines which can't be enabled, or that can't be disabled. Either way we can't use them.
        let mut unusable_interrupt_lines = Vec::new();

        for enable_register_index in 0..(interrupt_line_count / 32) {
            distributor_registers
                .at_offset::<u32>(
                    DISTRIBUTOR_SET_ENABLE_OFFSET + (enable_register_index as usize * 4),
                )
                .write(0xFFFFFFFF);
            let set_enable_register_value = distributor_registers
                .at_offset::<u32>(
                    DISTRIBUTOR_SET_ENABLE_OFFSET + (enable_register_index as usize * 4),
                )
                .read();

            distributor_registers
                .at_offset::<u32>(
                    DISTRIBUTOR_CLEAR_ENABLE_OFFSET + (enable_register_index as usize * 4),
                )
                .write(0xFFFFFFFF);
            let clear_enable_register_value = distributor_registers
                .at_offset::<u32>(
                    DISTRIBUTOR_CLEAR_ENABLE_OFFSET + (enable_register_index as usize * 4),
                )
                .read();

            for bit_index in 0..32 {
                let interrupt_line_index = enable_register_index * 32 + bit_index;
                let set_enable_bit = (set_enable_register_value >> bit_index) & 0b1;
                let clear_enable_bit = (clear_enable_register_value >> bit_index) & 0b1;

                if set_enable_bit == 0 || clear_enable_bit == 1 {
                    unusable_interrupt_lines.push(interrupt_line_index);
                }
            }
        }

        // Now we just have to compose the ranges of usable interrupts from the list of unusable ones.
        // I think the easiest way would be to store the the last unusable interrupt + 1, then create a range from there to the next unusable one.
        // It helps here that the list of unusable interrupts is sorted.
        let mut available_interrupt_ranges = Vec::new();
        let mut next_range_start = 0;
        for unusable_interrupt_line in unusable_interrupt_lines {
            if unusable_interrupt_line > next_range_start {
                available_interrupt_ranges.push(next_range_start..unusable_interrupt_line);
            }
            next_range_start = unusable_interrupt_line + 1;
        }
        if next_range_start < interrupt_line_count {
            available_interrupt_ranges.push(next_range_start..interrupt_line_count);
        }

        // Finally, we re-enable the distributor.
        distributor_registers
            .at_offset::<u32>(DISTRIBUTOR_CONTROL_OFFSET)
            .write(0x1);

        crate::println!(
            "Available interrupt lines: {:?}",
            available_interrupt_ranges
        );

        Self {
            distributor_registers,
            cpu_interface_registers,

            cpu_interface_count,
            available_interrupt_ranges: available_interrupt_ranges.into_boxed_slice(),
        }
    }
}

impl GenericInterruptController for Gicv2 {
    fn acknowledge_interrupt(&mut self) -> Option<InterruptInfo> {
        // SAFETY: we were created with an address, which was required to be valid.
        let acknowledge_register_value = unsafe {
            self.cpu_interface_registers
                .at_offset::<u32>(CPU_INTERFACE_ACKNOWLEDGE_REGISTER)
                .read()
        };
        let interrupt_number = acknowledge_register_value & 0x3FF;
        if interrupt_number == 1023 {
            None
        } else {
            Some(InterruptInfo {
                acknowledge_register_value,
                interrupt_number,
            })
        }
    }

    fn end_of_interrupt(&mut self, interrupt_info: InterruptInfo) {
        // SAFETY: we were created with an address, which was required to be valid.
        unsafe {
            self.cpu_interface_registers
                .at_offset::<u32>(CPU_INTERFACE_END_OF_INTERRUPT_REGISTER)
                .write(interrupt_info.acknowledge_register_value);
        }
    }

    fn enable_interrupt(&mut self, interrupt_number: u32) {
        assert!(
            self.interrupt_is_usable(interrupt_number),
            "attempted to enable an interrupt that is not usable"
        );
        // SAFETY: we were created with an address, which was required to be valid.
        unsafe {
            self.distributor_registers
                .at_offset::<u32>(
                    DISTRIBUTOR_SET_ENABLE_OFFSET + ((interrupt_number / 32) as usize * 4),
                )
                .write(1 << (interrupt_number % 32));
        }
    }

    fn disable_interrupt(&mut self, interrupt_number: u32) {
        assert!(
            self.interrupt_is_usable(interrupt_number),
            "attempted to disable an interrupt that is not usable"
        );
        // SAFETY: we were created with an address, which was required to be valid.
        unsafe {
            self.distributor_registers
                .at_offset::<u32>(
                    DISTRIBUTOR_CLEAR_ENABLE_OFFSET + ((interrupt_number / 32) as usize * 4),
                )
                .write(1 << (interrupt_number % 32));
        }
    }

    fn configure_interrupt(
        &mut self,
        interrupt_number: u32,
        edge_triggered: bool,
        priority: Priority,
    ) {
        assert!(
            self.interrupt_is_usable(interrupt_number),
            "attempted to configure an interrupt that is not usable"
        );
        // SAFETY: we were created with an address, which was required to be valid.
        unsafe {
            self.distributor_registers
                .at_offset::<u8>(DISTRIBUTOR_PRIORITY_OFFSET + interrupt_number as usize)
                .write(match priority {
                    Priority::Low => LOW_PRIORITY,
                    Priority::Normal => NORMAL_PRIORITY,
                    Priority::High => HIGH_PRIORITY,
                });

            let interrupt_configuration_register_offset =
                DISTRIBUTOR_INTERRUPT_CONFIGURATION_OFFSET + ((interrupt_number / 16) as usize * 4);

            // The way this works is that there is a 2-bit field for each interrupt number.
            // The first bit is reserved (probably used in an earlier version of the spec).
            // The second bit is 1 if the interrupt is edge-triggered, and 0 if it is level-triggered.

            // Unfortunately we have to use a read-modify-write pattern here since we have to preserve the other bits.
            self.distributor_registers
                .at_offset::<u32>(interrupt_configuration_register_offset)
                .write(
                    self.distributor_registers
                        .at_offset::<u32>(interrupt_configuration_register_offset)
                        .read()
                        & !(0b11 << ((interrupt_number % 16) * 2))
                        | ((edge_triggered as u32) << ((interrupt_number % 16) * 2)),
                );
        }
    }

    fn interrupt_is_usable(&self, interrupt_number: u32) -> bool {
        for available_interrupt_range in self.available_interrupt_ranges.iter() {
            if available_interrupt_range.contains(&interrupt_number) {
                return true;
            }
        }
        false
    }

    fn enable_interrupts_for_this_cpu(&mut self) {
        // SAFETY: we were created with an address, which was required to be valid.
        unsafe {
            self.cpu_interface_registers
                .at_offset::<u32>(CPU_INTERFACE_CONTROL_OFFSET)
                .write(0x1);
            // Also set the priority mask to LOW_PRIORITY+0x10 (which should allow all low-priority and above interrupts).
            self.cpu_interface_registers
                .at_offset::<u32>(CPU_INTERFACE_PRIORITY_MASK_OFFSET)
                .write(LOW_PRIORITY as u32 + 0x10);
        }
    }
}
