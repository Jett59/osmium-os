#[derive(Clone, Copy)]
struct LeafBuddyEntry {
    size: u32,
    free: bool,
    sibling: u32,
    parent: u32,
    next_of_this_size: u32,
    previous_of_this_size: u32,
}
#[derive(Clone, Copy)]
struct ParentBuddyEntry {
    size: u32,
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

struct BuddyAllocator<const CAPACITY: u32, const HIGHEST_ORDER: usize, const LOWEST_ORDER: usize>
where
    [(); HIGHEST_ORDER - LOWEST_ORDER + 1]:,
    [(); CAPACITY as usize]:,
{
    entries: [BuddyEntry; CAPACITY as usize],
    first_free_indices_for_orders: [Option<u32>; HIGHEST_ORDER - LOWEST_ORDER + 1],
    unused_entries: Option<u32>,
}

impl<const CAPACITY: u32, const HIGHEST_ORDER: usize, const LOWEST_ORDER: usize>
    BuddyAllocator<CAPACITY, HIGHEST_ORDER, LOWEST_ORDER>
where
    [(); HIGHEST_ORDER - LOWEST_ORDER + 1]:,
    [(); CAPACITY as usize]:,
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
            }); CAPACITY as usize],
            first_free_indices_for_orders: [None; HIGHEST_ORDER - LOWEST_ORDER + 1],
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
}
