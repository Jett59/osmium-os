use core::{mem, ops::Deref};

use alloc::boxed::Box;

pub trait MemoryToken {
    /// The type of the token that is used to represent a view into this memory region.
    /// Ordinarily this should be `Self`, unless `new` or `Drop::drop` requires unique ownership.
    type ViewToken: MemoryToken;

    /// # Safety
    /// Callers must ensure that the given memory region is valid for the token type and that it is uniquely owned.
    /// For unsafe code, ownership of the token implies unique ownership of the memory region it represents.
    /// Tokens may refer to memory outside the standard kernel address space (e.g. physical memory addresses), in which case uniqueness is only required within the token's particular address space.
    /// Note that zero-sized regions are valid, and impose no requirements on the caller.
    unsafe fn new(start: usize, size: usize) -> Self;

    fn empty(address: usize) -> Self
    where
        Self: Sized,
    {
        // SAFETY: zero-sized regions are valid, and impose no requirements on the caller.
        unsafe { Self::new(address, 0) }
    }

    fn address(&self) -> usize;
    fn size(&self) -> usize;

    /// Returns `self`, replacing `self` with an empty token at the same address.
    fn take(&mut self) -> Self
    where
        Self: Sized,
    {
        mem::replace(self, Self::empty(self.address()))
    }

    fn split_at(self, split_at: usize) -> (Self, Self)
    where
        Self: Sized,
    {
        assert!(
            split_at <= self.size(),
            "split point must be within the memory region"
        );
        let first = unsafe { Self::new(self.address(), split_at) };
        let second = unsafe { Self::new(self.address() + split_at, self.size() - split_at) };
        (first, second)
    }

    fn chunks(self, chunk_size: usize) -> MemoryTokenChunks<Self>
    where
        Self: Sized,
    {
        MemoryTokenChunks {
            remaining_token: self,
            chunk_size,
        }
    }

    fn merge(self, other: Self) -> Self
    where
        Self: Sized,
    {
        assert!(
            self.address() + self.size() == other.address(),
            "memory regions must be contiguous to merge"
        );
        unsafe { Self::new(self.address(), self.size() + other.size()) }
    }

    fn view(&self, offset: usize, size: usize) -> MemoryTokenView<'_, Self, Self::ViewToken>
    where
        Self: Sized,
    {
        assert!(
            size <= self.size() && offset + size <= self.size(),
            "view must be within the memory region"
        );
        // SAFETY: `view` is passed directly into the `MemoryTokenView` struct, which ensures that it cannot outlive the original token.
        let view = unsafe { Self::ViewToken::new(self.address() + offset, size) };
        MemoryTokenView { token: self, view }
    }
}

/// An iterator over chunks of a `MemoryToken`.
pub struct MemoryTokenChunks<Token: MemoryToken> {
    remaining_token: Token,
    chunk_size: usize,
}

impl<Token: MemoryToken> Iterator for MemoryTokenChunks<Token> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_token.size() == 0 {
            return None;
        }
        let chunk_size = self.chunk_size.min(self.remaining_token.size());
        let remaining_token = self.remaining_token.take();
        let (chunk, remaining) = remaining_token.split_at(chunk_size);
        self.remaining_token = remaining;
        Some(chunk)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_size = self.remaining_token.size();
        let num_chunks = remaining_size.div_ceil(self.chunk_size);
        (num_chunks, Some(num_chunks))
    }
}

/// A view into a memory region represented by a `MemoryToken`.
///
/// This type is used to provide a safe way to access a subset of a memory region without requiring unique ownership of the original token.
/// The only way to recover the `view` token by value is in the `Drop::drop` implementation, making this approximately equivalent to `&'a View`.
/// The lifetime bound ensures that the view cannot outlive the original token.
pub struct MemoryTokenView<'a, Token: MemoryToken, View: MemoryToken> {
    token: &'a Token,
    // This is almost unsound, but the lifetime bound ensures that this duplicated instance will never out-live the original token.
    // So long as `view` is never moved out of this type, meaning it can only be used to obtain a reference, this is sound.
    // Since unsafe code (outside of this module) cannot have ownership of `view`, the requirement of uniqueness of ownership is still satisfied.
    view: View,
}

impl<'a, Token: MemoryToken, View: MemoryToken> Deref for MemoryTokenView<'a, Token, View> {
    type Target = View;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

/// Represents ownership of an unused block of physical memory.
/// Paging code consumes this type to prevent double-allocation.
#[must_use]
pub struct PhysicalMemoryToken {
    start: usize,
    size: usize,
}

impl MemoryToken for PhysicalMemoryToken {
    type ViewToken = Self;

    unsafe fn new(start: usize, size: usize) -> Self {
        PhysicalMemoryToken { start, size }
    }

    fn address(&self) -> usize {
        self.start
    }

    fn size(&self) -> usize {
        self.size
    }
}

/// Represents ownership of an unallocated block of virtual memory.
#[must_use]
pub struct VirtualMemoryToken {
    start: usize,
    size: usize,
}

impl MemoryToken for VirtualMemoryToken {
    type ViewToken = Self;

    unsafe fn new(start: usize, size: usize) -> Self {
        VirtualMemoryToken { start, size }
    }

    fn address(&self) -> usize {
        self.start
    }

    fn size(&self) -> usize {
        self.size
    }
}

/// Represents ownership of an allocated block of normal memory.
/// This is more-or-less equivalent to a `Box<[u8]`, except that it is not automatically de-allocated when dropped, and it may not correspond to a heap allocation.
/// However, the assumptions of `Box<[u8]>` are a strict superset of the requirements of `AllocatedMemoryToken`, so it is possible to convert a `Box<[u8]>` into an `AllocatedMemoryToken`, but not vice-versa.
/// It is not suitable for MMIO, as it dereferences into a byte slice.
#[must_use]
pub struct AllocatedMemoryToken {
    start: usize,
    size: usize,
}

impl AllocatedMemoryToken {
    pub fn into_ptr(self) -> *mut u8 {
        self.start as *mut u8
    }

    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: The caller has guaranteed that the memory region is valid and, and the self borrow prevents invalid aliasing.
        unsafe { core::slice::from_raw_parts(self.start as *const u8, self.size) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: The caller has guaranteed that the memory region is valid and uniquely owned, and the self borrow prevents invalid aliasing.
        unsafe { core::slice::from_raw_parts_mut(self.start as *mut u8, self.size) }
    }
}

impl MemoryToken for AllocatedMemoryToken {
    type ViewToken = Self;

    unsafe fn new(start: usize, size: usize) -> Self {
        AllocatedMemoryToken { start, size }
    }

    fn address(&self) -> usize {
        self.start
    }

    fn size(&self) -> usize {
        self.size
    }
}

impl From<Box<[u8]>> for AllocatedMemoryToken {
    fn from(boxed: Box<[u8]>) -> Self {
        let size = boxed.len();
        let start = boxed.as_ptr() as usize;
        mem::forget(boxed);
        // SAFETY: The box ensures unique ownership, and the code above effectively removes the box's own ownership of the region.
        unsafe { Self::new(start, size) }
    }
}

impl<const N: usize> From<Box<[u8; N]>> for AllocatedMemoryToken {
    fn from(boxed: Box<[u8; N]>) -> Self {
        (boxed as Box<[u8]>).into()
    }
}

#[cfg(test)]
pub mod test {
    use crate::unsafe_impl::memory_token::MemoryToken;

    /// A simple implementation of `MemoryToken` for testing purposes.
    /// Unlike all other memory tokens, these are Clone + Copy, and so do not guarantee uniqueness.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct TestMemoryToken {
        start: usize,
        size: usize,
    }

    impl TestMemoryToken {
        pub fn safe_new(start: usize, size: usize) -> Self {
            TestMemoryToken { start, size }
        }
    }

    impl super::MemoryToken for TestMemoryToken {
        type ViewToken = Self;

        unsafe fn new(start: usize, size: usize) -> Self {
            TestMemoryToken { start, size }
        }

        fn address(&self) -> usize {
            self.start
        }

        fn size(&self) -> usize {
            self.size
        }
    }

    #[test]
    fn test_split_and_merge() {
        // Simple: split in half
        let token = TestMemoryToken::safe_new(0x1000, 0x1000);
        let (first, second) = token.split_at(0x800);
        assert_eq!(first.address(), 0x1000);
        assert_eq!(first.size(), 0x800);
        assert_eq!(second.address(), 0x1800);
        assert_eq!(second.size(), 0x800);

        // More complex: split into non-equal parts
        let token = TestMemoryToken::safe_new(0x2000, 0x1000);
        let (first, second) = token.split_at(0x600);
        assert_eq!(first.address(), 0x2000);
        assert_eq!(first.size(), 0x600);
        assert_eq!(second.address(), 0x2600);
        assert_eq!(second.size(), 0xA00);

        // Split into a zero-sized left token
        let token = TestMemoryToken::safe_new(0x3000, 0x1000);
        let (first, second) = token.split_at(0);
        assert_eq!(first.address(), 0x3000);
        assert_eq!(first.size(), 0);
        assert_eq!(second.address(), 0x3000);
        assert_eq!(second.size(), 0x1000);
    }

    #[test]
    fn test_view() {
        let token = TestMemoryToken::safe_new(0x4000, 0x1000);
        let view = token.view(0x800, 0x400);
        assert_eq!(view.address(), 0x4800);
        assert_eq!(view.size(), 0x400);
    }

    #[test]
    fn test_chunks() {
        let token = TestMemoryToken::safe_new(0x5000, 0x1000);
        let mut chunks = token.chunks(0x400);
        let first_chunk = chunks.next().unwrap();
        assert_eq!(first_chunk.address(), 0x5000);
        assert_eq!(first_chunk.size(), 0x400);
        let second_chunk = chunks.next().unwrap();
        assert_eq!(second_chunk.address(), 0x5400);
        assert_eq!(second_chunk.size(), 0x400);
        let third_chunk = chunks.next().unwrap();
        assert_eq!(third_chunk.address(), 0x5800);
        assert_eq!(third_chunk.size(), 0x400);
        let fourth_chunk = chunks.next().unwrap();
        assert_eq!(fourth_chunk.address(), 0x5C00);
        assert_eq!(fourth_chunk.size(), 0x400);
        assert!(chunks.next().is_none());
    }

    #[test]
    fn test_chunks_not_exact() {
        let token = TestMemoryToken::safe_new(0x5000, 0x1000);
        let mut chunks = token.chunks(0x600);
        let first_chunk = chunks.next().unwrap();
        assert_eq!(first_chunk.address(), 0x5000);
        assert_eq!(first_chunk.size(), 0x600);
        let second_chunk = chunks.next().unwrap();
        assert_eq!(second_chunk.address(), 0x5600);
        assert_eq!(second_chunk.size(), 0x600);
        let third_chunk = chunks.next().unwrap();
        assert_eq!(third_chunk.address(), 0x5C00);
        assert_eq!(third_chunk.size(), 0x400);
        assert!(chunks.next().is_none());
    }
}
