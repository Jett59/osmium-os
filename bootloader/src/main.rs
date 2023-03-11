#![no_main]
#![no_std]

mod beryllium;
mod config;
mod elf;
mod toml;

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use config::Config;
use uefi::{
    prelude::*,
    proto::{media::file::{File, FileAttribute, FileInfo, FileMode}, device_path::text::DevicePathToText, console::gop::GraphicsOutput}, table::boot::{self, OpenProtocolAttributes, OpenProtocolParams},
};
use uefi::{CStr16, Result};
use uefi_services::println;

use crate::config::parse_config;

//Function to get the mode from the graphics output protocol
fn get_mode(graphicsOutputProtocol: &GraphicsOutput) -> Result {
    let mode = graphicsOutputProtocol.current_mode_info();
    println!("Got handle {:#?}", mode);
    Ok(())
}

//Function to get the frame buffer from the graphics output protocol
fn get_frame_buffer(graphicsOutputProtocol: &GraphicsOutput) -> Result {
    // let frame_buffer = graphicsOutputProtocol.frame_buffer();
    // println!("Got handle {:#?}", frame_buffer);
    Ok(())
}

//Function to get the handle for the graphics output protocol
fn graphics(image: Handle, boot_services: &BootServices) -> Result {
    let handle = boot_services.get_handle_for_protocol::<GraphicsOutput>().unwrap();
    println!("Got handle {:#?}", handle);
    //open the graphics output protocol
    println!("About to open protocol");
    //open the graphics output protocol
    let graphicsOutputProtocol = unsafe {
        boot_services.open_protocol::<GraphicsOutput>(
            OpenProtocolParams {
                handle,
                agent: image,
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }.unwrap();
    println!("Opened protocol");

    //get the mode
    get_mode(&graphicsOutputProtocol).unwrap();
    //get the frame buffer
    get_frame_buffer(&graphicsOutputProtocol).unwrap();

    Ok(())
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
