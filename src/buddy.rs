#[derive(Clone, Copy)]
struct LeafBuddyEntry {
    order: u8,
    free: bool,
    address: usize,
    sibling: u32,
    parent: u32,
    next_of_this_size: u32,
    previous_of_this_size: u32,
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

#[derive(Clone, Copy)]
enum BuddyEntry {
    Unused(UnusedBuddyEntry),
    Parent(ParentBuddyEntry),
    Leaf(LeafBuddyEntry),
}

impl BuddyEntry {
    fn as_leaf(&self) -> &LeafBuddyEntry {
        match self {
            BuddyEntry::Leaf(leaf) => leaf,
            _ => panic!("Not a leaf but expected to be"),
        }
    }
    fn as_leaf_mut(&mut self) -> &mut LeafBuddyEntry {
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

struct BuddyAllocator<const CAPACITY: usize, const HIGHEST_ORDER: u8, const LOWEST_ORDER: u8>
where
    [(); (HIGHEST_ORDER - LOWEST_ORDER + 1) as usize]:,
{
    entries: [BuddyEntry; CAPACITY],
    free_indices_for_orders: [Option<u32>; (HIGHEST_ORDER - LOWEST_ORDER + 1) as usize],
    allocated_indices_for_orders: [Option<u32>; (HIGHEST_ORDER - LOWEST_ORDER + 1) as usize],
    unused_entries: Option<u32>,
}

impl<const CAPACITY: usize, const HIGHEST_ORDER: u8, const LOWEST_ORDER: u8>
    BuddyAllocator<CAPACITY, HIGHEST_ORDER, LOWEST_ORDER>
where
    [(); (HIGHEST_ORDER - LOWEST_ORDER + 1) as usize]:,
{
    const NON_EXISTANT_INDEX: u32 = u32::MAX;

    /// Creates a buddy allocator which has all elements initialized to zero (probably) and all indices None.
    /// This is a good state in which to call .all_unused(), which initializes the unused indices properly.
    /// The reason why this not the default is that initializing with zeros allows for storage in the BSS section (good for large buddy allocators).
    /// Additionally, using assignment would mean (potentially) having to store one of these things on the stack, which may be impossible for large ones. Using a builder-style interface is the best I can think of.
    pub const fn unusable() -> Self {
        Self {
            entries: [BuddyEntry::Unused(UnusedBuddyEntry {
                next: 0,
                previous: 0,
            }); CAPACITY],
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
            self.entries[i as usize] = BuddyEntry::Unused(UnusedBuddyEntry {
                next: (i + 1) as u32,
                previous: (i - 1) as u32,
            });
        }
        self.entries[(CAPACITY - 1) as usize] = BuddyEntry::Unused(UnusedBuddyEntry {
            next: Self::NON_EXISTANT_INDEX,
            previous: (CAPACITY - 2) as u32,
        });
        self
    }

    fn get_order(size: usize) -> u8 {
        let size = size.next_power_of_two();
        size.trailing_zeros() as u8
    }
    fn get_size(order: u8) -> usize {
        1 << order
    }

    fn append_to_free_list(&mut self, order: u8, index: u32) {
        if let Some(previous_index) = self.free_indices_for_orders[(order - LOWEST_ORDER) as usize]
        {
            // We can fairly safely assume that the previous index is a leaf, because we only ever add leaves to the free list.
            self.entries[index as usize].as_leaf_mut().next_of_this_size = previous_index;
            self.entries[previous_index as usize]
                .as_leaf_mut()
                .previous_of_this_size = index;
        }
        self.free_indices_for_orders[(order - LOWEST_ORDER) as usize] = Some(index);
    }

    fn remove_from_free_list(&mut self, index: u32) {
        let entry = *self.entries[index as usize].as_leaf();
        let order = entry.order;
        if entry.previous_of_this_size == Self::NON_EXISTANT_INDEX {
            if entry.next_of_this_size == Self::NON_EXISTANT_INDEX {
                self.free_indices_for_orders[(order - LOWEST_ORDER) as usize] = None;
            } else {
                self.free_indices_for_orders[(order - LOWEST_ORDER) as usize] =
                    Some(entry.next_of_this_size);
            }
        } else {
            self.entries[entry.previous_of_this_size as usize]
                .as_leaf_mut()
                .next_of_this_size = entry.next_of_this_size;
        }
        if entry.next_of_this_size != Self::NON_EXISTANT_INDEX {
            self.entries[entry.next_of_this_size as usize]
                .as_leaf_mut()
                .previous_of_this_size = entry.previous_of_this_size;
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

    pub fn add_entry(&mut self, size: usize, address: usize) -> &mut Self {
        let index = self.find_unused_index();
        let first_unused_entry = &mut self.entries[index as usize];
        let order = Self::get_order(size);
        *first_unused_entry = BuddyEntry::Leaf(LeafBuddyEntry {
            order,
            free: true,
            address,
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
        let left_child = LeafBuddyEntry {
            order: old_entry.order - 1,
            free: true,
            address: old_entry.address,
            sibling: right_child_index,
            parent: index,
            next_of_this_size: Self::NON_EXISTANT_INDEX,
            previous_of_this_size: Self::NON_EXISTANT_INDEX,
        };
        let right_child = LeafBuddyEntry {
            order: old_entry.order - 1,
            free: true,
            address: old_entry.address + Self::get_size(old_entry.order - 1),
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
        self.append_to_free_list(left_child.order, left_child_index);
        self.append_to_free_list(right_child.order, right_child_index);
    }

    fn append_to_allocated_list(&mut self, order: u8, index: u32) {
        if let Some(previous_index) =
            self.allocated_indices_for_orders[(order - LOWEST_ORDER) as usize]
        {
            // Again, assuming leaf here is pretty safe (it should only fail if there is a bug somewhere else, in which case this is a good way of detecting it).
            self.entries[previous_index as usize]
                .as_leaf_mut()
                .next_of_this_size = index;
            self.entries[index as usize]
                .as_leaf_mut()
                .previous_of_this_size = previous_index;
        }
        self.allocated_indices_for_orders[(order - LOWEST_ORDER) as usize] = Some(index);
    }

    pub fn allocate(&mut self, size: usize) -> Option<usize> {
        let order = Self::get_order(size);
        let first_index_for_this_order =
            self.free_indices_for_orders[(order - LOWEST_ORDER) as usize];
        let allocated_entry = if let Some(index) = first_index_for_this_order {
            self.remove_from_free_list(index);
            let entry = self.entries[index as usize];
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
                        let entry = self.entries[first_free_index as usize];
                        break 'result Some((entry, first_free_index));
                    }
                }
                None
            }
        };
        allocated_entry.map(|(entry, index)| {
            self.append_to_allocated_list(entry.as_leaf().order, index);
            entry.as_leaf().address
        })
    }
}
