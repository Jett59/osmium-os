use core::mem::size_of;

use crate::{
    arch_api::console,
    memory::{reinterpret_memory, slice_from_memory, Validateable},
};

#[repr(C, packed)]
struct MbiHeader {
    total_size: u32,
    reserved: u32,
}

impl Validateable for MbiHeader {
    fn validate(&self) -> bool {
        // We must be at least 8 bytes and aligned to an 8-byte boundary.
        self.total_size >= 8 && self.total_size % 8 == 0
    }
}

extern "C" {
    static mbi_pointer: *const u8;
}

pub fn parse_multiboot_structures() {
    let mbi_header: &MbiHeader = unsafe {
        reinterpret_memory(slice_from_memory(mbi_pointer, size_of::<MbiHeader>()).unwrap()).unwrap()
    };
    
}
