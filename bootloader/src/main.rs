#![no_main]
#![no_std]
#![feature(abi_efiapi)]
#![allow(stable_features)]

mod config;

extern crate alloc;

use alloc::{vec, string::String};
use uefi::{prelude::*, proto::{media::file::{FileMode, FileAttribute, FileInfo, File}}};
use uefi::{Result};

use crate::config::config_from_str;
use crate::config::Config;

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
    let config_string = String::from_utf8(buffer).unwrap();
    let config: Config = config_from_str(config_string).unwrap();

    let kernel_version = config.kernel.version;
    uefi_services::println!("Kernel Version: {kernel_version}");

    return Ok(());
}