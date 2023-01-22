#![no_main]
#![no_std]
#![feature(abi_efiapi)]
#![allow(stable_features)]

use uefi::{prelude::*, proto::{loaded_image::{LoadedImage, self}, device_path::{DevicePath}}};

#[entry]
fn main(handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    uefi_services::println!("Hello, Osmium World!");

    let boot_services = system_table.boot_services();
    let loaded_image = boot_services.open_protocol_exclusive::<LoadedImage>(handle).unwrap();

    let device_path = boot_services.open_protocol_exclusive::<DevicePath>(loaded_image.device()).unwrap();
    for device in device_path.node_iter() {
        uefi_services::println!("Device Node: {:?}", device);
    }
    for device_instance in device_path.instance_iter() {
        uefi_services::println!("Device Instance: {:?}", device_instance);
    }

    loop {}
}
