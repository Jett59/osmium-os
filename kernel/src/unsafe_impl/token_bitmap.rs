use core::{
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::unsafe_impl::memory::MemoryToken;

/// A token that represents a range of memory in terms of blocks, where each block is of size `BLOCK_SIZE`.
/// Slightly confusingly, the `address` and `size` methods return the address and size in blocks, not bytes.
pub struct BitmapToken<Inner: MemoryToken, const BLOCK_SIZE: usize> {
    block_index: usize,
    block_count: usize,
    _phantom: PhantomData<Inner>,
}

impl<Inner: MemoryToken, const BLOCK_SIZE: usize> MemoryToken for BitmapToken<Inner, BLOCK_SIZE> {
    type ViewToken = Self;

    unsafe fn new(start: usize, size: usize) -> Self {
        Self {
            block_index: start,
            block_count: size,
            _phantom: PhantomData,
        }
    }

    fn address(&self) -> usize {
        self.block_index
    }

    fn size(&self) -> usize {
        self.block_count
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BitmapTokenError {
    UnalignedAddress,
    UnalignedSize,
}

impl<Inner: MemoryToken, const BLOCK_SIZE: usize> BitmapToken<Inner, BLOCK_SIZE> {
    pub fn byte_address(&self) -> usize {
        self.block_index * BLOCK_SIZE
    }

    pub fn byte_size(&self) -> usize {
        self.block_count * BLOCK_SIZE
    }

    pub fn try_from_inner(inner: Inner) -> Result<Self, BitmapTokenError> {
        let start = inner.address();
        let size = inner.size();
        if !start.is_multiple_of(BLOCK_SIZE) {
            return Err(BitmapTokenError::UnalignedAddress);
        }
        if !size.is_multiple_of(BLOCK_SIZE) {
            return Err(BitmapTokenError::UnalignedSize);
        }
        Ok(Self {
            block_index: start / BLOCK_SIZE,
            block_count: size / BLOCK_SIZE,
            _phantom: PhantomData,
        })
    }

    pub fn into_inner(self) -> Inner {
        let start = self.byte_address();
        let size = self.byte_size();
        unsafe { Inner::new(start, size) }
    }
}

pub const fn get_bitmap_size(bits: usize) -> usize {
    bits.div_ceil(usize::BITS as usize)
}

/// A token store backed by a bitmap, where each bit represents a block of some particular size.
/// This type provides simple API to store and retrieve tokens by address, but does not provide an allocation algorithm.
pub struct TokenBitmap<T: MemoryToken, const BLOCK_SIZE: usize, const BITS: usize>
where
    [(); get_bitmap_size(BITS)]:,
{
    // Each bit represents a token, with addresses starting at 0.
    bits: [AtomicUsize; get_bitmap_size(BITS)],
    _phantom: PhantomData<T>,
}

impl<T: MemoryToken, const BLOCK_SIZE: usize, const BITS: usize> TokenBitmap<T, BLOCK_SIZE, BITS>
where
    [(); get_bitmap_size(BITS)]:,
{
    pub const fn new() -> Self {
        Self {
            bits: [const { AtomicUsize::new(0) }; get_bitmap_size(BITS)],
            _phantom: PhantomData,
        }
    }

    fn get_index_and_bit_offset(block_index: usize) -> (usize, usize) {
        let index = block_index / usize::BITS as usize;
        let bit_offset = block_index % usize::BITS as usize;
        (index, bit_offset)
    }

    fn create_bit_mask(start_bit_offset: usize, range_length: usize) -> usize {
        if start_bit_offset == 0 && range_length == usize::BITS as usize {
            !0
        } else {
            ((1 << range_length) - 1) << start_bit_offset
        }
    }

    pub fn store_range(&self, token: BitmapToken<T, BLOCK_SIZE>) {
        let (first_usize_index, first_bit_offset) =
            Self::get_index_and_bit_offset(token.block_index);
        let (last_usize_index, last_bit_offset) =
            Self::get_index_and_bit_offset(token.block_index + token.block_count - 1);

        if first_usize_index == last_usize_index {
            let mask = Self::create_bit_mask(first_bit_offset, token.block_count);
            self.bits[first_usize_index].fetch_or(mask, Ordering::SeqCst);
        } else {
            // Set the bits in the first usize
            let first_mask =
                Self::create_bit_mask(first_bit_offset, usize::BITS as usize - first_bit_offset);
            self.bits[first_usize_index].fetch_or(first_mask, Ordering::SeqCst);

            // Set the bits in the middle usizes
            for i in (first_usize_index + 1)..last_usize_index {
                self.bits[i].store(!0, Ordering::SeqCst);
            }

            // Set the bits in the last usize
            let last_mask = Self::create_bit_mask(0, last_bit_offset + 1);
            self.bits[last_usize_index].fetch_or(last_mask, Ordering::SeqCst);
        }
    }

    pub fn read_range(
        &self,
        start_block: usize,
        block_count: usize,
    ) -> BitmapRangeIterator<'_, T, BLOCK_SIZE, BITS> {
        BitmapRangeIterator {
            bitmap: self,
            current_block_index: start_block,
            block_count,
        }
    }
}

pub struct BitmapRangeIterator<'a, T: MemoryToken, const BLOCK_SIZE: usize, const BITS: usize>
where
    [(); get_bitmap_size(BITS)]:,
{
    bitmap: &'a TokenBitmap<T, BLOCK_SIZE, BITS>,
    current_block_index: usize,
    block_count: usize,
}

impl<'a, T: MemoryToken, const BLOCK_SIZE: usize, const BITS: usize> Iterator
    for BitmapRangeIterator<'a, T, BLOCK_SIZE, BITS>
where
    [(); get_bitmap_size(BITS)]:,
{
    type Item = BitmapToken<T, BLOCK_SIZE>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.block_count == 0 {
            return None;
        }

        let (mut current_usize_index, current_bit_offset) =
            TokenBitmap::<T, BLOCK_SIZE, BITS>::get_index_and_bit_offset(self.current_block_index);
        let (last_usize_index, last_bit_offset) =
            TokenBitmap::<T, BLOCK_SIZE, BITS>::get_index_and_bit_offset(
                self.current_block_index + self.block_count - 1,
            );

        // The algorithm essentially reads several bits and sets them to zero at the same time.
        // While this becomes more complex, it neatly avoids the need for retries or locks.
        // Effectively it allocates many more tokens than requested, then checks how many it got and puts the rest back.

        // Unfortunately, we need another special case if the start and end are in the same usize.
        if current_usize_index == last_usize_index {
            let mask = TokenBitmap::<T, BLOCK_SIZE, BITS>::create_bit_mask(
                current_bit_offset,
                self.block_count,
            );
            let bits = self.bitmap.bits[current_usize_index].fetch_and(!mask, Ordering::SeqCst);
            let found_bits = bits & mask;
            if found_bits == 0 {
                self.block_count = 0;
                return None;
            }
            let first_set_bit = found_bits.trailing_zeros() as usize;
            // This trick clears the lowest set bit
            let other_bits = found_bits & (found_bits - 1);
            self.bitmap.bits[current_usize_index].fetch_or(other_bits, Ordering::SeqCst);
            self.current_block_index += first_set_bit - current_bit_offset;
            self.block_count -= first_set_bit - current_bit_offset;
            return Some(BitmapToken::<T, BLOCK_SIZE> {
                block_index: self.current_block_index,
                block_count: 1,
                _phantom: PhantomData,
            });
        }

        let mut search_mask = TokenBitmap::<T, BLOCK_SIZE, BITS>::create_bit_mask(
            current_bit_offset,
            usize::BITS as usize - current_bit_offset,
        );
        while search_mask != 0 {
            let bits =
                self.bitmap.bits[current_usize_index].fetch_and(!search_mask, Ordering::SeqCst);
            let found_bits = bits & search_mask;
            if found_bits != 0 {
                let first_set_bit = found_bits.trailing_zeros() as usize;
                // Clear lowest set bit
                let other_bits = found_bits & (found_bits - 1);
                self.bitmap.bits[current_usize_index].fetch_or(other_bits, Ordering::SeqCst);
                let new_block_index = current_usize_index * usize::BITS as usize + first_set_bit;
                self.block_count -= new_block_index - self.current_block_index;
                self.current_block_index = new_block_index;
                return Some(BitmapToken::<T, BLOCK_SIZE> {
                    block_index: self.current_block_index,
                    block_count: 1,
                    _phantom: PhantomData,
                });
            }
            current_usize_index += 1;
            if current_usize_index < last_usize_index {
                search_mask = !0;
            } else if current_usize_index == last_usize_index {
                search_mask =
                    TokenBitmap::<T, BLOCK_SIZE, BITS>::create_bit_mask(0, last_bit_offset + 1);
            } else {
                break;
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use crate::unsafe_impl::memory::test::TestMemoryToken;

    use super::*;

    #[test]
    fn simple_bitmap_test() {
        let bitmap: TokenBitmap<TestMemoryToken, 1, 128> = TokenBitmap::new();
        // Store a single at index 0 and read it back
        bitmap.store_range(BitmapToken::try_from_inner(TestMemoryToken::safe_new(0, 1)).unwrap());
        let mut iter = bitmap.read_range(0, 128);
        assert_eq!(
            iter.next().map(BitmapToken::into_inner),
            Some(TestMemoryToken::safe_new(0, 1)),
        );
        assert!(iter.next().is_none());
        // Check that it actually cleared the bit
        let mut iter = bitmap.read_range(0, 128);
        assert!(iter.next().is_none());

        // Add a token in the second usize
        bitmap.store_range(BitmapToken::try_from_inner(TestMemoryToken::safe_new(64, 1)).unwrap());
        let mut iter = bitmap.read_range(0, 128);
        assert_eq!(
            iter.next().map(BitmapToken::into_inner),
            Some(TestMemoryToken::safe_new(64, 1)),
        );
        assert!(iter.next().is_none());

        // Add a token that spans two usizes
        bitmap.store_range(BitmapToken::try_from_inner(TestMemoryToken::safe_new(63, 2)).unwrap());
        let mut iter = bitmap.read_range(0, 128);
        assert_eq!(
            iter.next().map(BitmapToken::into_inner),
            Some(TestMemoryToken::safe_new(63, 1)),
        );
        assert_eq!(
            iter.next().map(BitmapToken::into_inner),
            Some(TestMemoryToken::safe_new(64, 1)),
        );
        assert!(iter.next().is_none());
    }
}
