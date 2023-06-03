#![no_main]
#![no_std]

#[cfg_attr(target_arch = "aarch64", path = "aarch64/mod.rs")]
mod arch;
mod beryllium;
mod config;
mod elf;
mod toml;

extern crate alloc;

use core::slice;

use alloc::vec;
use alloc::vec::Vec;
use config::Config;
use uefi::{
    prelude::*,
    proto::{
        console::gop::{GraphicsOutput, ModeInfo, PixelFormat},
        media::file::{File, FileAttribute, FileInfo, FileMode},
    },
    table::boot::{AllocateType, MemoryType, OpenProtocolAttributes, OpenProtocolParams},
};
use uefi::{CStr16, Result};
use uefi_services::println;

use crate::{arch::PageAllocator, config::parse_config};

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
    let mut root = fs.open_volume()?;
    let mut file = root
        .open(name, FileMode::Read, FileAttribute::empty())?
        .into_regular_file()
        .unwrap();

    let mut info_buffer = vec![0; 128];
    let file_info = file.get_info::<FileInfo>(&mut info_buffer).unwrap();
    let file_size = file_info.file_size() as usize;

    let mut buffer = vec![0u8; file_size];
    file.read(&mut buffer).unwrap();

    Ok(buffer)
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
        &CStr16::from_str_with_buf(path.as_str(), path_buffer.as_mut_slice()).unwrap(),
    )?;
    let elf = elf::load_elf(kernel_binary.as_slice()).unwrap();
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
        println!("Graphics mode: {:?}", graphics.mode);
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

    let mut page_tables = arch::PageTables::new(&mut page_allocator);

    for segment in elf.loadable_segments {
        let allocated_memory = page_allocator
            .allocate(arch::page_align_up(segment.size_in_memory) / arch::PAGE_SIZE)
            .unwrap();
        unsafe {
            // Copy the bytes.
            let src = kernel_binary.as_ptr().add(segment.file_offset as usize);
            allocated_memory.copy_from(src, segment.size_in_file);
            // And zero out the rest.
            allocated_memory
                .add(segment.size_in_file)
                .write_bytes(0, segment.size_in_memory - segment.size_in_file);
        }

        page_tables.map(
            &mut page_allocator,
            segment.virtual_address,
            allocated_memory as usize,
            segment.size_in_memory,
            segment.writable,
            segment.executable,
        );
    }

    // Allocate space for the memory map.
    // It says that we should allocate more space than we need just in case it grows in the meantime, so we do twice as much as we have been told.
    let memory_map_size = boot_services.memory_map_size().map_size * 2;
    let memory_map = page_allocator
        .allocate(arch::page_align_up(memory_map_size) / arch::PAGE_SIZE)
        .unwrap();

    unsafe {
        system_table
            .exit_boot_services(
                image,
                slice::from_raw_parts_mut(memory_map, memory_map_size),
            )
            .unwrap();
    }

    arch::enter_kernel(
        entrypoint,
        stack_tag.base as usize,
        stack_tag.memory_size,
        &page_tables,
    );
}
