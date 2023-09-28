#![no_main]
#![no_std]

#[cfg_attr(target_arch = "aarch64", path = "aarch64/mod.rs")]
mod arch;
mod beryllium;
mod config;
mod elf;
mod toml;

extern crate alloc;

use core::{mem::size_of, slice};

use alloc::vec;
use alloc::vec::Vec;
use config::Config;
use uefi::{
    prelude::*,
    proto::console::gop::{GraphicsOutput, ModeInfo, PixelFormat},
    table::boot::{AllocateType, MemoryType, OpenProtocolAttributes, OpenProtocolParams},
};
use uefi::{CStr16, Result};
use uefi_services::println;

use crate::{
    arch::{page_align_up, PageAllocator, PAGE_SIZE},
    beryllium::{MemoryMapEntry, MemoryMapEntryType, MemoryMapTag},
    config::parse_config,
};

struct GraphicsInfo {
    mode: ModeInfo,
    frame_buffer_ptr: *mut u8,
}

//Function to get the handle for the graphics output protocol
fn graphics(image: Handle, boot_services: &BootServices) -> Result<GraphicsInfo> {
    let handle = boot_services
        .get_handle_for_protocol::<GraphicsOutput>()
        .unwrap();
    let mut graphics_output_protocol = unsafe {
        boot_services.open_protocol::<GraphicsOutput>(
            OpenProtocolParams {
                handle,
                agent: image,
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }
    .unwrap();

    //query modes
    let modes = graphics_output_protocol.modes();

    //filter on modes that are not PixelFormat::BltOnly and return the info of the largest one
    let mode = modes
        .filter(|mode| mode.info().pixel_format() != PixelFormat::BltOnly)
        .max_by_key(|mode| mode.info().resolution().0 * mode.info().resolution().1)
        .unwrap();

    //set the mode
    graphics_output_protocol.set_mode(&mode)?;

    //Get the frame buffer
    let mut frame_buffer = graphics_output_protocol.frame_buffer();
    let frame_buffer_ptr = frame_buffer.as_mut_ptr();

    Ok(GraphicsInfo {
        mode: *mode.info(),
        frame_buffer_ptr,
    })
}

#[entry]
fn main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    system_table.stdout().clear().unwrap();

    arch::check_environment();

    let boot_services = system_table.boot_services();
    let config = read_config(image, boot_services).unwrap();

    println!("Loading kernel from {}", config.default_entry);

    //find entry with default_entry label
    let entry = config
        .entries
        .iter()
        .find(|entry| entry.label == config.default_entry)
        .unwrap();
    println!("Kernel path: {}", entry.kernel_path);

    boot_services.stall((config.timeout * 1_000_000) as usize);

    load_kernel(image, system_table, entry.kernel_path.as_str()).unwrap();

    loop {}
}

fn read_file(image: Handle, boot_services: &BootServices, name: &CStr16) -> Result<Vec<u8>> {
    let mut fs = boot_services.get_image_file_system(image)?;
    fs.read(name).map_err(|error| match error {
        uefi::fs::Error::Io(io_error) => io_error.uefi_error,
        uefi::fs::Error::Path(_) => uefi::Error::new(Status::INVALID_PARAMETER, ()),
        uefi::fs::Error::Utf8Encoding(_) => uefi::Error::new(Status::INVALID_PARAMETER, ()),
    })
}

fn read_config(image: Handle, boot_services: &BootServices) -> Result<Config> {
    let bytes = read_file(image, boot_services, cstr16!("\\boot\\osmium\\boot.toml"))?;

    let config_string = core::str::from_utf8(&bytes).unwrap();
    let config = parse_config(config_string);

    Ok(config)
}

/// If this function succeeds, it will never return.
fn load_kernel(image: Handle, system_table: SystemTable<Boot>, path: &str) -> Result {
    let boot_services = system_table.boot_services();

    let path = path.replace('/', "\\");
    let mut path_buffer = vec![0u16; path.len() + 1]; // Includes null terminator.
    let mut kernel_binary = read_file(
        image,
        boot_services,
        CStr16::from_str_with_buf(path.as_str(), path_buffer.as_mut_slice()).unwrap(),
    )?;
    let elf = elf::load_elf(kernel_binary.as_slice()).unwrap();

    // Since the memory map has to go after the kernel (according to the spec), we find the end of the kernel and map it there.
    let memory_map_virtual_address = page_align_up(
        elf.loadable_segments
            .iter()
            .map(|segment| segment.virtual_address + segment.size_in_memory)
            .max()
            .unwrap(),
    );

    let beryllium_section = elf
        .sections
        .iter()
        .find(|section| section.name == ".beryllium")
        .expect("Beryllium signature not found");
    assert!(
        beryllium_section.size >= 16,
        "Beryllium signature not found"
    );
    let beryllium_bytes = &mut kernel_binary[beryllium_section.file_offset as usize
        ..(beryllium_section.file_offset + beryllium_section.size) as usize];
    let beryllium_signature = &beryllium_bytes[..16];
    assert!(
        beryllium_signature == b"Beryllium Ready!",
        "Beryllium signature not found"
    );
    let tag_bytes = &mut beryllium_bytes[16..];
    let tags = beryllium::parse_tags(tag_bytes);

    if let Some(frame_buffer_tag) = tags.frame_buffer {
        let graphics = graphics(image, boot_services).unwrap();

        frame_buffer_tag.address = graphics.frame_buffer_ptr as usize;
        frame_buffer_tag.width = graphics.mode.resolution().0 as u32;
        frame_buffer_tag.height = graphics.mode.resolution().1 as u32;
        match graphics.mode.pixel_format() {
            PixelFormat::Bgr => {
                frame_buffer_tag.bits_per_pixel = 32;
                frame_buffer_tag.red_byte = 2;
                frame_buffer_tag.green_byte = 1;
                frame_buffer_tag.blue_byte = 0;
            }
            PixelFormat::Rgb => {
                frame_buffer_tag.bits_per_pixel = 32;
                frame_buffer_tag.red_byte = 0;
                frame_buffer_tag.green_byte = 1;
                frame_buffer_tag.blue_byte = 2;
            }
            PixelFormat::Bitmask => {
                todo!();
            }
            PixelFormat::BltOnly => {
                panic!("BltOnly pixel format is not supported");
            }
        }
        frame_buffer_tag.pitch =
            graphics.mode.stride() as u32 * frame_buffer_tag.bits_per_pixel / 8;
    }

    let mut page_allocator = |page_count| {
        boot_services
            .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, page_count)
            .map(|address| address as *mut u8)
            .ok()
    };

    let entrypoint = elf.entrypoint as usize;
    let stack_tag = tags
        .stack_pointer
        .expect("Stack tag not found in kernel binary")
        .clone();

    let memory_map_tag_offset = tags.memory_map_offset;
    let mut final_memory_map_tag = None;

    let mut page_tables = arch::PageTables::new(&mut page_allocator);

    for segment in elf.loadable_segments {
        let allocated_memory = page_allocator
            .allocate(arch::page_align_up(segment.size_in_memory) / arch::PAGE_SIZE)
            .unwrap();
        unsafe {
            // Copy the bytes.
            let src = kernel_binary.as_ptr().add(segment.file_offset);
            allocated_memory.copy_from(src, segment.size_in_file);
            // And zero out the rest.
            allocated_memory
                .add(segment.size_in_file)
                .write_bytes(0, segment.size_in_memory - segment.size_in_file);

            if segment.file_offset == beryllium_section.file_offset {
                if let Some(memory_map_tag_offset) = memory_map_tag_offset {
                    final_memory_map_tag = Some(
                        &mut *(allocated_memory.add(16 + memory_map_tag_offset)
                            as *mut MemoryMapTag),
                    );
                }
            }
        }
        page_tables.map(
            &mut page_allocator,
            segment.virtual_address,
            allocated_memory as usize,
            page_align_up(segment.size_in_memory),
            segment.writable,
            segment.executable,
        );
    }

    // Since we have to exit boot services to get the memory map, but we need to allocate memory to store the memory map first, we just hope this is enough space.
    // It should be fine because the entries are 24 bytes each, so we can store 170 entries in 4 KiB.
    let memory_map_storage = unsafe {
        slice::from_raw_parts_mut(
            boot_services
                .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1)
                .unwrap() as *mut u8,
            PAGE_SIZE,
        )
    };
    page_tables.map(
        &mut page_allocator,
        memory_map_virtual_address,
        memory_map_storage.as_ptr() as usize,
        memory_map_storage.len(),
        false,
        false,
    );

    let (_runtime_table, mut memory_map) = system_table.exit_boot_services();
    memory_map.sort();

    let mut memory_map_storage_iterator = memory_map_storage
        .chunks_exact_mut(size_of::<MemoryMapEntry>())
        .map::<&mut MemoryMapEntry, _>(|slice| TryFrom::try_from(slice).unwrap());
    let mut previous_storage_entry: Option<&mut MemoryMapEntry> = None;
    for memory_map_entry in memory_map.entries() {
        let memory_type = match memory_map_entry.ty {
            MemoryType::CONVENTIONAL => MemoryMapEntryType::Available,
            MemoryType::LOADER_CODE => MemoryMapEntryType::Kernel,
            MemoryType::LOADER_DATA => MemoryMapEntryType::Kernel,
            MemoryType::BOOT_SERVICES_CODE => MemoryMapEntryType::Available,
            MemoryType::BOOT_SERVICES_DATA => MemoryMapEntryType::Available,
            MemoryType::RUNTIME_SERVICES_CODE => MemoryMapEntryType::EfiRuntime,
            MemoryType::RUNTIME_SERVICES_DATA => MemoryMapEntryType::EfiRuntime,
            MemoryType::ACPI_RECLAIM => MemoryMapEntryType::AcpiReclaimable,
            _ => MemoryMapEntryType::Reserved,
        };
        if let Some(ref mut previous_storage_entry) = previous_storage_entry {
            if previous_storage_entry.memory_type == memory_type
                && previous_storage_entry.address as u64 + previous_storage_entry.size as u64
                    == memory_map_entry.phys_start
            {
                previous_storage_entry.size += memory_map_entry.page_count as usize * PAGE_SIZE;
                continue;
            }
        }
        if let Some(memory_map_storage_entry) = memory_map_storage_iterator.next() {
            *memory_map_storage_entry = MemoryMapEntry {
                address: memory_map_entry.phys_start as *mut u8,
                size: memory_map_entry.page_count as usize * PAGE_SIZE,
                memory_type,
            };
            previous_storage_entry = Some(memory_map_storage_entry);
        } else {
            break;
        }
    }

    let remaining_memory_map_storage_entries = memory_map_storage_iterator.count();
    let used_memory_map_storage_entries = memory_map_storage.len() / size_of::<MemoryMapEntry>()
        - remaining_memory_map_storage_entries;

    if let Some(memory_map_tag) = final_memory_map_tag {
        memory_map_tag.base = memory_map_virtual_address as *mut u8;
        memory_map_tag.memory_size = memory_map
            .entries()
            .len()
            .max(used_memory_map_storage_entries)
            * size_of::<MemoryMapEntry>();
    }

    arch::enter_kernel(
        entrypoint,
        stack_tag.base as usize,
        stack_tag.memory_size,
        &mut page_tables,
    );
}
