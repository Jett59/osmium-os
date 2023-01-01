use crate::arch_api::paging::PAGE_SIZE;
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
    bits: [AtomicUsize; get_bitmap_size(BITS)],
}

impl<const BITS: usize> MemoryBitmapAllocator<BITS>
where
    [(); get_bitmap_size(BITS)]:,
{
    pub const fn new() -> Self {
        // Since AtomicUsize isn't copyable, this is the best solution I could find (ref: https://stackoverflow.com/a/69756635/11553216)
        unsafe {
            let mut bits: [MaybeUninit<AtomicUsize>; get_bitmap_size(BITS)] =
                MaybeUninit::uninit_array();
            // For loops are disallowed in const functions.
            let mut i = 0;
            while i < get_bitmap_size(BITS) {
                bits[i].write(AtomicUsize::new(0));
                i += 1;
            }
            Self {
                bits: MaybeUninit::array_assume_init(bits),
            }
        }
    }

    fn get_index_and_bit_offset(bit: usize) -> (usize, usize) {
        let index = bit / size_of::<usize>() * 8;
        let bit_offset = bit % size_of::<usize>() * 8;
        (index, bit_offset)
    }

    pub fn mark_as_used(&mut self, bit: usize) {
        let (index, bit_offset) = Self::get_index_and_bit_offset(bit);
        self.bits[index].fetch_or(1 << bit_offset, Ordering::SeqCst);
    }

    pub fn mark_as_free(&mut self, bit: usize) {
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

    pub fn mark_range_as_used(&mut self, start: usize, end: usize) {
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

    pub fn mark_range_as_free(&mut self, start: usize, end: usize) {
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
}

// The size of a block (bit) in the bitmap allocator.
pub const BLOCK_SIZE: usize = 65536;

pub const PAGES_PER_BLOCK: usize = BLOCK_SIZE / PAGE_SIZE;

pub const MAX_PHYSICAL_MEMORY: usize = 0x1000000000;

pub const BLOCK_COUNT: usize = MAX_PHYSICAL_MEMORY / BLOCK_SIZE;

pub fn get_block_index(address: usize) -> usize {
    address / BLOCK_SIZE
}

pub static mut GLOBAL_PMM: MemoryBitmapAllocator<BLOCK_COUNT> = MemoryBitmapAllocator::new();
