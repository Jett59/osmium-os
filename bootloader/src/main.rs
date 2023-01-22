#![no_main]
#![no_std]
#![feature(abi_efiapi)]
#![allow(stable_features)]

use uefi::{prelude::*, proto::{loaded_image::{LoadedImage, self}, device_path::{DevicePath, DevicePathNodeEnum}}, table::boot::ScopedProtocol};

#[entry]
fn main(handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    uefi_services::println!("Hello, Osmium World!");

    let boot_services = system_table.boot_services();
    let loaded_image = boot_services.open_protocol_exclusive::<LoadedImage>(handle).unwrap();

    discover_devices(loaded_image, boot_services);

    loop {}
}

fn discover_devices(loaded_image: ScopedProtocol<LoadedImage>, boot_services: &BootServices) {
    let device_path = boot_services.open_protocol_exclusive::<DevicePath>(loaded_image.device()).unwrap();
    for device_node in device_path.node_iter() {
        let device_node_enum = device_node.as_enum().unwrap();
        match device_node_enum {
            DevicePathNodeEnum::AcpiAcpi(value)=> {
                uefi_services::println!("ACPI ACPI {:?}", value);
            },
            DevicePathNodeEnum::AcpiAdr(value)=> {
                uefi_services::println!("ACPI ADR {:?}", value);
            },
            DevicePathNodeEnum::AcpiExpanded(value)=> {
                uefi_services::println!("ACPI Expanded {:?}", value);
            },
            DevicePathNodeEnum::AcpiNvdimm(value)=> {
                uefi_services::println!("ACPI Nvdimm {:?}", value);
            },
            _ => {
                uefi_services::println!("Device Node: {:?}", device_node);
            }
        }        
    }
    for device_instance in device_path.instance_iter() {
        uefi_services::println!("Device Instance: {:?}", device_instance);
    }

}