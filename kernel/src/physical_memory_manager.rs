use crate::{assert::const_assert, paging::PAGE_SIZE};
use core::{
    intrinsics::size_of,
    mem::MaybeUninit,
    sync::atomic::{AtomicUsize, Ordering},
};

pub const fn get_bitmap_size(bits: usize) -> usize {
    (bits + size_of::<usize>() * 8 - 1) / (size_of::<usize>() * 8)
}

pub struct MemoryBitmapAllocator<const BITS: usize>
where
    [(); get_bitmap_size(BITS)]:,
{
    // If a bit is one, that means that it is available (free). Otherwise it is marked as unavailable (used)
    bits: [AtomicUsize; get_bitmap_size(BITS)],
}

impl<const BITS: usize> MemoryBitmapAllocator<BITS>
where
    [(); get_bitmap_size(BITS)]:,
{
    pub const fn new() -> Self {
        // A bit of a dirty hack, but there is no easy way to create a zero-initialised array of AtomicUsizes in a const context.
        let mut result = MaybeUninit::uninit_array();
        // And there is also no for loop, so we do a range manually.
        let mut i = 0;
        while i < get_bitmap_size(BITS) {
            result[i] = MaybeUninit::new(AtomicUsize::new(0));
            i += 1;
        }
        Self {
            bits: unsafe { MaybeUninit::array_assume_init(result) },
        }
    }

    fn get_index_and_bit_offset(bit: usize) -> (usize, usize) {
        let index = bit / (size_of::<usize>() * 8);
        let bit_offset = bit % (size_of::<usize>() * 8);
        (index, bit_offset)
    }

    pub fn mark_as_free(&mut self, bit: usize) {
        let (index, bit_offset) = Self::get_index_and_bit_offset(bit);
        self.bits[index].fetch_or(1 << bit_offset, Ordering::SeqCst);
    }

    pub fn mark_as_used(&mut self, bit: usize) {
        let (index, bit_offset) = Self::get_index_and_bit_offset(bit);
        self.bits[index].fetch_and(!(1 << bit_offset), Ordering::SeqCst);
    }

    pub fn get_bit_range_mask(start: usize, end: usize) -> usize {
        let mut mask = 0;
        for i in start..end {
            mask |= 1 << i;
        }
        mask
    }

    pub fn mark_range_as_free(&mut self, start: usize, end: usize) {
        // Split into a series of masks so that we don't have to do so many operations.
        // This means that we take the leading bits before a multiple of the size of usize, we create a mask for that, then we go over all of the complete usizes and set them straight up, then we do the same with the trailing bits.
        let (start_index, start_bit_offset) = Self::get_index_and_bit_offset(start);
        let (end_index, end_bit_offset) = Self::get_index_and_bit_offset(end);
        if start_index == end_index {
            // If the start and end are in the same usize, we can just create a mask for the range between them.
            self.bits[start_index].fetch_or(
                Self::get_bit_range_mask(start_bit_offset, end_bit_offset),
                Ordering::SeqCst,
            );
        } else {
            // Otherwise the lengthier algorithm.
            self.bits[start_index].fetch_or(
                Self::get_bit_range_mask(start_bit_offset, size_of::<usize>() * 8),
                Ordering::SeqCst,
            );
            for i in start_index + 1..end_index {
                self.bits[i].store(!0, Ordering::SeqCst);
            }
            self.bits[end_index].fetch_or(
                Self::get_bit_range_mask(0, end_bit_offset),
                Ordering::SeqCst,
            );
        }
    }

    pub fn mark_range_as_used(&mut self, start: usize, end: usize) {
        let (start_index, start_bit_offset) = Self::get_index_and_bit_offset(start);
        let (end_index, end_bit_offset) = Self::get_index_and_bit_offset(end);
        if start_index == end_index {
            self.bits[start_index].fetch_and(
                !Self::get_bit_range_mask(start_bit_offset, end_bit_offset),
                Ordering::SeqCst,
            );
        } else {
            self.bits[start_index].fetch_and(
                !Self::get_bit_range_mask(start_bit_offset, size_of::<usize>() * 8),
                Ordering::SeqCst,
            );
            for i in start_index + 1..end_index {
                self.bits[i].store(0, Ordering::SeqCst);
            }
            self.bits[end_index].fetch_and(
                !Self::get_bit_range_mask(0, end_bit_offset),
                Ordering::SeqCst,
            );
        }
    }

    pub fn allocate_block(&mut self) -> Option<usize> {
        // Simply traverse the list of usizes and find the first non-zero one. If there are none, we will return None.
        // This does have a race condition if someone goes and frees some memory while we are allocating, however this is an edge-case and can be safely ignored to make it simpler and faster.
        for i in 0..get_bitmap_size(BITS) {
            if self.bits[i].load(Ordering::SeqCst) != 0 {
                // Take the entire entry, set it to all ones (all used), then put it back with the relevant bits set to zero.
                let entry = self.bits[i].swap(0, Ordering::SeqCst);
                if entry == 0 {
                    continue;
                }
                let bit = entry.trailing_zeros() as usize;
                // Don't forget to write it back, otherwise no one will be able to allocate from this chunk of blocks anymore.
                // Use or to include any frees which happened while we weren't looking.
                self.bits[i].fetch_or(entry & !(1 << bit), Ordering::SeqCst);
                return Some(i * size_of::<usize>() * 8 + bit);
            }
        }
        None
    }
}

// The size of a block (bit) in the bitmap allocator.
pub const BLOCK_SIZE: usize = 65536;
pub const LOG2_BLOCK_SIZE: u8 = BLOCK_SIZE.trailing_zeros() as u8;

const_assert!(
    1 << LOG2_BLOCK_SIZE == BLOCK_SIZE,
    "BLOCK_SIZE must be a power of two"
);

const_assert!(
    PAGE_SIZE < BLOCK_SIZE && BLOCK_SIZE % PAGE_SIZE == 0,
    "Block size must be larger and divisible by (platform-specific) page size."
);

pub const PAGES_PER_BLOCK: usize = BLOCK_SIZE / PAGE_SIZE;

pub const MAX_PHYSICAL_MEMORY: usize = 0x1000000000;

pub const BLOCK_COUNT: usize = MAX_PHYSICAL_MEMORY / BLOCK_SIZE;

pub fn get_block_index(address: usize) -> usize {
    address / BLOCK_SIZE
}

pub fn get_address(block_index: usize) -> usize {
    block_index * BLOCK_SIZE
}

pub static mut GLOBAL_PMM: MemoryBitmapAllocator<BLOCK_COUNT> = MemoryBitmapAllocator::new();

pub fn mark_as_free(address: usize) {
    unsafe {
        GLOBAL_PMM.mark_as_free(get_block_index(address));
    }
}

pub fn mark_as_used(address: usize) {
    unsafe {
        GLOBAL_PMM.mark_as_used(get_block_index(address));
    }
}

pub fn mark_range_as_free(start_address: usize, end_address: usize) {
    unsafe {
        GLOBAL_PMM.mark_range_as_free(get_block_index(start_address), get_block_index(end_address));
    }
}

pub fn mark_range_as_used(start_address: usize, end_address: usize) {
    unsafe {
        GLOBAL_PMM.mark_range_as_used(get_block_index(start_address), get_block_index(end_address));
    }
}

pub fn allocate_block_address() -> Option<usize> {
    unsafe { GLOBAL_PMM.allocate_block().map(get_address) }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pmm_bitmap_test() {
        let mut allocator: MemoryBitmapAllocator<1024> = MemoryBitmapAllocator::new();
        assert_eq!(allocator.allocate_block(), None);
        allocator.mark_range_as_free(52, 60);
        for i in 52..60 {
            assert_eq!(allocator.allocate_block(), Some(i));
        }
        assert_eq!(allocator.allocate_block(), None);
        allocator.mark_as_free(17);
        assert_eq!(allocator.allocate_block(), Some(17));
    }
}

// This is outside the test module because it is for testing in the real kernel environment and not part of the unit testing suite.
pub fn sanity_check() {
    // Make sure there is some memory to work with. I'll probably add more stuff later.
    mark_as_free(
        allocate_block_address().expect("There should be at least some memory by this point"),
    );
}
