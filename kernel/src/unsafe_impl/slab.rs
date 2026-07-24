use core::ptr::null_mut;

use crate::{
    physical_memory_manager::BLOCK_SIZE,
    unsafe_impl::memory::{AllocatedMemoryToken, MemoryToken},
};

#[derive(Clone, Copy)]
struct SlabUnusedEntry {
    next_index: u16, // 0 for non-existent
}

#[derive(Clone, Copy)]
struct SlabHeadEntry<const SIZE: usize> {
    next: *mut SlabEntry<SIZE>,
    previous_of_this_size: *mut SlabEntry<SIZE>,
    first_unused_index: u16,
    allocated_count: u16,
}

#[repr(C)] // Make sure the 'data' field is at offset 0
union SlabEntry<const SIZE: usize> {
    data: [u8; SIZE],
    unused: SlabUnusedEntry,
    head: SlabHeadEntry<SIZE>,
}

/// A block-sized chunk of memory, divisible into fixed-size entries.
#[must_use = "Forgetting a slab leaks its memory"]
pub struct Slab<const SIZE: usize> {
    pointer: *mut SlabEntry<SIZE>,
}

impl<const SIZE: usize> Slab<SIZE> {
    const ENTRY_COUNT: usize = BLOCK_SIZE / size_of::<SlabEntry<SIZE>>();

    pub fn new(allocation: AllocatedMemoryToken) -> Self {
        // Slightly awkward, but basically checks that `32 <= SIZE <= BLOCK_SIZE / 2` at compile time.
        const fn check<const SIZE: usize>() {
            assert!(SIZE >= 32, "Slab entries must be at least 32 bytes");
            assert!(
                SIZE <= BLOCK_SIZE / 2,
                "Slab entries must be at most half a block"
            );
        }
        const { check::<SIZE>() };

        assert_eq!(allocation.size(), BLOCK_SIZE, "Slabs must be block-sized");
        let pointer = allocation.into_ptr() as *mut SlabEntry<SIZE>;
        // SAFETY: the pointer points to a valid block of memory which is uniquely owned by us.
        let entries = unsafe { core::slice::from_raw_parts_mut(pointer, Self::ENTRY_COUNT) };
        entries[0].head = SlabHeadEntry {
            next: null_mut(),
            previous_of_this_size: null_mut(),
            first_unused_index: 1,
            allocated_count: 0,
        };
        for (i, entry) in entries.iter_mut().enumerate().skip(1) {
            entry.unused = SlabUnusedEntry {
                next_index: (i + 1) as u16,
            };
        }
        entries[Self::ENTRY_COUNT - 1].unused.next_index = 0; // Last entry points to nothing
        Self { pointer }
    }

    fn head(&self) -> &SlabHeadEntry<SIZE> {
        // SAFETY: the pointer is valid, and the first entries is always a head entry.
        unsafe { &(*self.pointer).head }
    }

    fn head_mut(&mut self) -> &mut SlabHeadEntry<SIZE> {
        // SAFETY: the pointer is valid, and the first entries is always a head entry.
        unsafe { &mut (*self.pointer).head }
    }

    fn entry(&self, index: usize) -> &SlabEntry<SIZE> {
        assert!(index < Self::ENTRY_COUNT);
        // SAFETY: The index is within bounds.
        unsafe { &*self.pointer.add(index) }
    }

    fn entry_mut(&mut self, index: usize) -> &mut SlabEntry<SIZE> {
        assert!(index < Self::ENTRY_COUNT);
        // SAFETY: The index is within bounds.
        unsafe { &mut *self.pointer.add(index) }
    }

    pub fn is_full(&self) -> bool {
        self.head().first_unused_index == 0
    }

    pub fn is_empty(&self) -> bool {
        self.head().allocated_count == 0
    }

    pub fn try_into_allocation(self) -> Result<AllocatedMemoryToken, Self> {
        if self.is_empty() {
            // SAFETY: we have recovered all entries, so we can reconstitute the original block-sized allocation.
            Ok(unsafe { AllocatedMemoryToken::new(self.pointer as usize, BLOCK_SIZE) })
        } else {
            Err(self)
        }
    }

    fn into_ptr(self) -> *mut SlabEntry<SIZE> {
        self.pointer
    }

    /// # Safety
    /// The pointer must have been obtained from a slab of the same size, and must not be held by another `Slab` instance.
    unsafe fn from_ptr(ptr: *mut SlabEntry<SIZE>) -> Self {
        Self { pointer: ptr }
    }

    pub fn free(&mut self, token: AllocatedMemoryToken) {
        assert_eq!(token.size(), SIZE, "Token is wrong size for this slab");
        let offset = token.address() - self.pointer as usize;
        assert!(
            offset.is_multiple_of(SIZE),
            "Token does not point to a valid entry"
        );
        let index = offset / SIZE;
        assert!(
            index < Self::ENTRY_COUNT,
            "Token does not belong to this slab"
        );
        debug_assert_ne!(index, 0, "Cannot free the head entry of a slab"); // Should be impossible

        let head = self.head_mut();
        debug_assert!(
            head.allocated_count > 0,
            "Cannot free an entry from an empty slab"
        );
        let old_unused_index = head.first_unused_index;
        head.first_unused_index = index as u16;
        head.allocated_count -= 1;
        let entry = self.entry_mut(index);
        entry.unused = SlabUnusedEntry {
            next_index: old_unused_index,
        };
    }

    pub fn allocate(&mut self) -> Option<AllocatedMemoryToken> {
        let head = self.head_mut();
        if head.first_unused_index == 0 {
            return None; // No free entries
        }
        let index = head.first_unused_index as usize;
        let entry = self.entry_mut(index);
        // SAFETY: Our methods ensure that the index indeed points to an unused entry.
        let new_unused_index = unsafe { entry.unused.next_index };
        let head = self.head_mut(); // Reborrow to make Rust happy
        head.first_unused_index = new_unused_index;
        head.allocated_count += 1;
        // SAFETY: now that the entry is out of the linked list, we have effectively removed the slab's ownership over it.
        Some(unsafe { AllocatedMemoryToken::new(self.pointer as usize + index * SIZE, SIZE) })
    }

    pub fn take_next(&mut self) -> Option<Self> {
        let head = self.head_mut();
        if head.next.is_null() {
            None
        } else {
            // SAFETY: `next` is guaranteed to have been generated from a slab of the same size, which was previously owned by this slab.
            let next_slab = unsafe { Slab::from_ptr(head.next) };
            head.next = null_mut();
            Some(next_slab)
        }
    }

    pub fn take_previous(&mut self) -> Option<Self> {
        let head = self.head_mut();
        if head.previous_of_this_size.is_null() {
            None
        } else {
            // SAFETY: `previous_of_this_size` is guaranteed to have been generated from a slab of the same size, which was previously owned by this slab.
            let previous_slab = unsafe { Slab::from_ptr(head.previous_of_this_size) };
            head.previous_of_this_size = null_mut();
            Some(previous_slab)
        }
    }

    /// Sets the next slab in the linked list of slabs.
    /// If this slab had a next slab, it is returned. If the provided slab had a previous slab, this is also returned.
    pub fn set_next(&mut self, mut next: Slab<SIZE>) -> (Option<Self>, Option<Self>) {
        let this_next = self.take_next();
        let provided_previous = next.take_previous();
        next.head_mut().previous_of_this_size = self.pointer;
        let next_ptr = next.into_ptr();
        let head = self.head_mut();
        head.next = next_ptr;
        (this_next, provided_previous)
    }

    /// Sets the previous slab in the linked list of slabs.
    /// If this slab had a previous slab, it is returned. If the provided slab had a next slab, this is also returned.
    pub fn set_previous(&mut self, mut previous: Slab<SIZE>) -> (Option<Self>, Option<Self>) {
        let this_previous = self.take_previous();
        let provided_next = previous.take_next();
        previous.head_mut().next = self.pointer;
        let previous_ptr = previous.into_ptr();
        let head = self.head_mut();
        head.previous_of_this_size = previous_ptr;
        (this_previous, provided_next)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::{boxed::Box, vec::Vec};

    #[test]
    fn test_slab_allocate_and_free() {
        let mut slab = Slab::<32>::new(Box::new([0u8; BLOCK_SIZE]).into());
        let token1 = slab.allocate().expect("Should allocate first entry");
        let token2 = slab.allocate().expect("Should allocate second entry");
        assert_ne!(
            token1.address(),
            token2.address(),
            "Allocated tokens should be different"
        );
        let token1_address = token1.address();
        slab.free(token1);
        let token3 = slab.allocate().expect("Should allocate after freeing");
        assert_eq!(
            token1_address,
            token3.address(),
            "Freed token should be reallocated"
        );
    }

    #[test]
    fn test_slab_full() {
        let mut slab = Slab::<32>::new(Box::new([0u8; BLOCK_SIZE]).into());
        for _ in 0..(BLOCK_SIZE / 32 - 1) {
            assert!(slab.allocate().is_some(), "Should allocate until full");
        }
        assert!(slab.allocate().is_none(), "Should not allocate when full");
    }

    #[test]
    fn test_slab_alloc_order() {
        let mut slab = Slab::<32>::new(Box::new([0u8; BLOCK_SIZE]).into());
        // Allocate several tokens, then free them in a different order and check that they are reallocated in (reverse-)freed order.
        let mut tokens = (0..5)
            .map(|_| slab.allocate().expect("Should allocate"))
            .collect::<Vec<_>>();
        let free_order = [3, 1, 4, 0, 2];
        for &i in &free_order {
            slab.free(tokens[i].take());
        }
        for &i in free_order.iter().rev() {
            let token = slab.allocate().expect("Should allocate");
            assert_eq!(
                token.address(),
                tokens[i].address(),
                "Reallocated token should match freed token"
            );
        }
    }

    #[test]
    fn test_slab_linked_list() {
        let mut slab1 = Slab::<32>::new(Box::new([0u8; BLOCK_SIZE]).into());
        let mut slab2 = Slab::<32>::new(Box::new([0u8; BLOCK_SIZE]).into());
        let slab3 = Slab::<32>::new(Box::new([0u8; BLOCK_SIZE]).into());

        let slab2_pointer = slab2.pointer;
        let slab3_pointer = slab3.pointer;

        // Link slab1 -> slab2 -> slab3
        let (old_next, old_prev) = slab2.set_next(slab3);
        assert!(old_next.is_none(), "slab2 should not have had a next slab");
        assert!(
            old_prev.is_none(),
            "slab3 should not have had a previous slab"
        );

        let (old_next, old_prev) = slab1.set_next(slab2);
        assert!(old_next.is_none(), "slab1 should not have had a next slab");
        assert!(
            old_prev.is_none(),
            "slab2 should not have had a previous slab"
        );

        let mut next_slab1 = slab1.take_next().expect("slab1 should have a next slab");
        assert_eq!(
            next_slab1.pointer, slab2_pointer,
            "Next slab of slab1 should be slab2"
        );
        assert!(
            slab1.take_next().is_none(),
            "slab1 should not have a next slab anymore"
        );
        let mut next_slab2 = next_slab1
            .take_next()
            .expect("slab2 should have a next slab");
        assert_eq!(
            next_slab2.pointer, slab3_pointer,
            "Next slab of slab2 should be slab3"
        );
        assert!(
            next_slab1.take_next().is_none(),
            "slab2 should not have a next slab anymore"
        );
        assert!(
            next_slab2.take_next().is_none(),
            "slab3 should not have a next slab"
        );
        assert!(
            next_slab1.take_next().is_none(),
            "slab2 should not have a next slab anymore"
        );
    }

    #[test]
    fn test_slab_try_into_allocation() {
        let boxed = Box::new([0u8; BLOCK_SIZE]);
        let original_address = boxed.as_ptr() as usize;
        let mut slab = Slab::<32>::new(boxed.into());
        let token1 = slab.allocate().expect("Should allocate first entry");
        let token2 = slab.allocate().expect("Should allocate second entry");
        let Err(mut slab) = slab.try_into_allocation() else {
            panic!("Slab should not be convertible to allocation when not empty");
        };
        slab.free(token1);
        slab.free(token2);
        let Ok(allocation) = slab.try_into_allocation() else {
            panic!("Slab should be convertible to allocation when empty");
        };
        assert_eq!(
            allocation.address(),
            original_address,
            "Allocation address should match original address"
        );
        assert_eq!(
            allocation.size(),
            BLOCK_SIZE,
            "Allocation size should match block size"
        );
    }
}
