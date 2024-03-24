use crate::{
    arch_api::paging::{is_valid_user_address, MemoryType},
    paging::map_block,
    physical_memory_manager::{allocate_block_address, BLOCK_SIZE},
};

pub fn allocate_user_memory_at(virtual_address: usize, size: usize) {
    assert_eq!(
        virtual_address % BLOCK_SIZE,
        0,
        "virtual_address must be BLOCK_SIZE aligned"
    );
    assert!(
        is_valid_user_address(virtual_address),
        "Invalid virtual address {}",
        virtual_address
    );

    for virtual_block_address in (virtual_address..virtual_address + size).step_by(BLOCK_SIZE) {
        let physical_address = allocate_block_address().expect("Out of memory");
        map_block(virtual_block_address, physical_address, MemoryType::Normal);
    }
}
