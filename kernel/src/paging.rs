pub use crate::arch_api::paging::{get_physical_address, map_page, unmap_page, PAGE_SIZE};
use crate::physical_memory_manager::PAGES_PER_BLOCK;

pub fn map_block(virtual_address: usize, physical_address: usize) {
    for i in 0..PAGES_PER_BLOCK {
        map_page(
            virtual_address + i * PAGE_SIZE,
            physical_address + i * PAGE_SIZE,
        );
    }
}

pub fn unmap_block(virtual_address: usize) {
    for i in 0..PAGES_PER_BLOCK {
        unmap_page(virtual_address + i * PAGE_SIZE);
    }
}
