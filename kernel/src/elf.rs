use core::slice;

use common::elf::ElfBinary;

use crate::user_memory::allocate_user_memory_at;

pub fn map_sections(elf: &ElfBinary, file: &[u8]) {
    for loadable_segment in &elf.loadable_segments {
        allocate_user_memory_at(
            loadable_segment.virtual_address,
            loadable_segment.size_in_memory,
        );
        // TODO: There must be a cleaner way than this.
        let bytes = unsafe {
            slice::from_raw_parts_mut(
                loadable_segment.virtual_address as *mut u8,
                loadable_segment.size_in_memory,
            )
        };
        bytes.copy_from_slice(
            &file[loadable_segment.file_offset
                ..loadable_segment.file_offset + loadable_segment.size_in_file],
        );
        bytes[loadable_segment.size_in_file..].fill(0);
    }
}
