#![no_main]
#![no_std]
#![feature(abi_efiapi)]
#![allow(stable_features)]

use uefi::{prelude::*, proto::loaded_image::LoadedImage};

#[entry]
fn main(handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    uefi_services::println!("Hello, Osmium World!");

    let boot_services = system_table.boot_services();
    let loaded_image = boot_services.open_protocol_exclusive::<LoadedImage>(handle).unwrap();
    let (_, info) = loaded_image.info();

    uefi_services::println!("Loaded Image: {info}");

    loop {}
}
