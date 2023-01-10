use core::{alloc::GlobalAlloc, ptr::null_mut};

use crate::{
    assert::const_assert,
    buddy::BuddyAllocator,
    lazy_init::lazy_static,
    paging::{map_block, unmap_block},
    pmm::{BLOCK_SIZE, LOG2_BLOCK_SIZE},
};

#[cfg(target_arch = "x86_64")]
const VIRTUAL_HEAP_START: usize = 0xffffa00000000000;

#[cfg(target_arch = "x86_64")]
const HEAP_SIZE: usize = 0x1000000000; // 64GB

const LOG2_HEAP_SIZE: u8 = HEAP_SIZE.trailing_zeros() as u8;

const_assert!(
    1 << LOG2_HEAP_SIZE == HEAP_SIZE,
    "HEAP_SIZE must be a power of two"
);

lazy_static! {
    static ref HEAP_VIRTUAL_MEMORY_ALLOCATOR: &mut BuddyAllocator<256, LOG2_HEAP_SIZE, LOG2_BLOCK_SIZE> = {
        static mut REAL_ALLOCATOR: BuddyAllocator<256, LOG2_HEAP_SIZE, LOG2_BLOCK_SIZE> =
            BuddyAllocator::unusable();
        unsafe {
            REAL_ALLOCATOR
                .all_unused()
                .add_entry(HEAP_SIZE, VIRTUAL_HEAP_START);
            &mut REAL_ALLOCATOR
        }
    };
}

struct HeapAllocator;

unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // If the allocation is for less than the size of a block, we use a different algorithm.
        // Otherwise we simply allocate the required amount of virtual memory and map it to freshly allocated physical memory.
        let size = layout.size().next_power_of_two();
        // Buddy allocators align memory to its size, so we shouldn't have to worry about alignment here.
        assert!(layout.align() <= size);
        if size >= BLOCK_SIZE {
            let address = HEAP_VIRTUAL_MEMORY_ALLOCATOR.allocate(size);
            if let Some(address) = address {
                for virtual_block_address in (address..(address + size)).step_by(BLOCK_SIZE) {
                    let physical_block_address = crate::pmm::allocate_block_address();
                    if let Some(physical_address) = physical_block_address {
                        map_block(virtual_block_address, physical_address);
                    } else {
                        return null_mut();
                    }
                }
                return address as *mut u8;
            } else {
                return null_mut();
            }
        } else {
            todo!();
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let address = ptr as usize;
        let size = layout.size().next_power_of_two();
        // Same logic as above.
        if size >= BLOCK_SIZE {
            for block_address in (address..(address + size)).step_by(BLOCK_SIZE) {
                unmap_block(block_address);
            }
            HEAP_VIRTUAL_MEMORY_ALLOCATOR.free(size, address);
        } else {
            todo!();
        }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: HeapAllocator = HeapAllocator {};
