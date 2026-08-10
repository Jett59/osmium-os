use core::{
    alloc::{GlobalAlloc, Layout},
    cmp::min,
    ptr::{self, null_mut},
};

use crate::{
    assert::const_assert,
    memory::align_address_down,
    memory_allocator::{alloc_large, free_large},
    physical_memory_manager::{BLOCK_SIZE, LOG2_BLOCK_SIZE},
    unsafe_impl::{
        memory_token::{AllocatedMemoryToken, AllocatedRwToken, MemoryToken},
        slab::{DynamicSizedSlab, MIN_SLAB_ENTRY_SIZE, Slab},
    },
};

const MIN_SMALL_SIZE: usize = MIN_SLAB_ENTRY_SIZE.next_power_of_two();

struct GlobalAllocator;

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // We require `size` to be a power of 2.
        let size = layout.size().max(MIN_SMALL_SIZE).next_power_of_two();
        // Allocations smaller than BLOCK_SIZE / 2 are handled by the slab allocator, and larger allocations are handled by the large allocator.
        if size <= BLOCK_SIZE / 2 {
            let allocation = alloc_small(size);
            if let Some(allocation) = allocation {
                assert_eq!(
                    allocation.size(),
                    size,
                    "Slab allocator returned a block of the wrong size"
                );
                allocation.into_ptr()
            } else {
                null_mut()
            }
        } else {
            let allocation = alloc_large(size);
            if let Some(allocation) = allocation {
                assert_eq!(
                    allocation.size(),
                    size,
                    "Large allocator returned a block of the wrong size"
                );
                allocation.into_ptr()
            } else {
                null_mut()
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(MIN_SMALL_SIZE).next_power_of_two();
        if size <= BLOCK_SIZE / 2 {
            // SAFETY: see `alloc`: the pointer here must have originated in that function.
            let allocated_token = unsafe { AllocatedRwToken::new(ptr as usize, size) };
            // SAFETY: The token must have been created by `alloc_small`.
            unsafe { free_small(allocated_token) };
        } else {
            let allocated_token = unsafe { AllocatedRwToken::new(ptr as usize, size) };
            free_large(allocated_token);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // If the new size rounds up to the same power of 2, we can just return the same pointer.
        let old_size = layout.size().next_power_of_two();
        let new_size = new_size.next_power_of_two();
        if old_size == new_size {
            ptr
        } else {
            // Allocate new memory, copy the old data, and free the old memory.
            let new_ptr =
                unsafe { self.alloc(Layout::from_size_align(new_size, layout.align()).unwrap()) };
            if !new_ptr.is_null() {
                unsafe {
                    ptr::copy_nonoverlapping(ptr, new_ptr, min(old_size, new_size));
                    self.dealloc(ptr, layout);
                };
            }
            new_ptr
        }
    }
}

#[cfg_attr(not(test), global_allocator)]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

// TODO: is there a better place for the small allocator?

struct SmallAllocator {
    partial_lists: [Option<DynamicSizedSlab>; LOG2_BLOCK_SIZE as usize],
}

static SMALL_ALLOCATOR: spin::Mutex<SmallAllocator> = spin::Mutex::new(SmallAllocator {
    partial_lists: [const { None }; LOG2_BLOCK_SIZE as usize],
});

// The below code must be updated if these variables change.
const_assert!(MIN_SMALL_SIZE == 32, "MIN_SMALL_SIZE currently must be 32");
const_assert!(BLOCK_SIZE == 65536, "BLOCK_SIZE currently must be 65536");

fn alloc_small(size: usize) -> Option<AllocatedRwToken> {
    match size {
        32 => alloc_small_const::<32>(),
        64 => alloc_small_const::<64>(),
        128 => alloc_small_const::<128>(),
        256 => alloc_small_const::<256>(),
        512 => alloc_small_const::<512>(),
        1024 => alloc_small_const::<1024>(),
        2048 => alloc_small_const::<2048>(),
        4096 => alloc_small_const::<4096>(),
        8192 => alloc_small_const::<8192>(),
        16384 => alloc_small_const::<16384>(),
        32768 => alloc_small_const::<32768>(),
        _ => panic!(
            "alloc_small called with size {} which is not a power of 2 between {} and {}",
            size,
            MIN_SMALL_SIZE,
            BLOCK_SIZE / 2
        ),
    }
}

fn alloc_small_const<const SIZE: usize>() -> Option<AllocatedRwToken> {
    let mut allocator = SMALL_ALLOCATOR.lock();
    let index = SIZE.trailing_zeros() as usize;
    if let Some(slab) = allocator.partial_lists[index].as_mut() {
        let mut slab = slab
            .get_mut::<SIZE>()
            .expect("Slab index/size calculation failed");
        let token = slab.allocate().expect("Partial list full");
        // If it became full, we remove from the partial list.
        if slab.is_full() {
            allocator.partial_lists[index] = slab.take_next().map(DynamicSizedSlab::new);
        }
        return Some(token);
    }
    // Allocate a new slab and add it to the partial list.
    let slab_allocation = alloc_large(BLOCK_SIZE)?;
    assert!(
        slab_allocation.address().is_multiple_of(BLOCK_SIZE),
        "Slab allocation must be block-size aligned"
    );
    let mut new_slab = Slab::<SIZE>::new(slab_allocation);
    let token = new_slab.allocate().expect("New slab should not be full");
    if !new_slab.is_full() {
        allocator.partial_lists[index] = Some(DynamicSizedSlab::new(new_slab));
    }
    Some(token)
}

/// # Safety
/// The token must have been created by `alloc_small`.
unsafe fn free_small(token: AllocatedRwToken) {
    let size = token.size();
    unsafe {
        match size {
            32 => free_small_const::<32>(token),
            64 => free_small_const::<64>(token),
            128 => free_small_const::<128>(token),
            256 => free_small_const::<256>(token),
            512 => free_small_const::<512>(token),
            1024 => free_small_const::<1024>(token),
            2048 => free_small_const::<2048>(token),
            4096 => free_small_const::<4096>(token),
            8192 => free_small_const::<8192>(token),
            16384 => free_small_const::<16384>(token),
            32768 => free_small_const::<32768>(token),
            _ => panic!(
                "free_small called with size {} which is not a power of 2 between {} and {}",
                size,
                MIN_SMALL_SIZE,
                BLOCK_SIZE / 2
            ),
        }
    }
}

unsafe fn free_small_const<const SIZE: usize>(token: AllocatedRwToken) {
    // We must hold the lock to prevent races, although we do not immediately use it.
    let mut allocator = SMALL_ALLOCATOR.lock();
    let index = SIZE.trailing_zeros() as usize;
    // A little dirty, but we grab a reference to the slab by aligning the token address down to a block boundary.
    let slab_address = align_address_down(token.address(), BLOCK_SIZE) as *mut u8;
    let mut slab = unsafe { Slab::<SIZE>::from_raw(slab_address) };
    let was_full = slab.is_full();
    slab.free(token);
    let is_empty = slab.is_empty();
    if is_empty {
        // Remove from the partial list chain.
        match (slab.take_previous(), slab.take_next()) {
            (Some(mut previous), Some(next)) => {
                previous.set_next(next);
            }
            (Some(_previous), None) => {} // The `take_previous` already removed us.
            (None, Some(next)) => {
                allocator.partial_lists[index] = Some(DynamicSizedSlab::new(next));
            }
            (None, None) => {
                allocator.partial_lists[index] = None;
            }
        }
        let Ok(allocation) = slab.try_into_allocation() else {
            panic!("Slab is empty but could not be converted into an allocation");
        };
        free_large(allocation);
    } else if was_full {
        // Add to the partial list
        if let Some(previous_partial_list) = allocator.partial_lists[index].take() {
            slab.set_next(
                previous_partial_list
                    .try_into()
                    .expect("Partial list type mismatch"),
            );
        }
        allocator.partial_lists[index] = Some(DynamicSizedSlab::new(slab));
    }
}
