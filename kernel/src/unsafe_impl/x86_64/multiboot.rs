use core::{mem::size_of, ptr::addr_of};

use crate::{
    arch_api::{acpi, initial_ramdisk},
    memory::{
        Array, DynamicallySized, DynamicallySizedItem, DynamicallySizedObjectIterator, Endianness,
        FromBytes, ReservedMemory, Validateable, align_address_down, align_address_up,
    },
    memory_allocator::map_physical_memory,
    memory_struct,
    paging::PagePermissions,
    physical_memory_manager::{BLOCK_SIZE, mark_as_free},
    unsafe_impl::{
        init_cell::NoConcurrency,
        memory_token::{MemoryToken, PhysicalMmioToken, PhysicalRoToken, PhysicalRwToken},
    },
};
use common::framebuffer::{self, FrameBuffer};

#[cfg(not(test))]
unsafe extern "C" {
    #[allow(improper_ctypes)]
    static KERNEL_PHYSICAL_END: ();
}

#[cfg(test)]
static KERNEL_PHYSICAL_END: () = ();

memory_struct! {
struct MbiHeader<'a> {
    total_size: u32,
    _reserved: ReservedMemory<4>,
}
}

impl Validateable for MbiHeader<'_> {
    fn validate(&self) -> bool {
        // We must be at least 8 bytes and aligned to an 8-byte boundary.
        self.total_size() >= 8 && self.total_size().is_multiple_of(8)
    }
}

#[cfg(not(test))] // Unless you want a link error
unsafe extern "C" {
    static mbi_pointer: *const u8;
}

#[cfg(test)]
#[allow(non_upper_case_globals)]
const mbi_pointer: *const u8 = core::ptr::null();

memory_struct! {
struct MbiTag<'a> {
    tag_type: u32,
    size: u32,
}
}

impl Validateable for MbiTag<'_> {
    fn validate(&self) -> bool {
        self.size() >= 8
    }
}

impl DynamicallySized for MbiTag<'_> {
    fn size(&self) -> usize {
        self.size() as usize
    }

    const ALIGNMENT: usize = 8;
}

const MBI_TAG_MODULE: u32 = 3;
const MBI_TAG_MEMORY_MAP: u32 = 6;
const MBI_TAG_FRAME_BUFFER: u32 = 8;
const MBI_TAG_ACPI_OLD: u32 = 14;
const MBI_TAG_ACPI_NEW: u32 = 15;

memory_struct! {
struct MbiModuleTag<'lifetime> {
    base_tag: MbiTag<'lifetime>,
    module_start: u32,
    module_end: u32,
}
}

impl Validateable for MbiModuleTag<'_> {
    fn validate(&self) -> bool {
        // Make sure we are the right type and that the module end is after the module start.
        self.base_tag().tag_type() == MBI_TAG_MODULE && self.module_end() > self.module_start()
    }
}

memory_struct! {
struct MbiMemoryMapTag<'lifetime> {
    base_tag: MbiTag<'lifetime>,
    entry_size: u32,
    entry_version: u32,
}
}

impl Validateable for MbiMemoryMapTag<'_> {
    fn validate(&self) -> bool {
        // Make sure we are the right type, the entry size is at least the minimum (24) and a multiple of 8 bytes and also make sure there is at least one entry.
        self.base_tag().tag_type() == MBI_TAG_MEMORY_MAP
            && self.entry_size() >= 24
            && self.entry_size().is_multiple_of(8)
    }
}

memory_struct! {
struct MbiFrameBufferTag<'lifetime> {
    base_tag: MbiTag<'lifetime>,
    address: u64,
    pitch: u32,
    width: u32,
    height: u32,
    bits_per_pixel: u8,
    framebuffer_type: u8,
    _reserved: ReservedMemory<2>,
    red_position: u8,
    _red_mask_size: u8,
    green_position: u8,
    _green_mask_size: u8,
    blue_position: u8,
    _blue_mask_size: u8,
}
}

impl Validateable for MbiFrameBufferTag<'_> {
    fn validate(&self) -> bool {
        self.base_tag().tag_type() == MBI_TAG_FRAME_BUFFER
        // If we are an rgb framebuffer tag, we should have exactly the size of this structure. Otherwise we don't know.
            && (self.framebuffer_type() != 1 || self.base_tag().size() == MbiFrameBufferTag::SIZE as u32)
            && self.bits_per_pixel().is_multiple_of(8)
            && self.pitch() >= self.width() * self.bits_per_pixel() as u32 / 8
    }
}

memory_struct! {
struct MbiAcpiOldTag<'lifetime> {
    base_tag: MbiTag<'lifetime>,
    rsdp_signature: Array<'lifetime, u8, 8>,
    rsdp_checksum: u8,
    rsdp_oem_id: Array<'lifetime, u8, 6>,
    rsdp_revision: u8,
    rsdt_address: u32,
}
}

impl Validateable for MbiAcpiOldTag<'_> {
    fn validate(&self) -> bool {
        self.base_tag().tag_type() == MBI_TAG_ACPI_OLD
            && self.base_tag().size() == MbiAcpiOldTag::SIZE as u32
            && *self.rsdp_signature() == *b"RSD PTR "
    }
}

memory_struct! {
struct MbiAcpiNewTag<'lifetime> {
    base_tag: MbiTag<'lifetime>,
    rsdp_signature: Array<'lifetime, u8, 8>,
    rsdp_checksum: u8,
    rsdp_oem_id: Array<'lifetime, u8, 6>,
    rsdp_revision: u8,
    rsdt_address: u32,
    rsdp_length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    _reserved: ReservedMemory<3>,
}
}

impl Validateable for MbiAcpiNewTag<'_> {
    fn validate(&self) -> bool {
        self.base_tag().tag_type() == MBI_TAG_ACPI_NEW
            && self.base_tag().size() == MbiAcpiNewTag::SIZE as u32
            && *self.rsdp_signature() == *b"RSD PTR "
            && self.rsdp_revision() >= 2
            && self.rsdp_length() == MbiAcpiNewTag::SIZE as u32 - MbiTag::SIZE as u32
    }
}

pub fn parse_multiboot_structures(no_concurrency: &NoConcurrency) {
    let mbi_header: MbiHeader = unsafe {
        MbiHeader::from_bytes(
            Endianness::Little,
            core::slice::from_raw_parts(mbi_pointer, size_of::<MbiHeader>()),
        )
        .unwrap()
    };
    let tag_memory = unsafe {
        core::slice::from_raw_parts(
            mbi_pointer.add(MbiHeader::SIZE),
            mbi_header.total_size() as usize - MbiHeader::SIZE,
        )
    };
    let tag_iterator: DynamicallySizedObjectIterator<MbiTag> =
        DynamicallySizedObjectIterator::new(Endianness::Little, tag_memory);
    let mut memory_map = None; // Delayed initialization to allow for module to be detected first.
    let mut memory_map_tag_memory = None;
    let mut frame_buffer = None; // Delayed initialization to allow for memory to be detected first.
    let mut module = None; // Same as above
    let mut found_new_acpi = false;
    for DynamicallySizedItem {
        value: tag,
        value_memory: tag_memory,
    } in tag_iterator
    {
        let tag_type = tag.tag_type();
        match tag_type {
            MBI_TAG_MODULE => {
                let module_tag: MbiModuleTag =
                    MbiModuleTag::from_bytes(Endianness::Little, tag_memory).unwrap();
                assert!(module_tag.validate());
                module = Some(module_tag);
            }
            MBI_TAG_MEMORY_MAP => {
                let memory_map_tag: MbiMemoryMapTag =
                    MbiMemoryMapTag::from_bytes(Endianness::Little, tag_memory).unwrap();
                assert!(memory_map_tag.validate());
                memory_map = Some(memory_map_tag);
                memory_map_tag_memory = Some(tag_memory);
            }
            MBI_TAG_FRAME_BUFFER => {
                let frame_buffer_tag: MbiFrameBufferTag =
                    MbiFrameBufferTag::from_bytes(Endianness::Little, tag_memory).unwrap();
                assert!(frame_buffer_tag.validate());
                frame_buffer = Some(frame_buffer_tag);
            }
            MBI_TAG_ACPI_OLD if !found_new_acpi => {
                let acpi_old_tag: MbiAcpiOldTag =
                    MbiAcpiOldTag::from_bytes(Endianness::Little, tag_memory).unwrap();
                assert!(acpi_old_tag.validate());
                acpi::init(acpi_old_tag.rsdt_address() as usize, no_concurrency);
            }
            MBI_TAG_ACPI_NEW => {
                found_new_acpi = true;
                let acpi_new_tag: MbiAcpiNewTag =
                    MbiAcpiNewTag::from_bytes(Endianness::Little, tag_memory).unwrap();
                assert!(acpi_new_tag.validate());
                acpi::init(acpi_new_tag.xsdt_address() as usize, no_concurrency);
            }
            0 => break, // End of tags
            _ => {}
        }
    }
    // Since Grub puts the module in `available` memory, we need to explicitly mark it as used.
    let module_start_address = align_address_down(
        module.map_or(0, |module| module.module_start() as usize),
        BLOCK_SIZE,
    );
    let module_end_address = align_address_up(
        module.map_or(0, |module| module.module_end() as usize),
        BLOCK_SIZE,
    );
    if let Some(memory_map) = memory_map {
        parse_memory_map(
            memory_map,
            memory_map_tag_memory.unwrap(),
            &[
                (
                    module_start_address,
                    module_end_address - module_start_address,
                ),
                (
                    0,
                    align_address_up(addr_of!(KERNEL_PHYSICAL_END) as usize, BLOCK_SIZE),
                ),
            ],
        );
    }
    if let Some(module) = module {
        parse_module(module, no_concurrency);
    }
    if let Some(frame_buffer) = frame_buffer {
        parse_frame_buffer(frame_buffer);
    }
}

fn parse_module(module: MbiModuleTag, no_concurrency: &NoConcurrency) {
    let module_size = module.module_end() - module.module_start();
    // SAFETY: The memory should be valid (Grub makes sure of this), and it won't be given out to anyone since it is marked as used.
    let module_memory = map_physical_memory(
        unsafe { PhysicalRoToken::new(module.module_start() as usize, module_size as usize) },
        PagePermissions::KERNEL_READ_ONLY,
    );
    // SAFETY: There are no data races possible, since there is only one thread running at the moment.
    initial_ramdisk::set_initial_ramdisk(module_memory.into_slice(), no_concurrency);
}

memory_struct! {
struct MemoryMapEntry<'a> {
    base_address: u64,
    length: u64,
    entry_type: u32,
    _reserved: ReservedMemory<4>,
}
}

impl Validateable for MemoryMapEntry<'_> {
    fn validate(&self) -> bool {
        // The length must not be zero and the end address must be less than the limit on the physical address space (56 bits)
        self.length() > 0 && self.base_address() + self.length() < (1 << 56)
    }
}

fn parse_memory_map(memory_map: MbiMemoryMapTag, tag_memory: &[u8], exclusions: &[(usize, usize)]) {
    let entry_area_size = memory_map.base_tag().size() - MbiMemoryMapTag::SIZE as u32;
    let entry_area = &tag_memory[MbiMemoryMapTag::SIZE..];
    let entry_size = memory_map.entry_size();
    let entry_count = entry_area_size / entry_size;
    for i in 0..entry_count {
        let entry_memory = &entry_area[entry_size as usize * i as usize..];
        let entry: MemoryMapEntry =
            MemoryMapEntry::from_bytes(Endianness::Little, entry_memory).unwrap();
        assert!(entry.validate());
        // Type 1 means available, so therefore we should mark them as such in the PMM (by default everything is used).
        if entry.entry_type() == 1 {
            let start_address = align_address_up(entry.base_address() as usize, BLOCK_SIZE);
            let end_address = align_address_down(
                entry.base_address() as usize + entry.length() as usize,
                BLOCK_SIZE,
            );
            // We must remove the exclusions here
            // The easiest way is to go through each block and skip the excluded ones
            for block_start_address in (start_address..end_address).step_by(BLOCK_SIZE) {
                let block_end_address = block_start_address + BLOCK_SIZE;
                if exclusions
                    .iter()
                    .any(|&(exclusion_start, exclusion_length)| {
                        let exclusion_end = exclusion_start + exclusion_length;
                        // Check if the block overlaps with the exclusion
                        !(block_end_address <= exclusion_start
                            || block_start_address >= exclusion_end)
                    })
                {
                    continue; // Skip this block as it overlaps with an exclusion
                }
                // SAFETY: we know that this block is validand unused
                let token = unsafe { PhysicalRwToken::new(block_start_address, BLOCK_SIZE) };
                mark_as_free(token);
            }
        }
    }
}

fn parse_frame_buffer(frame_buffer: MbiFrameBufferTag) {
    if frame_buffer.framebuffer_type() == 1 {
        framebuffer::init(FrameBuffer {
            width: frame_buffer.width() as usize,
            height: frame_buffer.height() as usize,
            pitch: frame_buffer.pitch() as usize,
            bytes_per_pixel: frame_buffer.bits_per_pixel() / 8,
            red_byte: frame_buffer.red_position() / 8,
            green_byte: frame_buffer.green_position() / 8,
            blue_byte: frame_buffer.blue_position() / 8,
            pixels: {
                // # Safety
                // This is the only place where the framebuffer is mapped, so there should be no aliasing issues.
                let physical_address_handle = map_physical_memory(
                    unsafe {
                        PhysicalMmioToken::new(
                            frame_buffer.address() as usize,
                            frame_buffer.pitch() as usize * frame_buffer.height() as usize,
                        )
                    },
                    PagePermissions::KERNEL_READ_WRITE,
                );
                physical_address_handle.into_mut_ptr()
            },
        });
    }
}
