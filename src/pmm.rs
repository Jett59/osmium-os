use crate::{arch_api::paging, lazy_init::lazy_static};
use core::{intrinsics::size_of, sync::atomic::AtomicUsize};

const fn get_bitmap_size(bits: usize) -> usize {
    (bits + size_of::<usize>()) / 64
}

struct MemoryBitmap<const BITS: usize>
where
    [(); get_bitmap_size(BITS)]:,
{
    bits: [AtomicUsize; get_bitmap_size(BITS)],
}

impl<const BITS: usize> MemoryBitmap<BITS>
where
    [(); get_bitmap_size(BITS)]:,
{
    pub fn new() -> Self {
        // Since AtomicUsize isn't copyable, this is the best solution I could find (ref: https://stackoverflow.com/a/69756635/11553216)
        let bits = [(); get_bitmap_size(BITS)].map(|_| AtomicUsize::new(0));
        Self { bits }
    }
}

lazy_static! {
    static ref GLOBAL_PMM: MemoryBitmap<1024> = MemoryBitmap::new();
}
