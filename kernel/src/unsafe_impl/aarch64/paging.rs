use core::arch::asm;

use bitflags::bitflags;
use spin::LazyLock;

use crate::{
    buddy::BuddyAllocator,
    paging::{MemoryType, PagePermissions},
    physical_memory_manager,
    unsafe_impl::{
        arch::asm,
        memory::{AllocatedMemoryToken, MemoryToken, PhysicalMemoryToken, VirtualMemoryToken},
    },
};

pub const PAGE_SIZE: usize = 4096;

/// The recursive page index for the upper half (kernel) is set to 0 by the bootloader.
const UPPER_RECURSIVE_MAPPING_INDEX: usize = 0;
const UPPER_RECURSIVE_MAPPING_ADDRESS: *mut u64 = 0xffff_0000_0000_0000 as *mut u64;

/// For the lower half, we set it to the top of the lower half address space (index 511).
const LOWER_RECURSIVE_MAPPING_INDEX: usize = 511;
const LOWER_RECURSIVE_MAPPING_ADDRESS: *mut u64 = 0x0000_ff80_0000_0000 as *mut u64;

const PHYSICAL_PAGE_MASK: u64 = 0x0000_ffff_ffff_f000;

fn recursive_mapping_index(upper_half: bool) -> usize {
    if upper_half {
        UPPER_RECURSIVE_MAPPING_INDEX
    } else {
        LOWER_RECURSIVE_MAPPING_INDEX
    }
}

bitflags! {
    struct PageTableFlags: u64 {
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
    fn from_type_permissions(memory_type: MemoryType, permissions: PagePermissions) -> Self {
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
}

#[derive(Clone, Copy)]
struct PageTableIndices {
    upper_half: bool,
    level_0_index: usize,
    level_1_index: usize,
    level_2_index: usize,
    level_3_index: usize,
}

impl PageTableIndices {
    fn new(address: usize) -> Self {
        Self {
            upper_half: address >= 0x8000_0000_0000_0000,
            level_0_index: (address >> 39) & 0x1ff,
            level_1_index: (address >> 30) & 0x1ff,
            level_2_index: (address >> 21) & 0x1ff,
            level_3_index: (address >> 12) & 0x1ff,
        }
    }

    fn address(&self) -> usize {
        let mut address = 0;
        if self.upper_half {
            address |= 0xffff_0000_0000_0000;
        }
        address |= self.level_0_index << 39;
        address |= self.level_1_index << 30;
        address |= self.level_2_index << 21;
        address |= self.level_3_index << 12;
        address
    }

    fn calculate_page_table_entry_address(&self) -> *mut u64 {
        // Note that the offset is in units of 8 bytes.
        let offset = self.level_3_index
            + self.level_2_index * 512
            + self.level_1_index * 512 * 512
            + self.level_0_index * 512 * 512 * 512;
        if self.upper_half {
            // SAFETY: the `new` function ensures that the indices are in the valid range of 0..512, so the offset will always be within the bounds of the recursive mapping.
            unsafe { UPPER_RECURSIVE_MAPPING_ADDRESS.add(offset) }
        } else {
            // SAFETY: same as above
            unsafe { LOWER_RECURSIVE_MAPPING_ADDRESS.add(offset) }
        }
    }

    fn calculate_child_page_table_address(&self) -> *mut u64 {
        let indices = PageTableIndices {
            upper_half: self.upper_half,
            level_0_index: self.level_1_index,
            level_1_index: self.level_2_index,
            level_2_index: self.level_3_index,
            level_3_index: 0,
        };
        indices.calculate_page_table_entry_address()
    }

    fn is_root_page_table(&self) -> bool {
        self.level_0_index == recursive_mapping_index(self.upper_half)
            && self.level_1_index == recursive_mapping_index(self.upper_half)
            && self.level_2_index == recursive_mapping_index(self.upper_half)
            && self.level_3_index == recursive_mapping_index(self.upper_half)
    }

    /// Returns the indices of the parent page table entry.
    /// The root page table is considered to be its own parent.
    fn parent_indices(&self) -> PageTableIndices {
        PageTableIndices {
            upper_half: self.upper_half,
            level_0_index: recursive_mapping_index(self.upper_half),
            level_1_index: self.level_0_index,
            level_2_index: self.level_1_index,
            level_3_index: self.level_2_index,
        }
    }

    /// Returns the indices of the ancestors of the page table entry, including itself.
    /// They are returned in order from the root page table to the leaf page table entry.
    fn ancestor_indices(&self) -> impl Iterator<Item = PageTableIndices> {
        [
            self.parent_indices().parent_indices().parent_indices(),
            self.parent_indices().parent_indices(),
            self.parent_indices(),
            *self,
        ]
        .into_iter()
        .filter(|indices| !indices.is_root_page_table())
    }
}

static PAGE_TABLE_ALLOCATION_POOL: LazyLock<
    &'static spin::Mutex<
        BuddyAllocator<PhysicalMemoryToken, 128, { physical_memory_manager::LOG2_BLOCK_SIZE }, 12>,
    >,
> = LazyLock::new(|| {
    static ACTUAL_ALLOCATOR: spin::Mutex<BuddyAllocator<PhysicalMemoryToken, 128, 16, 12>> =
        spin::Mutex::new(BuddyAllocator::unusable());
    ACTUAL_ALLOCATOR.lock().all_unused();
    &ACTUAL_ALLOCATOR
});

fn allocate_page_table() -> PhysicalMemoryToken {
    let mut page_allocation_pool = PAGE_TABLE_ALLOCATION_POOL.lock();
    if let Some(allocated_page) = page_allocation_pool.allocate(4096) {
        allocated_page
    } else {
        unsafe {
            page_allocation_pool.add_entry(PhysicalMemoryToken::new(
                physical_memory_manager::allocate_block_address()
                    .expect("Failed to get physical memory for page tables"),
                physical_memory_manager::BLOCK_SIZE,
            ));
            page_allocation_pool
                .allocate(4096)
                .expect("Adding new entry to page table allocation pool didn't change anything")
        }
    }
}
fn free_page_table(table: PhysicalMemoryToken) {
    let mut page_allocation_pool = PAGE_TABLE_ALLOCATION_POOL.lock();
    page_allocation_pool.free(table);
    // If this merged into a 64 kb block, return it to the physical memory manager (PMM) so it can be used by someone else.
    if let Some(free_block) = page_allocation_pool.allocate(physical_memory_manager::BLOCK_SIZE) {
        // Unlock the page allocation pool
        drop(page_allocation_pool);
        physical_memory_manager::mark_as_free(free_block.address());
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
            let entry = *ancestor_indices.calculate_page_table_entry_address();
            if !PageTableFlags::from_bits_truncate(entry).contains(PageTableFlags::VALID) {
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
    let flags = PageTableFlags::VALID
        | PageTableFlags::NOT_BLOCK
        | PageTableFlags::ACCESS
        | PageTableFlags::NORMAL_MEMORY;
    let entry = flags.bits() | (physical_page_table.address() as u64 & PHYSICAL_PAGE_MASK);
    // SAFETY: the caller ensured that the parent page table is present.
    // We hold the lock, so there is no chance of data races.
    // So long as we zero out the page table before releasing the lock, this will not create any extraneous mappings.
    unsafe { *entry_address = entry };
        asm::dsb_ish();
    asm::isb();

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
        virtual_address.address().is_multiple_of(4096),
        "Virtual address must be page-aligned"
    );
    assert_eq!(
        virtual_address.size(),
        4096,
        "Virtual address must be a single page"
    );
    assert!(
        physical_address.address().is_multiple_of(4096),
        "Physical address must be page-aligned"
    );
    assert_eq!(
        physical_address.size(),
        4096,
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
    // You must trust me that I got the right format of a page table entry.
    unsafe { *entry_address = entry };
    asm::dsb_ish();
    asm::isb();
    // SAFETY: the mapping is now present, so the caller can safely use the address.
    unsafe { AllocatedMemoryToken::new(virtual_address.address(), 4096) }
}

pub fn take_page_mapping(
    allocated_address: AllocatedMemoryToken,
) -> (PhysicalMemoryToken, VirtualMemoryToken) {
    assert!(
        allocated_address.address().is_multiple_of(4096),
        "Allocated address must be page-aligned"
    );
    assert_eq!(
        allocated_address.size(),
        4096,
        "Allocated address must be a single page"
    );

    let indices = PageTableIndices::new(allocated_address.address());
    let entry_address = indices.calculate_page_table_entry_address();
    assert!(
        is_page_table_entry_present(indices, &LOCK.read()),
        "Taking a page which is not mapped!"
    );
    // SAFETY: we just checked that the page table entry exists.
    let entry = unsafe { *entry_address };
    // SAFETY: since we now own the allocation through the token, we are allowed to de-allocate it
    unsafe { *entry_address = 0 };
    // SAFETY: invalidating the TLB is never unsafe.
    unsafe {
        asm!("tlbi vaae1is, {}", in (reg) allocated_address.address() / 4096, options(nomem, nostack))
    };
    asm::dsb_ish();
    asm::isb();

    let flags = PageTableFlags::from_bits_truncate(entry);
    assert!(
        flags.contains(PageTableFlags::VALID),
        "Taking a page which is not mapped!"
    );
    let physical_address = entry & PHYSICAL_PAGE_MASK;
    // SAFETY: the physical and virtual addresses are guaranteed to be owned by the caller.
    unsafe {
        (
            PhysicalMemoryToken::new(physical_address as usize, 4096),
            VirtualMemoryToken::new(allocated_address.address(), 4096),
        )
    }
}

pub fn is_valid_user_address(address: usize) -> bool {
    address < LOWER_RECURSIVE_MAPPING_ADDRESS as usize
}

pub mod init {
    use crate::heap::map_physical_memory;

    use super::*;

    pub fn initialize_lower_half_table() {
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
}
