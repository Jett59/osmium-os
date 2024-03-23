use core::{mem::size_of, ptr::null};

use common::beryllium::{BootRequestTagType, InitialRamdiskTag};

#[cfg_attr(not(test), link_section = ".beryllium")]
#[no_mangle]
pub static mut INITIAL_RAMDISK_TAG: InitialRamdiskTag = InitialRamdiskTag {
    tag_type: BootRequestTagType::InitialRamdisk,
    size: size_of::<InitialRamdiskTag>() as u16,
    flags: 0,
    base: null(),
    file_size: 0,
};

pub fn get_initial_ramdisk() -> Option<&'static [u8]> {
    // SAFETY: The module tag is initialized by the bootloader, and never modified afterwards.
    if unsafe { INITIAL_RAMDISK_TAG }.file_size == 0 {
        return None;
    }

    // SAFETY: We trust the bootloader to give us a valid pointer and a correct size.
    unsafe {
        let base = INITIAL_RAMDISK_TAG.base;
        let size = INITIAL_RAMDISK_TAG.file_size;
        Some(core::slice::from_raw_parts(base, size))
    }
}
