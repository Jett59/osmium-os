use core::slice;

use common::elf::ElfBinary;

use crate::{
    paging::{change_block_permissions, MemoryType, PagePermissions},
    user_memory::allocate_user_memory_at,
};

pub fn map_sections(elf: &ElfBinary, file: &[u8]) {
    for loadable_segment in &elf.loadable_segments {
        allocate_user_memory_at(
            loadable_segment.virtual_address,
            loadable_segment.size_in_memory,
            PagePermissions::READ_WRITE, // Allows us to write the contents first.
        );
        // TODO: There must be a cleaner way than this.
        let bytes = unsafe {
            slice::from_raw_parts_mut(
                loadable_segment.virtual_address as *mut u8,
                loadable_segment.size_in_memory,
            )
        };
        bytes[..loadable_segment.size_in_file].copy_from_slice(
            &file[loadable_segment.file_offset
                ..loadable_segment.file_offset + loadable_segment.size_in_file],
        );
        bytes[loadable_segment.size_in_file..].fill(0);
        // Now set the permissions
        change_block_permissions(
            loadable_segment.virtual_address,
            MemoryType::Normal,
            PagePermissions::new(true, loadable_segment.writable, loadable_segment.executable),
        );
    }
}
