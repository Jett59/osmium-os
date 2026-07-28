use crate::{
    buddy::BuddyAllocator,
    paging::{MemoryType, PagePermissions},
    physical_memory_manager,
    unsafe_impl::memory::{
        AllocatedMemoryToken, MemoryToken, PhysicalMemoryToken, VirtualMemoryToken,
    },
};

#[cfg(target_arch = "aarch64")]
mod arch {
    use core::arch::asm;

    use bitflags::bitflags;

    use crate::{
        paging::{MemoryType, PagePermissions},
        unsafe_impl::arch::asm,
    };

    pub const PAGE_SIZE: usize = 4096;
    pub const PAGE_TABLE_LEVELS: usize = 4;
    pub const ENTRIES_PER_TABLE: usize = 512;

    /// The recursive page index for the upper half (kernel) is set to 0 by the bootloader.
    const UPPER_RECURSIVE_MAPPING_INDEX: usize = 0;
    const UPPER_RECURSIVE_MAPPING_ADDRESS: *mut u64 = 0xffff_0000_0000_0000 as *mut u64;

    /// For the lower half, we set it to the top of the lower half address space (index 511).
    const LOWER_RECURSIVE_MAPPING_INDEX: usize = 511;
    const LOWER_RECURSIVE_MAPPING_ADDRESS: *mut u64 = 0x0000_ff80_0000_0000 as *mut u64;

    pub const PHYSICAL_PAGE_MASK: u64 = 0x0000_ffff_ffff_f000;

    pub fn recursive_mapping_index(upper_half: bool) -> usize {
        if upper_half {
            UPPER_RECURSIVE_MAPPING_INDEX
        } else {
            LOWER_RECURSIVE_MAPPING_INDEX
        }
    }

    pub fn recursive_mapping_address(upper_half: bool) -> *mut u64 {
        if upper_half {
            UPPER_RECURSIVE_MAPPING_ADDRESS
        } else {
            LOWER_RECURSIVE_MAPPING_ADDRESS
        }
    }

    pub fn is_valid_user_address(address: usize) -> bool {
        address < LOWER_RECURSIVE_MAPPING_ADDRESS as usize
    }

    bitflags! {
        pub struct PageTableFlags: u64 {
            const VALID = 1 << 0;
            const NOT_BLOCK = 1 << 1;

            // Programmed in the MAIR by the bootloader. MAIR[0]=Device-NGNRNE, MAIR[1]=Normal write-back
            const NORMAL_MEMORY = 1 << 2 | 3 << 8;
            const DEVICE_MEMORY = 0 << 2;

            const USER_ACCESSIBLE = 1 << 6;
            const READ_ONLY = 1 << 7;

            const ACCESS = 1 << 10;

            const PRIVILEGED_EXECUTE_NEVER = 1 << 53;
            const USER_EXECUTE_NEVER = 1 << 54;
        }
    }

    impl PageTableFlags {
        pub const INTERNAL_PAGE_TABLE: PageTableFlags = PageTableFlags::VALID
            .union(PageTableFlags::NOT_BLOCK)
            .union(PageTableFlags::ACCESS)
            .union(PageTableFlags::NORMAL_MEMORY);

        pub fn from_type_permissions(
            memory_type: MemoryType,
            permissions: PagePermissions,
        ) -> Self {
            let mut flags = PageTableFlags::VALID
                | PageTableFlags::NOT_BLOCK
                | PageTableFlags::ACCESS
                | match memory_type {
                    MemoryType::Normal => PageTableFlags::NORMAL_MEMORY,
                    MemoryType::Device => PageTableFlags::DEVICE_MEMORY,
                };

            if permissions.user {
                flags |= PageTableFlags::USER_ACCESSIBLE;
                flags |= PageTableFlags::PRIVILEGED_EXECUTE_NEVER;
            } else {
                flags |= PageTableFlags::USER_EXECUTE_NEVER;
            }
            if !permissions.writable {
                flags |= PageTableFlags::READ_ONLY;
            }
            if !permissions.executable {
                flags |= PageTableFlags::PRIVILEGED_EXECUTE_NEVER;
                flags |= PageTableFlags::USER_EXECUTE_NEVER;
            }

            flags
        }

        pub fn is_present(&self) -> bool {
            self.contains(PageTableFlags::VALID)
        }
    }

    pub unsafe fn read_page_table_entry(pointer: *mut u64) -> u64 {
        unsafe { pointer.read_volatile() }
    }
    pub unsafe fn write_page_table_entry(
        pointer: *mut u64,
        value: u64,
        invalidate_tlb: bool,
        virtual_address: Option<usize>,
    ) {
        unsafe { pointer.write_volatile(value) };
        if invalidate_tlb {
            // SAFETY: invalidating the TLB is never unsafe.
            unsafe {
                asm!("tlbi vaae1is, {}", in (reg) virtual_address.unwrap() / 4096, options(nomem, nostack))
            };
        }
        asm::dsb_ish();
        asm::isb();
    }
}

#[cfg(target_arch = "x86_64")]
mod arch {
    use core::arch::asm;

    use bitflags::bitflags;

    use crate::paging::MemoryType;

    pub const PAGE_SIZE: usize = 4096;
    pub const PAGE_TABLE_LEVELS: usize = 4;
    pub const ENTRIES_PER_TABLE: usize = 512;

    const RECURSIVE_PAGE_TABLE_INDEX: usize = 256; // We are in the last 2g so we can't use 511.

    // 256 is the first entry in the higher half of the virtual address space.
    const RECURSIVE_PAGE_TABLE_POINTER: *mut u64 = 0xffff_8000_0000_0000 as *mut u64;

    pub const PHYSICAL_PAGE_MASK: u64 = 0x000f_ffff_ffff_f000;

    pub fn recursive_mapping_index(_upper_half: bool) -> usize {
        RECURSIVE_PAGE_TABLE_INDEX
    }
    pub fn recursive_mapping_address(_upper_half: bool) -> *mut u64 {
        RECURSIVE_PAGE_TABLE_POINTER
    }

    bitflags! {
            #[derive(Clone, Copy)]
            pub struct PageTableFlags: u64 {
    const PRESENT = 1 << 0;
    const WRITEABLE = 1 << 1;
    const USER_ACCESSIBLE = 1 << 2;
    const WRITE_THROUGH = 1 << 3;
    const CACHE_DISABLED = 1 << 4;
    const ACCESSED = 1 << 5;
    const DIRTY = 1 << 6;
    const HUGE_PAGE = 1 << 7;
    const GLOBAL = 1 << 8;
    const NO_EXECUTE = 1 << 63;
            }
        }

    impl PageTableFlags {
        pub const INTERNAL_PAGE_TABLE: PageTableFlags = PageTableFlags::PRESENT
            .union(PageTableFlags::ACCESSED)
            .union(PageTableFlags::WRITEABLE)
            .union(PageTableFlags::USER_ACCESSIBLE);

        pub fn from_type_permissions(
            memory_type: super::MemoryType,
            permissions: super::PagePermissions,
        ) -> Self {
            let mut flags = PageTableFlags::PRESENT | PageTableFlags::ACCESSED;
            if permissions.writable {
                flags |= PageTableFlags::WRITEABLE;
            }
            if !permissions.executable {
                flags |= PageTableFlags::NO_EXECUTE;
            }
            if permissions.user {
                flags |= PageTableFlags::USER_ACCESSIBLE;
            }
            match memory_type {
                MemoryType::Normal => {}
                MemoryType::Device => {
                    flags |= PageTableFlags::CACHE_DISABLED;
                }
            }
            flags
        }

        pub fn is_present(&self) -> bool {
            self.contains(PageTableFlags::PRESENT)
        }
    }

    pub unsafe fn read_page_table_entry(pointer: *mut u64) -> u64 {
        unsafe { pointer.read_volatile() }
    }
    pub unsafe fn write_page_table_entry(
        pointer: *mut u64,
        value: u64,
        invalidate_tlb: bool,
        virtual_address: Option<usize>,
    ) {
        unsafe { pointer.write_volatile(value) };
        if invalidate_tlb {
            // SAFETY: invalidating the TLB is never unsafe.
            unsafe {
                asm!("invlpg [{}]", in (reg) virtual_address.unwrap(), options(nomem, nostack))
            };
        }
    }

    pub fn is_valid_user_address(address: usize) -> bool {
        address < 0x8000_0000_0000_0000
    }
}

use arch::{
    ENTRIES_PER_TABLE, PAGE_TABLE_LEVELS, PHYSICAL_PAGE_MASK, PageTableFlags,
    read_page_table_entry, recursive_mapping_address, recursive_mapping_index,
    write_page_table_entry,
};
pub use arch::{PAGE_SIZE, is_valid_user_address};

#[derive(Clone, Copy)]
struct PageTableIndices {
    upper_half: bool,
    indices: [usize; PAGE_TABLE_LEVELS],
}

impl PageTableIndices {
    fn new(address: usize) -> Self {
        let mut indices = [0; PAGE_TABLE_LEVELS];
        for index in 0..PAGE_TABLE_LEVELS as u32 {
            // I think this is right
            indices[index as usize] = (address
                / (PAGE_SIZE * ENTRIES_PER_TABLE.pow(PAGE_TABLE_LEVELS as u32 - index - 1)))
                % ENTRIES_PER_TABLE;
        }
        Self {
            // Check if the highest bit is set
            upper_half: address.leading_zeros() == 0,
            indices,
        }
    }

    fn calculate_page_table_entry_address(&self) -> *mut u64 {
        // Note that the offset is in units of 8 bytes.
        let mut offset = 0;
        for index in 0..PAGE_TABLE_LEVELS {
            offset += self.indices[index]
                * ENTRIES_PER_TABLE.pow(PAGE_TABLE_LEVELS as u32 - index as u32 - 1);
        }
        // SAFETY: the recursive mapping is guaranteed to be valid, and the offset is guaranteed to be within the bounds of the page table.
        unsafe { recursive_mapping_address(self.upper_half).add(offset) }
    }

    fn calculate_child_page_table_address(&self) -> *mut u64 {
        let mut indices = [0; PAGE_TABLE_LEVELS];
        indices[..PAGE_TABLE_LEVELS - 1].copy_from_slice(&self.indices[1..]);
        PageTableIndices {
            upper_half: self.upper_half,
            indices,
        }
        .calculate_page_table_entry_address()
    }

    fn is_root_page_table(&self) -> bool {
        self.indices
            .iter()
            .all(|&index| index == recursive_mapping_index(self.upper_half))
    }

    /// Returns the indices of the parent page table entry.
    /// The root page table is considered to be its own parent.
    fn parent_indices(&self) -> PageTableIndices {
        // The 0-th index should be set to the recursive mapping index, and everything else should be shifted across by one.
        let mut parent_indices = [0; PAGE_TABLE_LEVELS];
        parent_indices[0] = recursive_mapping_index(self.upper_half);
        parent_indices[1..].copy_from_slice(&self.indices[..PAGE_TABLE_LEVELS - 1]);
        PageTableIndices {
            upper_half: self.upper_half,
            indices: parent_indices,
        }
    }

    /// Returns the indices of the ancestors of the page table entry, including itself.
    /// They are returned in order from the root page table to the leaf page table entry.
    fn ancestor_indices(&self) -> impl Iterator<Item = PageTableIndices> {
        let mut ancestors_backwards = [*self; PAGE_TABLE_LEVELS];
        for index in 1..PAGE_TABLE_LEVELS {
            ancestors_backwards[index] = ancestors_backwards[index - 1].parent_indices();
        }
        ancestors_backwards
            .into_iter()
            .rev()
            .filter(|indices| !indices.is_root_page_table())
    }
}

static PAGE_TABLE_ALLOCATION_POOL: spin::Mutex<
    BuddyAllocator<PhysicalMemoryToken, 128, { physical_memory_manager::LOG2_BLOCK_SIZE }, 12>,
> = spin::Mutex::new(BuddyAllocator::new());

fn allocate_page_table() -> PhysicalMemoryToken {
    let mut page_allocation_pool = PAGE_TABLE_ALLOCATION_POOL.lock();
    if let Some(allocated_page) = page_allocation_pool.allocate(4096) {
        allocated_page
    } else {
        page_allocation_pool.add_entry(
            physical_memory_manager::allocate_block()
                .expect("Failed to get physical memory for page tables"),
        );
        page_allocation_pool
            .allocate(4096)
            .expect("Adding new entry to page table allocation pool didn't change anything")
    }
}
fn free_page_table(table: PhysicalMemoryToken) {
    let mut page_allocation_pool = PAGE_TABLE_ALLOCATION_POOL.lock();
    page_allocation_pool.free(table);
    // If this merged into a 64 kb block, return it to the physical memory manager (PMM) so it can be used by someone else.
    if let Some(free_block) = page_allocation_pool.allocate(physical_memory_manager::BLOCK_SIZE) {
        // Unlock the page allocation pool
        drop(page_allocation_pool);
        physical_memory_manager::mark_as_free(free_block);
    }
}

/// Module to ensure that paging code cannot fake a lock
///
/// TODO: make this into a token-based system to demonstrate uniqueness of page tables.
/// This would be too complex for now.
mod _lock {
    use core::marker::PhantomData;

    pub struct Lock(PhantomData<()>);

    pub static LOCK: spin::RwLock<Lock> = spin::RwLock::new(Lock(PhantomData));
}

use _lock::*;

fn is_page_table_entry_present(indices: PageTableIndices, _lock: &Lock) -> bool {
    // SAFETY: each check ensures the validity of the next check.
    // Since we hold the lock, there is no chance of memory races.
    unsafe {
        for ancestor_indices in indices.ancestor_indices() {
            let entry =
                read_page_table_entry(ancestor_indices.calculate_page_table_entry_address());
            if !PageTableFlags::from_bits_truncate(entry).is_present() {
                return false;
            }
        }
    }
    true
}

/// Create a page table at the given indices.
/// This means that it should not point to a non-paging-related region of virtual memory.
/// # Safety
/// The caller must ensure that all previous levels of the page table are present for the given indices.
unsafe fn create_page_table(indices: PageTableIndices, _lock: &mut Lock) {
    let entry_address = indices.calculate_page_table_entry_address();
    let physical_page_table = allocate_page_table();
    let flags = PageTableFlags::INTERNAL_PAGE_TABLE;
    let entry = flags.bits() | (physical_page_table.address() as u64 & PHYSICAL_PAGE_MASK);
    // SAFETY: the caller ensured that the parent page table is present.
    // We hold the lock, so there is no chance of data races.
    // So long as we zero out the page table before releasing the lock, this will not create any extraneous mappings.
    unsafe { write_page_table_entry(entry_address, entry, false, None) };

    let page_table_address = indices.calculate_child_page_table_address();
    // SAFETY: it is guaranteed that the page table now exists, and we still hold the lock.
    // Note that `write_bytes` is in `u64` units.
    unsafe { core::ptr::write_bytes(page_table_address, 0, 512) };
}

/// Ensure that all page tables exist for the given indices.
/// For example, if `indices` refers to some non-paging-related virtual memory, then this function will ensure that the leaf-level page table for that region exists.
fn ensure_page_tables_exist(indices: PageTableIndices, lock: &mut Lock) {
    for ancestor_indices in indices.parent_indices().ancestor_indices() {
        if !is_page_table_entry_present(ancestor_indices, lock) {
            // SAFETY: we know that the parent table exists, but that this page table does not exist.
            unsafe { create_page_table(ancestor_indices, lock) };
        }
    }
}

pub fn create_page_mapping(
    memory_type: MemoryType,
    permissions: PagePermissions,
    physical_address: PhysicalMemoryToken,
    virtual_address: VirtualMemoryToken,
) -> AllocatedMemoryToken {
    assert!(
        virtual_address.address().is_multiple_of(PAGE_SIZE),
        "Virtual address must be page-aligned"
    );
    assert_eq!(
        virtual_address.size(),
        PAGE_SIZE,
        "Virtual address must be a single page"
    );
    assert!(
        physical_address.address().is_multiple_of(PAGE_SIZE),
        "Physical address must be page-aligned"
    );
    assert_eq!(
        physical_address.size(),
        PAGE_SIZE,
        "Physical address must be a single page"
    );

    let mut lock = LOCK.write();
    let indices = PageTableIndices::new(virtual_address.address());
    let flags = PageTableFlags::from_type_permissions(memory_type, permissions);
    let physical_address = physical_address.address() as u64;
    let entry = flags.bits() | physical_address & PHYSICAL_PAGE_MASK;
    let entry_address = indices.calculate_page_table_entry_address();
    ensure_page_tables_exist(indices, &mut lock);
    // SAFETY: the pointer is guaranteed to point to a page table entry by `ensure_page_tables_exist`.
    // The physical and virtual addresses are guaranteed to be valid pages owned by the caller.
    unsafe { write_page_table_entry(entry_address, entry, false, None) };

    // SAFETY: the mapping is now present, so the caller can safely use the address.
    unsafe { AllocatedMemoryToken::new(virtual_address.address(), 4096) }
}

pub fn take_page_mapping(
    allocated_address: AllocatedMemoryToken,
) -> (PhysicalMemoryToken, VirtualMemoryToken) {
    assert!(
        allocated_address.address().is_multiple_of(PAGE_SIZE),
        "Allocated address must be page-aligned"
    );
    assert_eq!(
        allocated_address.size(),
        PAGE_SIZE,
        "Allocated address must be a single page"
    );

    let indices = PageTableIndices::new(allocated_address.address());
    let entry_address = indices.calculate_page_table_entry_address();
    assert!(
        is_page_table_entry_present(indices, &LOCK.read()),
        "Taking a page which is not mapped!"
    );
    // SAFETY: we just checked that the page table entry exists.
    let entry = unsafe { read_page_table_entry(entry_address) };
    // SAFETY: since we now own the allocation through the token, we are allowed to de-allocate it
    unsafe { write_page_table_entry(entry_address, 0, true, Some(allocated_address.address())) };

    let flags = PageTableFlags::from_bits_truncate(entry);
    assert!(flags.is_present(), "Taking a page which is not mapped!");
    let physical_address = entry & PHYSICAL_PAGE_MASK;
    // SAFETY: the physical and virtual addresses are guaranteed to be owned by the caller.
    unsafe {
        (
            PhysicalMemoryToken::new(physical_address as usize, 4096),
            VirtualMemoryToken::new(allocated_address.address(), 4096),
        )
    }
}

pub mod init {
    use super::*;

    #[cfg(target_arch = "aarch64")]
    pub fn initialize_lower_half_table() {
        use crate::memory_allocator::map_physical_memory;
        use crate::unsafe_impl::arch::asm;
        // We need to set the TTBR0_EL1 register to a newly allocated page table.
        // We also need to put the recursive mapping in it, so we need access first.
        let page_table_address = allocate_page_table();
        unsafe {
            let recursive_mapping_entry_flags = PageTableFlags::VALID
                | PageTableFlags::NOT_BLOCK
                | PageTableFlags::NORMAL_MEMORY
                | PageTableFlags::ACCESS;
            let recursive_mapping_entry =
                recursive_mapping_entry_flags.bits() | page_table_address.address() as u64;
            let mut page_table_handle = map_physical_memory(
                page_table_address.address(),
                PAGE_SIZE,
                MemoryType::Normal,
                PagePermissions::KERNEL_READ_WRITE,
            );
            // Zero it out
            page_table_handle.fill(0);
            let final_entry: &mut [u8; 8] = (&mut page_table_handle[PAGE_SIZE - 8..])
                .try_into()
                .unwrap();
            *final_entry = recursive_mapping_entry.to_ne_bytes();
        }
        unsafe {
            asm::write_ttbr0(page_table_address.address() as u64);
        }
    }

    #[cfg(target_arch = "x86_64")]
    pub fn initialize_paging() {
        // We must remove some of the mappings the startup code used (there is one which maps the first gigabyte exactly like the last, and one which maps the first 512g likewise).
        // First remove the mapping of the low 512g:
        unsafe {
            let indices = PageTableIndices {
                upper_half: false,
                indices: [
                    recursive_mapping_index(false),
                    recursive_mapping_index(false),
                    recursive_mapping_index(false),
                    0,
                ],
            };
            write_page_table_entry(
                indices.calculate_page_table_entry_address(),
                0,
                true,
                Some(0),
            );
        }
        // Then for the 511th pml4, 0th pml3:
        unsafe {
            let indices = PageTableIndices {
                upper_half: false,
                indices: [
                    recursive_mapping_index(false),
                    recursive_mapping_index(false),
                    511,
                    0,
                ],
            };
            write_page_table_entry(
                indices.calculate_page_table_entry_address(),
                0,
                true,
                Some(0xffff_ff80_0000_0000),
            );
        }
    }
}
