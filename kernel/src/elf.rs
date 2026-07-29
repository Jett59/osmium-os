use common::elf::{ElfBinary, LoadableSegment};

use crate::{
    paging::{MemoryType, PAGE_SIZE, PagePermissions, change_permissions},
    unsafe_impl::memory_token::{AllocatedMemoryToken, MemoryToken},
    user_memory::{UserAddressSpaceHandle, allocate_user_memory_at},
};

/// # Safety
/// If the virtual address of the segment hasn't been mapped, this will do something weird (probably page fault, but who knows).
unsafe fn copy_elf_section(
    loadable_segment: &LoadableSegment,
    file: &[u8],
    address_space: &UserAddressSpaceHandle,
) {
    let memory = address_space.memory(
        loadable_segment.virtual_address,
        loadable_segment.size_in_memory,
    );
    memory.write_part(
        0,
        &file[loadable_segment.file_offset
            ..loadable_segment.file_offset + loadable_segment.size_in_file],
    );
    memory.write_bytes(
        loadable_segment.size_in_file,
        0,
        loadable_segment.size_in_memory - loadable_segment.size_in_file,
    );
}

pub fn map_sections(elf: &ElfBinary, file: &[u8], address_space: &UserAddressSpaceHandle) {
    for loadable_segment in &elf.loadable_segments {
        allocate_user_memory_at(
            loadable_segment.virtual_address,
            loadable_segment.size_in_memory,
            PagePermissions::KERNEL_READ_WRITE, // Allows us to write the contents first.
        );
        unsafe { copy_elf_section(loadable_segment, file, address_space) };
        // Now set the permissions
        change_permissions(
            // SAFETY: This is absolutely not safe.
            unsafe {
                AllocatedMemoryToken::new(
                    loadable_segment.virtual_address,
                    loadable_segment.size_in_memory.next_multiple_of(PAGE_SIZE),
                )
            },
            MemoryType::Normal,
            PagePermissions::new(true, loadable_segment.writable, loadable_segment.executable),
        );
    }
}
