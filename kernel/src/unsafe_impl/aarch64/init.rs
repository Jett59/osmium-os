use core::{
    ptr::null,
    slice,
    sync::atomic::{AtomicBool, Ordering},
};

use common::{
    beryllium::{
        BootRequestTagType, FrameBufferTag, MemoryMapEntry, MemoryMapEntryType, MemoryMapTag,
    },
    framebuffer::FrameBuffer,
};

use crate::{
    memory_allocator::{PhysicalAddressHandle, map_physical_memory},
    paging::{MemoryType, PagePermissions},
    unsafe_impl::memory_token::{
        MemoryToken, PhysicalMemoryToken, PhysicalMmioToken, PhysicalRwToken,
    },
};

#[cfg_attr(not(test), unsafe(link_section = ".beryllium"))]
#[unsafe(no_mangle)]
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

#[cfg_attr(not(test), unsafe(link_section = ".beryllium"))]
#[unsafe(no_mangle)]
pub static mut MEMORY_MAP_TAG: MemoryMapTag = MemoryMapTag {
    tag_type: BootRequestTagType::MemoryMap,
    size: size_of::<MemoryMapTag>() as u16,
    flags: 0,
    base: null(),
    memory_size: 0,
};

static GOT_MEMORY_MAP: AtomicBool = AtomicBool::new(false);

pub fn available_memory_map_entries() -> impl Iterator<Item = PhysicalRwToken> {
    let entries: &[MemoryMapEntry] = if GOT_MEMORY_MAP.swap(true, Ordering::SeqCst) {
        &[]
    } else {
        // SAFETY: it is in the beryllium spec that the memory map tag is valid and points to a valid memory map, so we can trust it.
        unsafe {
            slice::from_raw_parts(
                MEMORY_MAP_TAG.base as *const MemoryMapEntry,
                MEMORY_MAP_TAG.memory_size / size_of::<MemoryMapEntry>(),
            )
        }
    };

    entries
        .iter()
        .filter(|entry| entry.memory_type == MemoryMapEntryType::Available)
        // SAFETY: the memory map ensures that the regions are valid and unused.
        .map(|entry| unsafe { PhysicalRwToken::new(entry.address as usize, entry.size) })
}

static GOT_FRAME_BUFFER: AtomicBool = AtomicBool::new(false);

pub fn frame_buffer() -> Option<FrameBuffer> {
    if GOT_FRAME_BUFFER.swap(true, Ordering::SeqCst) {
        None
    } else {
        // SAFETY: no osmium code modifies the framebuffer tag, so it is safe to read it.
        let tag = unsafe { FRAME_BUFFER_TAG };
        // SAFETY: the spec ensures that the frame buffer is distinct from other memory regions, so it is safe to map it.
        let physical_memory_handle = unsafe {
            map_physical_memory::<PhysicalMmioToken>(
                tag.address,
                tag.pitch as usize * tag.height as usize,
                PagePermissions::KERNEL_READ_WRITE,
            )
        };
        Some(FrameBuffer {
            width: tag.width as usize,
            height: tag.height as usize,
            pitch: tag.pitch as usize,
            bytes_per_pixel: (tag.bits_per_pixel / 8) as u8,
            red_byte: tag.red_byte,
            green_byte: tag.green_byte,
            blue_byte: tag.blue_byte,
            pixels: physical_memory_handle.into_mut_ptr(),
        })
    }
}
