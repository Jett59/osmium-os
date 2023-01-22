#![no_main]
#![no_std]
#![feature(abi_efiapi)]
#![allow(stable_features)]

extern crate alloc;
use alloc::{vec, string::String};
use uefi::{prelude::*, proto::{media::file::{FileMode, FileAttribute, FileInfo, File}}};
use uefi::{Result};

#[entry]
fn main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    uefi_services::println!("Hello, Osmium World!");

    let boot_services = system_table.boot_services();
    let result = read_config(image, boot_services);
    if result != Result::Ok(()) {
        uefi_services::println!("Error: {:?}", result);
    }

    loop {}
}

fn read_config(image: Handle, boot_services: &BootServices) -> Result {
    uefi_services::println!("Reading config file");
    let mut fs = boot_services.get_image_file_system(image)?;
    let mut root = fs.open_volume()?;
    let mut file = root.open(cstr16!("\\boot\\osmium\\boot.toml"), FileMode::Read, FileAttribute::empty())?.into_regular_file().unwrap();
    
    let mut info_buffer = vec![0; 128];
    let file_info = file.get_info::<FileInfo>(&mut info_buffer).unwrap();
    let file_size = file_info.file_size() as usize;

    let mut buffer = vec![0u8; file_size];
    let _size = file.read(&mut buffer).unwrap();
    let config = String::from_utf8_lossy(&buffer);
    uefi_services::println!("{config}");

    return Ok(());
}