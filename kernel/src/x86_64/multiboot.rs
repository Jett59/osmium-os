use core::mem::size_of;

use crate::{
    arch_api::{acpi, initial_ramdisk},
    heap::{map_physical_memory, PhysicalAddressHandle},
    memory::{
        align_address_down, align_address_up, reinterpret_memory, slice_from_memory,
        DynamicallySized, DynamicallySizedItem, DynamicallySizedObjectIterator, Endianness,
        Validateable,
    },
    paging::{MemoryType, PagePermissions},
    physical_memory_manager::{mark_range_as_free, mark_range_as_used, BLOCK_SIZE},
};
use common::framebuffer::{self, FrameBuffer};

#[repr(C, packed)]
struct MbiHeader {
    total_size: u32,
    _reserved: u32,
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
        self.size >= 8
    }
}

impl DynamicallySized for MbiTag {
    fn size(&self) -> usize {
        self.size as usize
    }

    const ALIGNMENT: usize = 8;
}

const MBI_TAG_MODULE: u32 = 3;
const MBI_TAG_MEMORY_MAP: u32 = 6;
const MBI_TAG_FRAME_BUFFER: u32 = 8;
const MBI_TAG_ACPI_OLD: u32 = 14;
const MBI_TAG_ACPI_NEW: u32 = 15;

#[repr(C, packed)]
struct MbiModuleTag {
    base_tag: MbiTag,
    module_start: u32,
    module_end: u32,
}

impl Validateable for MbiModuleTag {
    fn validate(&self) -> bool {
        // Make sure we are the right type and that the module end is after the module start.
        self.base_tag.tag_type == MBI_TAG_MODULE && self.module_end > self.module_start
    }
}

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

#[repr(C, packed)]
struct MbiFrameBufferTag {
    base_tag: MbiTag,
    address: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bits_per_pixel: u8,
    framebuffer_type: u8,
    _reserved: u16,
    red_position: u8,
    _red_mask_size: u8,
    green_position: u8,
    _green_mask_size: u8,
    blue_position: u8,
    _blue_mask_size: u8,
}

impl Validateable for MbiFrameBufferTag {
    fn validate(&self) -> bool {
        self.base_tag.tag_type == MBI_TAG_FRAME_BUFFER
        // If we are an rgb framebuffer tag, we should have exactly the size of this structure. Otherwise we don't know.
            && (self.framebuffer_type != 1 || self.base_tag.size == size_of::<MbiFrameBufferTag>() as u32)
            && self.bits_per_pixel % 8 == 0
            && self.pitch >= self.width * self.bits_per_pixel as u32 / 8
    }
}

#[repr(C, packed)]
struct MbiAcpiOldTag {
    base_tag: MbiTag,
    rsdp_signature: [u8; 8],
    rsdp_checksum: u8,
    rsdp_oem_id: [u8; 6],
    rsdp_revision: u8,
    rsdt_address: u32,
}

impl Validateable for MbiAcpiOldTag {
    fn validate(&self) -> bool {
        self.base_tag.tag_type == MBI_TAG_ACPI_OLD
            && self.base_tag.size == size_of::<MbiAcpiOldTag>() as u32
            && self.rsdp_signature == *b"RSD PTR "
            && self.rsdp_revision == 0
    }
}

#[repr(C, packed)]
struct MbiAcpiNewTag {
    base_tag: MbiTag,
    rsdp_signature: [u8; 8],
    rsdp_checksum: u8,
    rsdp_oem_id: [u8; 6],
    rsdp_revision: u8,
    rsdt_address: u32,
    rsdp_length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    _reserved: [u8; 3],
}

impl Validateable for MbiAcpiNewTag {
    fn validate(&self) -> bool {
        self.base_tag.tag_type == MBI_TAG_ACPI_NEW
            && self.base_tag.size == size_of::<MbiAcpiNewTag>() as u32
            && self.rsdp_signature == *b"RSD PTR "
            && self.rsdp_revision >= 2
            && self.rsdp_length == size_of::<MbiAcpiNewTag>() as u32 - size_of::<MbiTag>() as u32
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
    let tag_iterator: DynamicallySizedObjectIterator<&MbiTag> =
        DynamicallySizedObjectIterator::new(Endianness::Little, tag_memory);
    let mut frame_buffer = None; // Delayed initialization to allow for memory to be detected first.
    let mut module = None; // Same as above
    let mut found_new_acpi = false;
    for DynamicallySizedItem {
        value: tag,
        value_memory: tag_memory,
    } in tag_iterator
    {
        let tag_type = tag.tag_type;
        match tag_type {
            MBI_TAG_MODULE => {
                let module_tag: &MbiModuleTag = unsafe { reinterpret_memory(tag_memory).unwrap() };
                module = Some(module_tag);
            }
            MBI_TAG_MEMORY_MAP => {
                let memory_map_tag: &MbiMemoryMapTag =
                    unsafe { reinterpret_memory(tag_memory).unwrap() };
                parse_memory_map(memory_map_tag, tag_memory);
            }
            MBI_TAG_FRAME_BUFFER => {
                let frame_buffer_tag: &MbiFrameBufferTag =
                    unsafe { reinterpret_memory(tag_memory).unwrap() };
                frame_buffer = Some(frame_buffer_tag);
            }
            MBI_TAG_ACPI_OLD if !found_new_acpi => {
                let acpi_old_tag: &MbiAcpiOldTag =
                    unsafe { reinterpret_memory(tag_memory).unwrap() };
                acpi::init(acpi_old_tag.rsdt_address as usize);
            }
            MBI_TAG_ACPI_NEW => {
                found_new_acpi = true;
                let acpi_new_tag: &MbiAcpiNewTag =
                    unsafe { reinterpret_memory(tag_memory).unwrap() };
                acpi::init(acpi_new_tag.xsdt_address as usize);
            }
            0 => break, // End of tags
            _ => {}
        }
    }
    if let Some(module) = module {
        parse_module(module);
    }
    if let Some(frame_buffer) = frame_buffer {
        parse_frame_buffer(frame_buffer);
    }
}

fn parse_module(module: &MbiModuleTag) {
    // Since Grub puts the module in `available` memory, we need to explicitly mark it as used.
    let start_address = align_address_down(module.module_start as usize, BLOCK_SIZE);
    let end_address = align_address_up(module.module_end as usize, BLOCK_SIZE);
    mark_range_as_used(start_address, end_address);

    let module_size = module.module_end - module.module_start;
    // SAFETY: The memory should be valid (Grub makes sure of this), and it won't be given out to anyone since it is marked as used.
    let module_memory = unsafe {
        map_physical_memory(
            module.module_start as usize,
            module_size as usize,
            MemoryType::Normal,
            PagePermissions::KERNEL_READ_ONLY,
        )
    };
    // SAFETY: There are no data races possible, since there is only one thread running at the moment.
    unsafe {
        initial_ramdisk::INITIAL_RAMDISK = Some(PhysicalAddressHandle::leak(module_memory));
    }
}

#[repr(C, packed)]
struct MemoryMapEntry {
    base_address: u64,
    length: u64,
    entry_type: u32,
    _reserved: u32,
}

impl Validateable for MemoryMapEntry {
    fn validate(&self) -> bool {
        // The length must not be zero and the end address must be less than the limit on the physical address space (56 bits)
        self.length > 0 && self.base_address + self.length < (1 << 56)
    }
}

fn parse_memory_map(memory_map: &MbiMemoryMapTag, tag_memory: &[u8]) {
    let entry_area_size = memory_map.base_tag.size - size_of::<MbiMemoryMapTag>() as u32;
    let entry_area = &tag_memory[size_of::<MbiMemoryMapTag>()..];
    let entry_size = memory_map.entry_size;
    let entry_count = entry_area_size / entry_size;
    for i in 0..entry_count {
        let entry_memory = &entry_area[entry_size as usize * i as usize..];
        let entry: &MemoryMapEntry = unsafe { reinterpret_memory(entry_memory).unwrap() };
        // Type 1 means available, so therefore we should mark them as such in the PMM (by default everything is used).
        if entry.entry_type == 1 {
            let starting_address = align_address_up(entry.base_address as usize, BLOCK_SIZE);
            let ending_address = align_address_down(
                entry.base_address as usize + entry.length as usize,
                BLOCK_SIZE,
            );
            mark_range_as_free(starting_address, ending_address);
        }
    }
}

fn parse_frame_buffer(frame_buffer: &MbiFrameBufferTag) {
    // The frame buffer may still be marked as valid even if it doesn't use RGB mode.
    if frame_buffer.framebuffer_type == 1 {
        framebuffer::init(FrameBuffer {
            width: frame_buffer.width as usize,
            height: frame_buffer.height as usize,
            pitch: frame_buffer.pitch as usize,
            bytes_per_pixel: frame_buffer.bits_per_pixel / 8,
            red_byte: frame_buffer.red_position / 8,
            green_byte: frame_buffer.green_position / 8,
            blue_byte: frame_buffer.blue_position / 8,
            pixels: {
                // # Safety
                // This is the only place where the framebuffer is mapped, so there should be no aliasing issues.
                let physical_address_handle = unsafe {
                    map_physical_memory(
                        frame_buffer.address as usize,
                        frame_buffer.pitch as usize * frame_buffer.height as usize,
                        MemoryType::Device,
                        PagePermissions::KERNEL_READ_WRITE,
                    )
                };
                PhysicalAddressHandle::leak(physical_address_handle)
            },
        });
    }
}
