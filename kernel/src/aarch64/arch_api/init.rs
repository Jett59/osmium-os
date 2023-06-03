use crate::arch::beryllium::{BootRequestTagType, FrameBufferTag, StackPointerTag};
use crate::arch_api::stack::Stack;
use crate::framebuffer;
use crate::heap::{map_physical_memory, PhysicalAddressHandle};
use core::mem::size_of;

// We include the stack pointer request tag here because I don't know where else it should go. TODO: maybe change this later?
static mut STACK: Stack = Stack::default();
#[link_section = ".beryllium"]
#[no_mangle]
pub static mut STACK_POINTER_TAG: StackPointerTag = StackPointerTag {
    tag_type: BootRequestTagType::StackPointer,
    size: size_of::<StackPointerTag>() as u16,
    flags: 0,
    base: unsafe { STACK.as_mut_ptr() },
    memory_size: size_of::<Stack>(),
};

#[link_section = ".beryllium"]
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

pub fn arch_init() {
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
                );
                PhysicalAddressHandle::leak(physical_memory_handle)
            },
        })
    }
}
