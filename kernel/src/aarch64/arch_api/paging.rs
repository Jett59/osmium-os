use core::arch::asm;

use bitflags::bitflags;

use crate::{buddy::BuddyAllocator, lazy_init::lazy_static, physical_memory_manager};

pub const PAGE_SIZE: usize = 4096;

const UPPER_RECURSIVE_MAPPING_INDEX: usize = 0;
const UPPER_RECURSIVE_MAPPING_ADDRESS: *mut u64 = 0xffff_0000_0000_0000 as *mut u64;

const PHYSICAL_PAGE_MASK: u64 = 0x0000_ffff_ffff_f000;

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

        const EXECUTE_NEVER = 3 << 53;
    }
}

unsafe fn write_upper_page_table_entry(
    flags: PageTableFlags,
    physical_address: u64,
    level_0_index: usize,
    level_1_index: usize,
    level_2_index: usize,
    level_3_index: usize,
) {
    let offset = level_3_index
        + level_2_index * 512
        + level_1_index * 512 * 512
        + level_0_index * 512 * 512 * 512;
    let entry = flags.bits() | physical_address & PHYSICAL_PAGE_MASK;
    *UPPER_RECURSIVE_MAPPING_ADDRESS.add(offset) = entry;
}

/// Read the flags and physical address from a page table entry.
unsafe fn read_upper_page_table_entry(
    level_0_index: usize,
    level_1_index: usize,
    level_2_index: usize,
    level_3_index: usize,
) -> (PageTableFlags, u64) {
    let offset = level_3_index
        + level_2_index * 512
        + level_1_index * 512 * 512
        + level_0_index * 512 * 512 * 512;
    let entry = *UPPER_RECURSIVE_MAPPING_ADDRESS.add(offset);
    let flags = PageTableFlags::from_bits_truncate(entry);
    let physical_address = entry & PHYSICAL_PAGE_MASK;
    (flags, physical_address)
}

unsafe fn get_upper_page_table_entry_address(
    level_0_index: usize,
    level_1_index: usize,
    level_2_index: usize,
    level_3_index: usize,
) -> *mut u64 {
    let offset = level_3_index
        + level_2_index * 512
        + level_1_index * 512 * 512
        + level_0_index * 512 * 512 * 512;
    UPPER_RECURSIVE_MAPPING_ADDRESS.add(offset)
}

struct PageTableIndices {
    upper_half: bool,
    level_0_index: usize,
    level_1_index: usize,
    level_2_index: usize,
    level_3_index: usize,
}

fn deconstruct_virtual_address(address: usize) -> PageTableIndices {
    PageTableIndices {
        upper_half: address >= 0xffff_0000_0000_0000,
        level_0_index: (address >> 39) & 0x1ff,
        level_1_index: (address >> 30) & 0x1ff,
        level_2_index: (address >> 21) & 0x1ff,
        level_3_index: (address >> 12) & 0x1ff,
    }
}

lazy_static! {
    static ref PAGE_TABLE_ALLOCATION_POOL: &'static mut BuddyAllocator<128, { physical_memory_manager::LOG2_BLOCK_SIZE }, 12> = {
        static mut ACTUAL_ALLOCATOR: BuddyAllocator<128, 16, 12> = BuddyAllocator::unusable();
        unsafe { ACTUAL_ALLOCATOR.all_unused() }
    };
}

fn allocate_page_table() -> usize {
    unsafe {
        if let Some(allocated_page) = PAGE_TABLE_ALLOCATION_POOL.allocate(4096) {
            allocated_page
        } else {
            PAGE_TABLE_ALLOCATION_POOL.add_entry(
                physical_memory_manager::BLOCK_SIZE,
                physical_memory_manager::allocate_block_address()
                    .expect("Failed to get physical memory for page tables"),
            );
            PAGE_TABLE_ALLOCATION_POOL
                .allocate(4096)
                .expect("Adding new entry to page table allocation pool didn't change anything")
        }
    }
}
fn free_page_table(address: usize) {
    unsafe {
        PAGE_TABLE_ALLOCATION_POOL.free(4096, address);
        // If this merged into a 64 kb block, return it to the phyical memory manager (PMM) so it can be used by someone else.
        if let Some(free_block) =
            PAGE_TABLE_ALLOCATION_POOL.allocate(physical_memory_manager::BLOCK_SIZE)
        {
            physical_memory_manager::mark_as_free(free_block);
        }
    }
}

fn ensure_page_table_exists(level_0_index: usize, level_1_index: usize, level_2_index: usize) {
    // Indexing with the first indices set to RECURSIVE_PAGE_TABLE_INDEX will give us the next layer up in the page tables.
    fn create_page_table_if_absent(
        level_0_index: usize,
        level_1_index: usize,
        level_2_index: usize,
    ) {
        let (flags, _) = unsafe {
            read_upper_page_table_entry(
                UPPER_RECURSIVE_MAPPING_INDEX,
                level_0_index,
                level_1_index,
                level_2_index,
            )
        };
        if !flags.contains(PageTableFlags::VALID) {
            unsafe {
                write_upper_page_table_entry(
                    PageTableFlags::VALID
                        | PageTableFlags::NOT_BLOCK
                        | PageTableFlags::NORMAL_MEMORY
                        | PageTableFlags::ACCESS,
                    allocate_page_table() as u64,
                    UPPER_RECURSIVE_MAPPING_INDEX,
                    level_0_index,
                    level_1_index,
                    level_2_index,
                );
                let address = get_upper_page_table_entry_address(
                    level_0_index,
                    level_1_index,
                    level_2_index,
                    0,
                );
                core::ptr::write_bytes(address as *mut u8, 0, PAGE_SIZE);
            }
        }
    }

    create_page_table_if_absent(
        UPPER_RECURSIVE_MAPPING_INDEX,
        UPPER_RECURSIVE_MAPPING_INDEX,
        level_0_index,
    );
    create_page_table_if_absent(UPPER_RECURSIVE_MAPPING_INDEX, level_0_index, level_1_index);
    create_page_table_if_absent(level_0_index, level_1_index, level_2_index);
}

pub fn map_page(virtual_address: usize, physical_address: usize) {
    unsafe {
        let indices = deconstruct_virtual_address(virtual_address);
        assert!(indices.upper_half, "Lower half not supported");
        ensure_page_table_exists(
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
        );
        let (flags, _) = read_upper_page_table_entry(
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
            indices.level_3_index,
        );
        if flags.contains(PageTableFlags::VALID) {
            panic!("Remapping a page which is already mapped!");
        }
        write_upper_page_table_entry(
            PageTableFlags::VALID
                | PageTableFlags::NOT_BLOCK
                | PageTableFlags::NORMAL_MEMORY
                | PageTableFlags::ACCESS,
            physical_address as u64,
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
            indices.level_3_index,
        );
    }
}

fn is_page_table_present(level_0_index: usize, level_1_index: usize, level_2_index: usize) -> bool {
    fn check_table(level_0_index: usize, level_1_index: usize, level_2_index: usize) -> bool {
        let (flags, _) = unsafe {
            read_upper_page_table_entry(
                UPPER_RECURSIVE_MAPPING_INDEX,
                level_0_index,
                level_1_index,
                level_2_index,
            )
        };
        flags.contains(PageTableFlags::VALID)
    }

    check_table(
        UPPER_RECURSIVE_MAPPING_INDEX,
        UPPER_RECURSIVE_MAPPING_INDEX,
        level_0_index,
    ) && check_table(UPPER_RECURSIVE_MAPPING_INDEX, level_0_index, level_1_index)
        && check_table(level_0_index, level_1_index, level_2_index)
}

unsafe fn invalidate_tlb(address: usize) {
    asm!("tlbi vae1, {}", in(reg) address, options(nomem, nostack));
}

pub fn unmap_page(virtual_address: usize) {
    unsafe {
        let indices = deconstruct_virtual_address(virtual_address);
        assert!(indices.upper_half, "Lower half not supported");
        if !is_page_table_present(
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
        ) {
            panic!("Unmapping a page which is not mapped!");
        }
        let (flags, _) = read_upper_page_table_entry(
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
            indices.level_3_index,
        );
        if !flags.contains(PageTableFlags::VALID) {
            panic!("Unmapping a page which is not mapped!");
        }
        write_upper_page_table_entry(
            PageTableFlags::empty(),
            0,
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
            indices.level_3_index,
        );
        invalidate_tlb(virtual_address);
    }
}

pub fn get_physical_address(_virtual_address: usize) -> usize {
    unimplemented!();
}
