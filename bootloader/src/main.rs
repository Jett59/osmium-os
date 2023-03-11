#![no_main]
#![no_std]

mod beryllium;
mod config;
mod elf;
mod toml;

extern crate alloc;

use core::ptr::null_mut;

use alloc::vec;
use alloc::vec::Vec;
use config::Config;
use uefi::{
    prelude::*,
    proto::{media::file::{File, FileAttribute, FileInfo, FileMode}, device_path::text::DevicePathToText, console::gop::{GraphicsOutput, ModeInfo, FrameBuffer, PixelFormat}}, table::boot::{self, OpenProtocolAttributes, OpenProtocolParams},
};
use uefi::{CStr16, Result};
use uefi_services::println;

use crate::config::parse_config;


struct GraphicsInfo {
    mode: ModeInfo,
    frame_buffer_ptr: *mut u8
}

//Function to get the handle for the graphics output protocol
fn graphics(image: Handle, boot_services: &BootServices) -> Result<GraphicsInfo> {
    let handle = boot_services.get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut graphics_output_protocol = unsafe {
        boot_services.open_protocol::<GraphicsOutput>(
            OpenProtocolParams {
                handle,
                agent: image,
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }.unwrap();

    //query modes
    let modes = graphics_output_protocol.modes();
    println!("Number of modes: {}", modes.len());
    //iterate
    for mode in modes.filter(|mode| mode.info().pixel_format() != PixelFormat::BltOnly) {
        println!("Mode: {:?}", mode.info());
    }
    
    let mode = graphics_output_protocol.current_mode_info();
    // let mut frame_buffer = graphics_output_protocol.frame_buffer();
    // let frame_buffer_ptr = frame_buffer.as_mut_ptr();

    Ok(GraphicsInfo { mode, frame_buffer_ptr: null_mut() })
}

#[entry]
fn main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    system_table.stdout().clear().unwrap();

    let boot_services = system_table.boot_services();

    graphics(image, boot_services).unwrap();

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

    load_kernel(image, boot_services, entry.kernel_path.as_str()).unwrap();

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
fn load_kernel(image: Handle, boot_services: &BootServices, path: &str) -> Result {
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
    // println!("Tags: {:#?}", tags);

    Ok(())
}
