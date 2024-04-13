use core::{mem::size_of, ptr::addr_of};

use bitflags::bitflags;

use super::asm::load_task_state_segment;

bitflags! {
    #[repr(C)]
    struct TaskStateSegmentDescriptorFlags: u8 {
        const TSS_TYPE = 0b00001001;
        const NOT_SYSTEM_DESCRIPTOR = 0b00010000;
        const USER_MODE = 0b01100000;
        const VALID = 0b10000000;
    }
}

#[repr(C, packed)]
struct TaskStateSegmentDescriptor {
    limit_low: u16,
    offset_low: u16,
    offset_low_middle: u8,
    flags: TaskStateSegmentDescriptorFlags,
    limit_high_and_additional_flags: u8,
    offset_high_middle: u8,
    offset_high: u32,
    zero: u32,
}

const DEFAULT_ADDITIONAL_FLAGS: u8 = 0b00000000;

#[repr(C, packed)]
struct TaskStateSegment {
    _reserved: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    _reserved2: u64,
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    _reserved3: u64,
    _reserved4: u16,
    iomap_base: u16,
}

static mut TASK_STATE_SEGMENT: TaskStateSegment = TaskStateSegment {
    _reserved: 0,
    rsp0: 0,
    rsp1: 0,
    rsp2: 0,
    _reserved2: 0,
    ist1: 0,
    ist2: 0,
    ist3: 0,
    ist4: 0,
    ist5: 0,
    ist6: 0,
    ist7: 0,
    _reserved3: 0,
    _reserved4: 0,
    iomap_base: 0,
};

extern "C" {
    static mut task_state_segment_descriptor: TaskStateSegmentDescriptor;
}

pub fn initialize(rsp0_address: u64) {
    let task_state_segment_address = unsafe { addr_of!(TASK_STATE_SEGMENT) as usize };
    let limit = size_of::<TaskStateSegment>() - 1;
    unsafe {
        task_state_segment_descriptor = TaskStateSegmentDescriptor {
            limit_low: limit as u16,
            offset_low: task_state_segment_address as u16,
            offset_low_middle: (task_state_segment_address >> 16) as u8,
            flags: TaskStateSegmentDescriptorFlags::TSS_TYPE
                | TaskStateSegmentDescriptorFlags::VALID,
            limit_high_and_additional_flags: ((limit >> 16) as u8) | DEFAULT_ADDITIONAL_FLAGS,
            offset_high_middle: (task_state_segment_address >> 24) as u8,
            offset_high: (task_state_segment_address >> 32) as u32,
            zero: 0,
        };
        TASK_STATE_SEGMENT.rsp0 = rsp0_address;

        load_task_state_segment(0x28);
    }
}
