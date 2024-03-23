use common::framebuffer;

use crate::arch::exceptions::load_exceptions;
use crate::arch_api::stack::Stack;
use crate::heap::{map_physical_memory, PhysicalAddressHandle};
use crate::physical_memory_manager;
use common::beryllium::{
    BootRequestTagType, FrameBufferTag, MemoryMapEntry, MemoryMapEntryType, MemoryMapTag,
    StackPointerTag,
};

use core::mem::size_of;
use core::ptr::null;
use core::slice;

use super::paging::MemoryType;

// We include the stack pointer request tag here because I don't know where else it should go. TODO: maybe change this later?
static mut STACK: Stack = Stack::default();
#[cfg_attr(not(test), link_section = ".beryllium")]
#[no_mangle]
pub static mut STACK_POINTER_TAG: StackPointerTag = StackPointerTag {
    tag_type: BootRequestTagType::StackPointer,
    size: size_of::<StackPointerTag>() as u16,
    flags: 0,
    base: unsafe { STACK.as_mut_ptr() },
    memory_size: size_of::<Stack>(),
};

#[cfg_attr(not(test), link_section = ".beryllium")]
#[no_mangle]
pub static mut FRAME_BUFFER_TAG: FrameBufferTag = FrameBufferTag {
    tag_type: BootRequestTagType::FrameBuffer,
    size: size_of::<FrameBufferTag>() as u16,
    flags: 0,
    address: 0,
    width: 0,
    height: 0,
    pitch: 0,
    bits_per_pixel: 0,
    red_byte: 0,
    green_byte: 0,
    blue_byte: 0,
};

#[cfg_attr(not(test), link_section = ".beryllium")]
#[no_mangle]
pub static mut MEMORY_MAP_TAG: MemoryMapTag = MemoryMapTag {
    tag_type: BootRequestTagType::MemoryMap,
    size: size_of::<MemoryMapTag>() as u16,
    flags: 0,
    base: null(),
    memory_size: 0,
};

pub fn arch_init() {
    load_exceptions();

    let memory_map = unsafe {
        slice::from_raw_parts(
            MEMORY_MAP_TAG.base as *const MemoryMapEntry,
            MEMORY_MAP_TAG.memory_size / size_of::<MemoryMapEntry>(),
        )
    };
    for entry in memory_map {
        if entry.memory_type == MemoryMapEntryType::Available {
            physical_memory_manager::mark_range_as_free(
                entry.address as usize,
                entry.address as usize + entry.size,
            );
        }
    }

    unsafe {
        framebuffer::init(framebuffer::FrameBuffer {
            width: FRAME_BUFFER_TAG.width as usize,
            height: FRAME_BUFFER_TAG.height as usize,
            pitch: FRAME_BUFFER_TAG.pitch as usize,
            bytes_per_pixel: (FRAME_BUFFER_TAG.bits_per_pixel / 8) as u8,
            red_byte: FRAME_BUFFER_TAG.red_byte,
            green_byte: FRAME_BUFFER_TAG.green_byte,
            blue_byte: FRAME_BUFFER_TAG.blue_byte,
            pixels: {
                let physical_memory_handle = map_physical_memory(
                    FRAME_BUFFER_TAG.address,
                    FRAME_BUFFER_TAG.pitch as usize * FRAME_BUFFER_TAG.height as usize,
                    MemoryType::Device,
                );
                PhysicalAddressHandle::leak(physical_memory_handle)
            },
        });
    }
}
