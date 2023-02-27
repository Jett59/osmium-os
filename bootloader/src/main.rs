#![no_main]
#![no_std]

mod config;
mod elf;
mod toml;

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use config::Config;
use uefi::{
    prelude::*,
    proto::media::file::{File, FileAttribute, FileInfo, FileMode},
};
use uefi::{CStr16, Result};
use uefi_services::println;

use crate::config::parse_config;

#[cfg_attr(not(test), panic_handler)]
fn panic(info: &core::panic::PanicInfo) -> ! {
    uefi_services::println!("{}", info);
    loop {}
}

#[entry]
fn main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    system_table.stdout().clear().unwrap();

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
    let kernel_binary = read_file(
        image,
        boot_services,
        &CStr16::from_str_with_buf(path.as_str(), path_buffer.as_mut_slice()).unwrap(),
    )?;
    let elf = elf::load_elf(kernel_binary.as_slice()).unwrap();
    uefi_services::println!("Kernel elf: {:?}", elf);
    //print the bytes in the beryllium section - find the section by iterating and filtering on name
    let beryllium_section = elf
        .sections
        .iter()
        .find(|section| section.name == ".beryllium")
        .expect("Beryllium signature not found");
    assert!(
        beryllium_section.size >= 16,
        "Beryllium signature not found"
    );
    let beryllium_bytes = &kernel_binary
        [beryllium_section.file_offset as usize..(beryllium_section.file_offset + 16) as usize];
    assert!(
        beryllium_bytes == b"Beryllium Ready!",
        "Beryllium signature not found"
    );
    uefi_services::println!("{}", core::str::from_utf8(beryllium_bytes).unwrap());

    Ok(())
}
