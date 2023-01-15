use core::{alloc::GlobalAlloc, intrinsics::size_of, ptr::null_mut};

use alloc::boxed::Box;

use crate::{
    assert::const_assert,
    buddy::BuddyAllocator,
    lazy_init::lazy_static,
    paging::{get_physical_address, map_block, unmap_block},
    pmm::{mark_as_free, BLOCK_SIZE, LOG2_BLOCK_SIZE},
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

#[derive(Clone, Copy)]
struct SlabUnusedEntry {
    // Indices are u16::MAX for non-existant.
    next_index: u16,
    previous_index: u16,
}

#[derive(Clone, Copy)]
struct SlabHeadEntry<const SIZE: usize> {
    next_of_this_size: *mut SlabEntry<SIZE>,
    previous_of_this_size: *mut SlabEntry<SIZE>,
    first_unused_entry: u16,
}

#[repr(C)] // Make sure the 'data' field is at offset 0
union SlabEntry<const SIZE: usize> {
    data: [u8; SIZE],
    unused: SlabUnusedEntry,
    head: SlabHeadEntry<SIZE>,
}

const MIN_SLAB_ENTRY_SIZE: usize = size_of::<SlabEntry<1>>().next_power_of_two();

struct SlabAllocator {
    partial_lists: [Option<*mut SlabEntry<MIN_SLAB_ENTRY_SIZE>>; LOG2_HEAP_SIZE as usize],
    // The empty ones are removed immediately and so are the full ones, so we just need to keep track of the partials.
}

// TODO: Add locking to the allocator so that this is actually safe.
unsafe impl Sync for SlabAllocator {}

struct HeapAllocator {
    slab_allocator: SlabAllocator,
}

#[cfg(not(test))]
#[global_allocator]
static mut GLOBAL_ALLOCATOR: HeapAllocator = HeapAllocator {
    slab_allocator: SlabAllocator::new(),
};

impl SlabAllocator {
    const fn new() -> Self {
        Self {
            partial_lists: [None; LOG2_HEAP_SIZE as usize],
        }
    }

    fn allocate_entry_block<const SIZE: usize>() -> *mut SlabEntry<SIZE> {
        unsafe {
            // Rust doesn't let us use any kind of allocator api or anything, so this is the best I can think of.
            // It is a bit of repetition, but it's not too bad.
            let virtual_address = HEAP_VIRTUAL_MEMORY_ALLOCATOR.allocate(BLOCK_SIZE);
            if let Some(virtual_address) = virtual_address {
                let physical_address = crate::pmm::allocate_block_address();
                if let Some(physical_address) = physical_address {
                    map_block(virtual_address, physical_address);
                    return virtual_address as *mut SlabEntry<SIZE>;
                }
            }
            panic!("Out of memory allocating slab entry block");
        }
    }

    /// This function is to initialize the head entry of the list and assumes that there were no entries before (so it is only really useful for creating an entry when the list is empty).
    fn initialize_entry_list<const SIZE: usize>(entry_list: *mut SlabEntry<SIZE>) {
        let entry_count = BLOCK_SIZE / SIZE;
        let entries = unsafe { core::slice::from_raw_parts_mut(entry_list, entry_count) };
        entries[0].head = SlabHeadEntry {
            next_of_this_size: null_mut(),
            previous_of_this_size: null_mut(),
            first_unused_entry: 1,
        };
        for i in 1..entry_count {
            entries[i].unused = SlabUnusedEntry {
                next_index: (i + 1) as u16,
                previous_index: (i - 1) as u16,
            };
        }
    }

    fn get_partial_list<const SIZE: usize>(&mut self) -> *mut SlabEntry<SIZE> {
        let index = SIZE.trailing_zeros();
        if let Some(partial_list) = self.partial_lists[index as usize] {
            // TODO: I don't think the intermediary cast should be necessary (maybe a compiler bug?)
            partial_list as *mut u8 as *mut SlabEntry<SIZE>
        } else {
            let result = Self::allocate_entry_block::<SIZE>();
            Self::initialize_entry_list(result);
            self.partial_lists[index as usize] =
                Some(result as *mut u8 as *mut SlabEntry<MIN_SLAB_ENTRY_SIZE>);
            result
        }
    }
}

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
                address as *mut u8
            } else {
                null_mut()
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
                mark_as_free(get_physical_address(block_address));
                unmap_block(block_address);
            }
            HEAP_VIRTUAL_MEMORY_ALLOCATOR.free(size, address);
        } else {
            todo!();
        }
    }
}

// It's rather difficult to use the unit testing here since this bit depends rather a lot on paging which we can't manage very well in a hosted environment.

pub fn sanity_check() {
    // The one thing we can't let happen is the optimizer to optimize out these checks, which would be trivial to do.
    // That is why we use #[inline(never)] all over the place.
    #[inline(never)]
    fn allocate_it<const SIZE: usize>() -> Box<[u8; SIZE]> {
        unsafe { Box::new_zeroed().assume_init() }
    }
    #[inline(never)]
    fn check_it<const SIZE: usize>(value: *mut u8) {
        let address = value as usize;
        assert!(address >= VIRTUAL_HEAP_START);
        assert!(address + SIZE <= VIRTUAL_HEAP_START + HEAP_SIZE);
        // We should touch all of the memory to make sure it is all accessible (and writeable).
        unsafe {
            for i in 0..SIZE {
                value.add(i).write_volatile(0);
            }
            for i in 0..SIZE {
                assert!(value.add(i).read_volatile() == 0);
            }
        }
    }
    // Allocate 1mb (for the large allocations case).
    check_it::<0x100000>(allocate_it::<0x100000>().as_mut_ptr());
    // TODO: Uncomment when this feature is implemented.
    // // Allocate 1kb (for the small allocations case).
    // let value = allocate_it::<0x400>();
    // check_it(&value);
}
