use core::{mem::size_of, ptr::null};

use common::beryllium::{BootRequestTagType, ModuleTag};

#[link_section = ".beryllium"]
#[no_mangle]
pub static mut MODULE_TAG: ModuleTag = ModuleTag {
    tag_type: BootRequestTagType::Module,
    size: size_of::<ModuleTag>() as u16,
    flags: 0,
    base: null(),
    file_size: 0,
};

pub fn get_initramfs() -> Option<&'static [u8]> {
    // SAFETY: The module tag is initialized by the bootloader, and never modified afterwards.
    if unsafe { MODULE_TAG }.file_size == 0 {
        return None;
    }

    // SAFETY: We trust the bootloader to give us a valid pointer and a correct size.
    unsafe {
        let base = MODULE_TAG.base;
        let size = MODULE_TAG.file_size;
        Some(core::slice::from_raw_parts(base, size))
    }
}
