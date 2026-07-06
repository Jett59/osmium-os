use core::mem::replace;

use crate::unsafe_impl::memory::MemoryToken;

struct LeafBuddyEntry<T: MemoryToken> {
    order: u8,
    token: T, // `size` is 0 for non-free memory
    sibling: u32,
    parent: u32,
    next_of_this_size: u32,
    previous_of_this_size: u32,
}

impl<T: MemoryToken> LeafBuddyEntry<T> {
    fn is_free(&self) -> bool {
        self.token.size() != 0
    }

    fn take_token(&mut self) -> Option<T> {
        if self.is_free() {
            let token_address = self.token.address();
            let token = replace(&mut self.token, T::empty(token_address));
            Some(token)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
struct ParentBuddyEntry {
    order: u8,
    sibling: u32,
    left_child: u32,
    right_child: u32,
    parent: u32,
}
#[derive(Clone, Copy)]
struct UnusedBuddyEntry {
    next: u32,
    previous: u32,
}

#[repr(u32)]
enum BuddyEntry<T: MemoryToken> {
    Uninitialized = 0,
    Unused(UnusedBuddyEntry),
    Parent(ParentBuddyEntry),
    Leaf(LeafBuddyEntry<T>),
}

impl<T: MemoryToken> BuddyEntry<T> {
    fn as_leaf(&self) -> &LeafBuddyEntry<T> {
        match self {
            BuddyEntry::Leaf(leaf) => leaf,
            _ => panic!("Not a leaf but expected to be"),
        }
    }
    fn as_leaf_mut(&mut self) -> &mut LeafBuddyEntry<T> {
        match self {
            BuddyEntry::Leaf(leaf) => leaf,
            _ => panic!("Not a leaf but expected to be"),
        }
    }

    fn as_parent(&self) -> &ParentBuddyEntry {
        match self {
            BuddyEntry::Parent(parent) => parent,
            _ => panic!("Not a parent but expected to be"),
        }
    }
    fn as_parent_mut(&mut self) -> &mut ParentBuddyEntry {
        match self {
            BuddyEntry::Parent(parent) => parent,
            _ => panic!("Not a parent but expected to be"),
        }
    }

    fn as_unused(&self) -> &UnusedBuddyEntry {
        match self {
            BuddyEntry::Unused(unused) => unused,
            _ => panic!("Not an unused entry but expected to be"),
        }
    }
    fn as_unused_mut(&mut self) -> &mut UnusedBuddyEntry {
        match self {
            BuddyEntry::Unused(unused) => unused,
            _ => panic!("Not an unused entry but expected to be"),
        }
    }
}

pub struct BuddyAllocator<
    T: MemoryToken,
    const CAPACITY: usize,
    const HIGHEST_ORDER: u8,
    const LOWEST_ORDER: u8,
> where
    [(); (HIGHEST_ORDER - LOWEST_ORDER + 1) as usize]:,
{
    entries: [BuddyEntry<T>; CAPACITY],
    free_indices_for_orders: [Option<u32>; (HIGHEST_ORDER - LOWEST_ORDER + 1) as usize],
    allocated_indices_for_orders: [Option<u32>; (HIGHEST_ORDER - LOWEST_ORDER + 1) as usize],
    unused_entries: Option<u32>,
}

impl<T: MemoryToken, const CAPACITY: usize, const HIGHEST_ORDER: u8, const LOWEST_ORDER: u8>
    BuddyAllocator<T, CAPACITY, HIGHEST_ORDER, LOWEST_ORDER>
where
    [(); (HIGHEST_ORDER - LOWEST_ORDER + 1) as usize]:,
{
    const NON_EXISTANT_INDEX: u32 = u32::MAX;

    /// Creates a buddy allocator which has all elements initialized to zero and all indices None.
    /// This is a good state in which to call .all_unused(), which initializes the unused indices properly.
    /// The reason why this not the default is that initializing with zeros allows for storage in the BSS section (good for large buddy allocators).
    /// Additionally, using assignment would mean (potentially) having to store one of these things on the stack, which may be impossible for large ones. Using a builder-style interface is the best I can think of.
    pub const fn unusable() -> Self {
        // Stupid const rules force us to do this as a separate function
        const fn construct_unusable<T: MemoryToken>() -> BuddyEntry<T> {
            BuddyEntry::<T>::Uninitialized
        }
        Self {
            entries: [const { construct_unusable() }; CAPACITY],
            free_indices_for_orders: [None; (HIGHEST_ORDER - LOWEST_ORDER + 1) as usize],
            allocated_indices_for_orders: [None; (HIGHEST_ORDER - LOWEST_ORDER + 1) as usize],
            unused_entries: None,
        }
    }

    pub fn all_unused(&mut self) -> &mut Self {
        self.unused_entries = Some(0);
        // Initialize the middle entries separately for simplicity.
        self.entries[0] = BuddyEntry::Unused(UnusedBuddyEntry {
            next: 1,
            previous: Self::NON_EXISTANT_INDEX,
        });
        for i in 1..CAPACITY - 1 {
            self.entries[i] = BuddyEntry::Unused(UnusedBuddyEntry {
                next: (i + 1) as u32,
                previous: (i - 1) as u32,
            });
        }
        self.entries[CAPACITY - 1] = BuddyEntry::Unused(UnusedBuddyEntry {
            next: Self::NON_EXISTANT_INDEX,
            previous: (CAPACITY - 2) as u32,
        });
        self
    }

    fn get_order(size: usize) -> u8 {
        size.trailing_zeros() as u8
    }
    fn get_size(order: u8) -> usize {
        1 << order
    }

    fn append_to_free_list(&mut self, order: u8, index: u32) {
        self.entries[index as usize]
            .as_leaf_mut()
            .previous_of_this_size = Self::NON_EXISTANT_INDEX;
        if let Some(previous_index) = self.free_indices_for_orders[(order - LOWEST_ORDER) as usize]
        {
            // We can safely assume that the previous index is a leaf, because we only ever add leaves to the free list.
            self.entries[index as usize].as_leaf_mut().next_of_this_size = previous_index;
            self.entries[previous_index as usize]
                .as_leaf_mut()
                .previous_of_this_size = index;
        } else {
            self.entries[index as usize].as_leaf_mut().next_of_this_size = Self::NON_EXISTANT_INDEX;
        }
        self.free_indices_for_orders[(order - LOWEST_ORDER) as usize] = Some(index);
    }

    fn remove_from_free_list(&mut self, index: u32) {
        let LeafBuddyEntry {
            order,
            token,
            next_of_this_size,
            previous_of_this_size,
            ..
        } = self.entries[index as usize].as_leaf();
        let order = *order;
        let previous_of_this_size = *previous_of_this_size;
        let next_of_this_size = *next_of_this_size;
        if previous_of_this_size == Self::NON_EXISTANT_INDEX {
            if next_of_this_size == Self::NON_EXISTANT_INDEX {
                self.free_indices_for_orders[(order - LOWEST_ORDER) as usize] = None;
            } else {
                self.free_indices_for_orders[(order - LOWEST_ORDER) as usize] =
                    Some(next_of_this_size);
            }
        } else {
            self.entries[previous_of_this_size as usize]
                .as_leaf_mut()
                .next_of_this_size = next_of_this_size;
        }
        if next_of_this_size != Self::NON_EXISTANT_INDEX {
            self.entries[next_of_this_size as usize]
                .as_leaf_mut()
                .previous_of_this_size = previous_of_this_size;
        }
    }

    fn find_unused_index(&mut self) -> u32 {
        let unused_indices = self.unused_entries.expect("Buddy allocator full!");
        let first_unused_entry = *self.entries[unused_indices as usize].as_unused();
        if first_unused_entry.next != Self::NON_EXISTANT_INDEX {
            let second_unused_entry =
                self.entries[first_unused_entry.next as usize].as_unused_mut();
            second_unused_entry.previous = Self::NON_EXISTANT_INDEX;
            self.unused_entries = Some(first_unused_entry.next);
        } else {
            self.unused_entries = None;
        }
        unused_indices
    }

    pub fn add_entry(&mut self, token: T) -> &mut Self {
        let size = token.size();
        assert!(size.is_power_of_two(), "Size not a power of two");
        assert!(
            token.address().is_multiple_of(size),
            "Address not naturally aligned"
        );
        let index = self.find_unused_index();
        let first_unused_entry = &mut self.entries[index as usize];
        let order = Self::get_order(size);
        *first_unused_entry = BuddyEntry::Leaf(LeafBuddyEntry {
            order,
            token,
            sibling: Self::NON_EXISTANT_INDEX,
            parent: Self::NON_EXISTANT_INDEX,
            next_of_this_size: Self::NON_EXISTANT_INDEX,
            previous_of_this_size: Self::NON_EXISTANT_INDEX,
        });
        self.append_to_free_list(order, index);
        self
    }

    fn split_entry(&mut self, index: u32) {
        self.remove_from_free_list(index);
        let left_child_index = self.find_unused_index();
        let right_child_index = self.find_unused_index();
        let old_entry = self.entries[index as usize].as_leaf_mut();
        let (left_token, right_token) = old_entry
            .take_token()
            .unwrap()
            .split_at(Self::get_size(old_entry.order - 1));
        let new_order = old_entry.order - 1;
        let left_child = LeafBuddyEntry {
            order: new_order,
            token: left_token,
            sibling: right_child_index,
            parent: index,
            next_of_this_size: Self::NON_EXISTANT_INDEX,
            previous_of_this_size: Self::NON_EXISTANT_INDEX,
        };
        let right_child = LeafBuddyEntry {
            order: new_order,
            token: right_token,
            sibling: left_child_index,
            parent: index,
            next_of_this_size: Self::NON_EXISTANT_INDEX,
            previous_of_this_size: Self::NON_EXISTANT_INDEX,
        };
        let parent_entry = ParentBuddyEntry {
            order: old_entry.order,
            sibling: old_entry.sibling,
            parent: old_entry.parent,
            left_child: left_child_index,
            right_child: right_child_index,
        };
        self.entries[left_child_index as usize] = BuddyEntry::Leaf(left_child);
        self.entries[right_child_index as usize] = BuddyEntry::Leaf(right_child);
        self.entries[index as usize] = BuddyEntry::Parent(parent_entry);
        // Append the right child first in order to have the left be the first one (so it is handed out first).
        // This is not strictly necessary, just nice.
        self.append_to_free_list(new_order, right_child_index);
        self.append_to_free_list(new_order, left_child_index);
    }

    fn append_to_allocated_list(&mut self, order: u8, index: u32) {
        self.entries[index as usize]
            .as_leaf_mut()
            .previous_of_this_size = Self::NON_EXISTANT_INDEX;
        if let Some(previous_index) =
            self.allocated_indices_for_orders[(order - LOWEST_ORDER) as usize]
        {
            // Again, assuming leaf here is pretty safe (it should only fail if there is a bug somewhere else, in which case this is a good way of detecting it).
            self.entries[index as usize].as_leaf_mut().next_of_this_size = previous_index;
            self.entries[previous_index as usize]
                .as_leaf_mut()
                .previous_of_this_size = index;
        } else {
            self.entries[index as usize].as_leaf_mut().next_of_this_size = Self::NON_EXISTANT_INDEX;
        }
        self.allocated_indices_for_orders[(order - LOWEST_ORDER) as usize] = Some(index);
    }

    fn remove_from_allocated_list(&mut self, index: u32) {
        let LeafBuddyEntry {
            order,
            next_of_this_size,
            previous_of_this_size,
            ..
        } = self.entries[index as usize].as_leaf();
        let order = *order;
        let previous_of_this_size = *previous_of_this_size;
        let next_of_this_size = *next_of_this_size;
        if previous_of_this_size == Self::NON_EXISTANT_INDEX {
            if next_of_this_size == Self::NON_EXISTANT_INDEX {
                self.allocated_indices_for_orders[(order - LOWEST_ORDER) as usize] = None;
            } else {
                self.allocated_indices_for_orders[(order - LOWEST_ORDER) as usize] =
                    Some(next_of_this_size);
            }
        } else {
            self.entries[previous_of_this_size as usize]
                .as_leaf_mut()
                .next_of_this_size = next_of_this_size;
        }
        if next_of_this_size != Self::NON_EXISTANT_INDEX {
            self.entries[next_of_this_size as usize]
                .as_leaf_mut()
                .previous_of_this_size = previous_of_this_size;
        }
    }

    pub fn allocate(&mut self, size: usize) -> Option<T> {
        let order = Self::get_order(size.next_power_of_two());
        let first_index_for_this_order =
            self.free_indices_for_orders[(order - LOWEST_ORDER) as usize];
        let allocated_entry = if let Some(index) = first_index_for_this_order {
            self.remove_from_free_list(index);
            let entry = &mut self.entries[index as usize];
            Some((entry, index))
        } else {
            // Otherwise we look up a larger one and break it.
            let target_order = order;
            'result: {
                for order in (target_order + 1)..=HIGHEST_ORDER {
                    if self.free_indices_for_orders[(order - LOWEST_ORDER) as usize].is_some() {
                        // I know we're redefining 'order' rather a lot, but it makes things easy.
                        let initial_order = order;
                        for order in ((target_order + 1)..=initial_order).rev() {
                            // There should be an index for this order (either already there or created on the previous step.)
                            let first_free_index = self.free_indices_for_orders
                                [(order - LOWEST_ORDER) as usize]
                                .unwrap();
                            self.split_entry(first_free_index);
                        }
                        // Now the logic is much the same as that for the fast branch above (the one which doesn't do any splitting).
                        let first_free_index = self.free_indices_for_orders
                            [(target_order - LOWEST_ORDER) as usize]
                            .unwrap();
                        self.remove_from_free_list(first_free_index);
                        let entry = &mut self.entries[first_free_index as usize];
                        break 'result Some((entry, first_free_index));
                    }
                }
                None
            }
        };
        if let Some((entry, index)) = allocated_entry {
            let order = entry.as_leaf().order;
            let token = entry.as_leaf_mut().take_token().unwrap();
            self.append_to_allocated_list(order, index);
            Some(token)
        } else {
            None
        }
    }

    fn append_to_unused_list(&mut self, index: u32) {
        self.entries[index as usize].as_unused_mut().previous = Self::NON_EXISTANT_INDEX;
        if let Some(previous_index) = self.unused_entries {
            self.entries[index as usize].as_unused_mut().next = previous_index;
            self.entries[previous_index as usize]
                .as_unused_mut()
                .previous = index;
        } else {
            self.entries[index as usize].as_unused_mut().next = Self::NON_EXISTANT_INDEX;
        }
        self.unused_entries = Some(index);
    }

    fn merge(&mut self, index: u32) {
        let mut optional_index = Some(index);
        while let Some(index) = optional_index {
            let LeafBuddyEntry {
                sibling: sibling_index,
                token,
                parent: parent_index,
                ..
            } = self.entries[index as usize].as_leaf();
            let free = self.entries[index as usize].as_leaf().is_free();
            let sibling_index = *sibling_index;
            let parent_index = *parent_index;
            if sibling_index != Self::NON_EXISTANT_INDEX {
                if let BuddyEntry::Leaf(ref sibling) = self.entries[sibling_index as usize]
                    && free
                    && sibling.is_free()
                {
                    self.remove_from_free_list(index);
                    self.remove_from_free_list(sibling_index);
                    let this_token = self.entries[index as usize]
                        .as_leaf_mut()
                        .take_token()
                        .unwrap();
                    let sibling_token = self.entries[sibling_index as usize]
                        .as_leaf_mut()
                        .take_token()
                        .unwrap();
                    let parent_token = if this_token.address() < sibling_token.address() {
                        this_token.merge(sibling_token)
                    } else {
                        sibling_token.merge(this_token)
                    };
                    let parent = *self.entries[parent_index as usize].as_parent();
                    let parent_order = parent.order;
                    let new_parent_entry = LeafBuddyEntry {
                        order: parent_order,
                        token: parent_token,
                        parent: parent.parent,
                        sibling: parent.sibling,
                        next_of_this_size: Self::NON_EXISTANT_INDEX,
                        previous_of_this_size: Self::NON_EXISTANT_INDEX,
                    };
                    self.entries[index as usize] = BuddyEntry::Unused(UnusedBuddyEntry {
                        next: Self::NON_EXISTANT_INDEX,
                        previous: Self::NON_EXISTANT_INDEX,
                    });
                    self.entries[sibling_index as usize] = BuddyEntry::Unused(UnusedBuddyEntry {
                        next: Self::NON_EXISTANT_INDEX,
                        previous: Self::NON_EXISTANT_INDEX,
                    });
                    self.entries[parent_index as usize] = BuddyEntry::Leaf(new_parent_entry);
                    self.append_to_unused_list(index);
                    self.append_to_unused_list(sibling_index);
                    self.append_to_free_list(parent_order, parent_index);
                    optional_index = Some(parent_index);
                } else {
                    optional_index = None;
                }
            } else {
                optional_index = None;
            }
        }
    }

    pub fn free(&mut self, token: T) {
        let size = token.size();
        assert!(
            size.is_power_of_two(),
            "Attempt to free non-power-of-two size"
        );
        assert!(
            token.address().is_multiple_of(size),
            "Attempt to free non-aligned address"
        );
        let order = Self::get_order(size);
        // We unfortunately have to traverse the list of allocated chunks for this order to find our allocation.
        // TODO: Maybe we could traverse as a binary tree from the root blocks (not exactly sure how to keep track of those properly though).
        let mut optional_index = self.allocated_indices_for_orders[(order - LOWEST_ORDER) as usize];
        while let Some(index) = optional_index {
            let entry = self.entries[index as usize].as_leaf_mut();
            let entry_address = entry.token.address();
            if entry_address == token.address() {
                self.remove_from_allocated_list(index);
                self.append_to_free_list(order, index);
                self.entries[index as usize].as_leaf_mut().token = token;
                self.merge(index);
                return;
            } else if entry.next_of_this_size != Self::NON_EXISTANT_INDEX {
                optional_index = Some(entry.next_of_this_size);
            } else {
                optional_index = None;
            }
        }
        panic!(
            "Attempt to free address which was either not allocated (as this size) or already freed: {}",
            token.address()
        );
    }
}

#[cfg(test)]
mod test {
    use crate::unsafe_impl::memory::test::TestMemoryToken;

    use super::*;

    #[test]
    fn test_buddy_allocator() {
        let mut allocator: BuddyAllocator<TestMemoryToken, 64, 20, 16> = BuddyAllocator::unusable();
        allocator.all_unused();
        // Just a simple one for now: Put in a big entry and pull out a small entry or two or more than two.
        // Then try some freeing and allocating stuff to make sure it coalesces properly.
        allocator.add_entry(TestMemoryToken::safe_new(0, 1048576));
        assert_eq!(
            allocator.allocate(65536),
            Some(TestMemoryToken::safe_new(0, 65536))
        );
        assert_eq!(
            allocator.allocate(65536),
            Some(TestMemoryToken::safe_new(65536, 65536))
        );
        assert_eq!(
            allocator.allocate(65536),
            Some(TestMemoryToken::safe_new(131072, 65536))
        );
        assert_eq!(
            allocator.allocate(131072),
            Some(TestMemoryToken::safe_new(262144, 131072))
        );
        assert_eq!(allocator.allocate(1048576), None);
        assert_eq!(
            allocator.allocate(524288),
            Some(TestMemoryToken::safe_new(524288, 524288))
        );
        assert_eq!(allocator.allocate(524288), None);
        allocator.free(TestMemoryToken::safe_new(0, 65536));
        allocator.free(TestMemoryToken::safe_new(65536, 65536));
        assert_eq!(
            allocator.allocate(131072),
            Some(TestMemoryToken::safe_new(0, 131072))
        );
        allocator.free(TestMemoryToken::safe_new(131072, 65536));
        allocator.free(TestMemoryToken::safe_new(0, 131072));
        allocator.free(TestMemoryToken::safe_new(262144, 131072));
        allocator.free(TestMemoryToken::safe_new(524288, 524288));
        assert_eq!(
            allocator.allocate(1048576),
            Some(TestMemoryToken::safe_new(0, 1048576))
        );
    }
}
