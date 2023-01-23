#![no_main]
#![no_std]
#![feature(abi_efiapi)]
#![allow(stable_features)]

mod config;
mod toml;

extern crate alloc;

use alloc::vec;
use config::Config;
use uefi::Result;
use uefi::{
    prelude::*,
    proto::media::file::{File, FileAttribute, FileInfo, FileMode},
};
use uefi_services::println;

use crate::config::parse_config;

#[entry]
fn main(image: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    system_table.stdout().clear().unwrap();

    uefi_services::println!("Hello, Osmium World!");

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

    println!("TODO: Load Kernel Now!!");

    loop {}
}

fn read_config(image: Handle, boot_services: &BootServices) -> Result<Config> {
    uefi_services::println!("Reading config file");
    let mut fs = boot_services.get_image_file_system(image)?;
    let mut root = fs.open_volume()?;
    let mut file = root
        .open(
            cstr16!("\\boot\\osmium\\boot.toml"),
            FileMode::Read,
            FileAttribute::empty(),
        )?
        .into_regular_file()
        .unwrap();

    let mut info_buffer = vec![0; 128];
    let file_info = file.get_info::<FileInfo>(&mut info_buffer).unwrap();
    let file_size = file_info.file_size() as usize;

    let mut buffer = vec![0u8; file_size];
    let _size = file.read(&mut buffer).unwrap();
    let config_string = core::str::from_utf8(&buffer).unwrap();
    let config = parse_config(config_string);

    Ok(config)
}
