use core::mem::size_of;

use crate::{
    arch_api::console,
    memory::{
        reinterpret_memory, slice_from_memory, DynamicallySized, DynamicallySizedItem,
        DynamicallySizedObjectIterator, Validateable,
    },
};

#[repr(C, packed)]
struct MbiHeader {
    total_size: u32,
    reserved: u32,
}

impl Validateable for MbiHeader {
    fn validate(&self) -> bool {
        // We must be at least 8 bytes and aligned to an 8-byte boundary.
        self.total_size >= 8 && self.total_size % 8 == 0
    }
}

#[cfg(not(test))] // Unless you want a link error
extern "C" {
    static mbi_pointer: *const u8;
}

#[cfg(test)]
#[allow(non_upper_case_globals)]
const mbi_pointer: *const u8 = 0 as *const u8;

#[repr(C, packed)]
struct MbiTag {
    tag_type: u32,
    size: u32,
}

impl Validateable for MbiTag {
    fn validate(&self) -> bool {
        true // I don't think there is any way to validate it.
    }
}

impl DynamicallySized for MbiTag {
    fn size(&self) -> usize {
        self.size as usize
    }

    const ALIGNMENT: usize = 8;
}

const MBI_TAG_MEMORY_MAP: u32 = 6;

#[repr(C, packed)]
struct MbiMemoryMapTag {
    base_tag: MbiTag,
    entry_size: u32,
    entry_version: u32,
}

impl Validateable for MbiMemoryMapTag {
    fn validate(&self) -> bool {
        // Make sure we are the right type, the entry size is at least the minimum (24) and a multiple of 8 bytes and also make sure there is at least one entry.
        self.base_tag.tag_type == MBI_TAG_MEMORY_MAP
            && self.entry_size >= 24
            && self.entry_size % 8 == 0
            && self.base_tag.size > size_of::<MbiMemoryMapTag>() as u32
    }
}

pub fn parse_multiboot_structures() {
    let mbi_header: &MbiHeader = unsafe {
        reinterpret_memory(slice_from_memory(mbi_pointer, size_of::<MbiHeader>()).unwrap()).unwrap()
    };
    let tag_memory = unsafe {
        slice_from_memory(
            mbi_pointer.add(size_of::<MbiHeader>()),
            mbi_header.total_size as usize - size_of::<MbiHeader>(),
        )
        .unwrap()
    };
    let tag_iterator: DynamicallySizedObjectIterator<MbiTag> =
        DynamicallySizedObjectIterator::new(tag_memory);
    for DynamicallySizedItem {
        value: tag,
        value_memory: tag_memory,
    } in tag_iterator
    {
        if tag.tag_type == MBI_TAG_MEMORY_MAP {
            let memory_map_tag: &MbiMemoryMapTag =
                unsafe { reinterpret_memory(tag_memory).unwrap() };
                console::write_string("Found the memory map!\n");
        }
    }
}
