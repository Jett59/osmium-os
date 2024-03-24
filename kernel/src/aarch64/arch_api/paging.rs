use core::arch::asm;

use bitflags::bitflags;

use crate::{
    arch::asm, buddy::BuddyAllocator, heap::map_physical_memory, lazy_init::lazy_static,
    physical_memory_manager,
};

pub const PAGE_SIZE: usize = 4096;

const UPPER_RECURSIVE_MAPPING_INDEX: usize = 0;
const UPPER_RECURSIVE_MAPPING_ADDRESS: *mut u64 = 0xffff_0000_0000_0000 as *mut u64;

const LOWER_RECURSIVE_MAPPING_INDEX: usize = 511;
const LOWER_RECURSIVE_MAPPING_ADDRESS: *mut u64 = 0x0000_ff80_0000_0000 as *mut u64;

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

fn recursive_mapping_index(upper_half: bool) -> usize {
    if upper_half {
        UPPER_RECURSIVE_MAPPING_INDEX
    } else {
        LOWER_RECURSIVE_MAPPING_INDEX
    }
}

unsafe fn write_page_table_entry(
    flags: PageTableFlags,
    physical_address: u64,
    upper_half: bool,
    level_0_index: usize,
    level_1_index: usize,
    level_2_index: usize,
    level_3_index: usize,
) {
    let entry = flags.bits() | physical_address & PHYSICAL_PAGE_MASK;
    let entry_address = calculate_page_table_entry_address(
        upper_half,
        level_0_index,
        level_1_index,
        level_2_index,
        level_3_index,
    );
    *entry_address = entry;
    asm::dsb_ish();
    asm::isb();
}

/// Read the flags and physical address from a page table entry.
unsafe fn read_page_table_entry(
    upper_half: bool,
    level_0_index: usize,
    level_1_index: usize,
    level_2_index: usize,
    level_3_index: usize,
) -> (PageTableFlags, u64) {
    let entry = *calculate_page_table_entry_address(
        upper_half,
        level_0_index,
        level_1_index,
        level_2_index,
        level_3_index,
    );
    let flags = PageTableFlags::from_bits_truncate(entry);
    let physical_address = entry & PHYSICAL_PAGE_MASK;
    (flags, physical_address)
}

unsafe fn calculate_page_table_entry_address(
    upper_half: bool,
    level_0_index: usize,
    level_1_index: usize,
    level_2_index: usize,
    level_3_index: usize,
) -> *mut u64 {
    let offset = level_3_index
        + level_2_index * 512
        + level_1_index * 512 * 512
        + level_0_index * 512 * 512 * 512;
    if upper_half {
        UPPER_RECURSIVE_MAPPING_ADDRESS.add(offset)
    } else {
        LOWER_RECURSIVE_MAPPING_ADDRESS.add(offset)
    }
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
        // If this merged into a 64 kb block, return it to the physical memory manager (PMM) so it can be used by someone else.
        if let Some(free_block) =
            PAGE_TABLE_ALLOCATION_POOL.allocate(physical_memory_manager::BLOCK_SIZE)
        {
            physical_memory_manager::mark_as_free(free_block);
        }
    }
}

fn ensure_page_table_exists(
    upper_half: bool,
    level_0_index: usize,
    level_1_index: usize,
    level_2_index: usize,
) {
    // Indexing with the first indices set to RECURSIVE_PAGE_TABLE_INDEX will give us the next layer up in the page tables.
    fn create_page_table_if_absent(
        upper_half: bool,
        level_0_index: usize,
        level_1_index: usize,
        level_2_index: usize,
    ) {
        let recursive_index = recursive_mapping_index(upper_half);
        let (flags, _) = unsafe {
            read_page_table_entry(
                upper_half,
                recursive_index,
                level_0_index,
                level_1_index,
                level_2_index,
            )
        };
        if !flags.contains(PageTableFlags::VALID) {
            unsafe {
                write_page_table_entry(
                    PageTableFlags::VALID
                        | PageTableFlags::NOT_BLOCK
                        | PageTableFlags::NORMAL_MEMORY
                        | PageTableFlags::ACCESS,
                    allocate_page_table() as u64,
                    upper_half,
                    recursive_index,
                    level_0_index,
                    level_1_index,
                    level_2_index,
                );
                let address = calculate_page_table_entry_address(
                    upper_half,
                    level_0_index,
                    level_1_index,
                    level_2_index,
                    0,
                );
                (address as *mut u8).write_bytes(0, PAGE_SIZE);
            }
        }
    }

    let recursive_index = recursive_mapping_index(upper_half);

    create_page_table_if_absent(upper_half, recursive_index, recursive_index, level_0_index);
    create_page_table_if_absent(upper_half, recursive_index, level_0_index, level_1_index);
    create_page_table_if_absent(upper_half, level_0_index, level_1_index, level_2_index);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MemoryType {
    Normal,
    Device,
}

pub fn map_page(virtual_address: usize, physical_address: usize, memory_type: MemoryType) {
    unsafe {
        let indices = deconstruct_virtual_address(virtual_address);
        ensure_page_table_exists(
            indices.upper_half,
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
        );
        let (flags, _) = read_page_table_entry(
            indices.upper_half,
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
            indices.level_3_index,
        );
        if flags.contains(PageTableFlags::VALID) {
            panic!("Remapping a page which is already mapped!");
        }
        let mut flags = PageTableFlags::VALID | PageTableFlags::NOT_BLOCK | PageTableFlags::ACCESS;
        match memory_type {
            MemoryType::Normal => flags |= PageTableFlags::NORMAL_MEMORY,
            MemoryType::Device => flags |= PageTableFlags::DEVICE_MEMORY,
        }
        if !indices.upper_half {
            flags |= PageTableFlags::USER_ACCESSIBLE;
        }
        write_page_table_entry(
            flags,
            physical_address as u64,
            indices.upper_half,
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
            indices.level_3_index,
        );
    }
}

fn is_page_table_present(
    upper_half: bool,
    level_0_index: usize,
    level_1_index: usize,
    level_2_index: usize,
) -> bool {
    fn check_table(
        upper_half: bool,
        level_0_index: usize,
        level_1_index: usize,
        level_2_index: usize,
    ) -> bool {
        let recursive_index = recursive_mapping_index(upper_half);
        let (flags, _) = unsafe {
            read_page_table_entry(
                upper_half,
                recursive_index,
                level_0_index,
                level_1_index,
                level_2_index,
            )
        };
        flags.contains(PageTableFlags::VALID)
    }

    let recursive_index = recursive_mapping_index(upper_half);

    check_table(upper_half, recursive_index, recursive_index, level_0_index)
        && check_table(upper_half, recursive_index, level_0_index, level_1_index)
        && check_table(upper_half, level_0_index, level_1_index, level_2_index)
}

unsafe fn invalidate_tlb(address: usize) {
    asm!("tlbi vaae1is, {}", in (reg) address / PAGE_SIZE, options(nomem, nostack));
    asm::dsb_ish();
    asm::isb();
}

pub fn unmap_page(virtual_address: usize) {
    unsafe {
        let indices = deconstruct_virtual_address(virtual_address);
        if !is_page_table_present(
            indices.upper_half,
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
        ) {
            panic!("Unmapping a page which is not mapped!");
        }
        let (flags, _) = read_page_table_entry(
            indices.upper_half,
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
            indices.level_3_index,
        );
        if !flags.contains(PageTableFlags::VALID) {
            panic!("Unmapping a page which is not mapped!");
        }
        write_page_table_entry(
            PageTableFlags::empty(),
            0,
            indices.upper_half,
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
            indices.level_3_index,
        );
        invalidate_tlb(virtual_address);
    }
}

pub fn get_physical_address(virtual_address: usize) -> usize {
    unsafe {
        let indices = deconstruct_virtual_address(virtual_address);
        if !is_page_table_present(
            indices.upper_half,
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
        ) {
            panic!("Getting the physical address of a page which is not mapped!");
        }
        let (flags, physical_address) = read_page_table_entry(
            indices.upper_half,
            indices.level_0_index,
            indices.level_1_index,
            indices.level_2_index,
            indices.level_3_index,
        );
        if !flags.contains(PageTableFlags::VALID) {
            panic!("Getting the physical address of a page which is not mapped!");
        }
        physical_address as usize
    }
}

pub(in crate::arch) fn initialize_lower_half_table() {
    // We need to set the TTBR0_EL1 register to a newly allocated page table.
    // We also need to put the recursive mapping in it, so we need access first.
    let page_table_address = allocate_page_table();
    unsafe {
        let recursive_mapping_entry_flags = PageTableFlags::VALID
            | PageTableFlags::NOT_BLOCK
            | PageTableFlags::NORMAL_MEMORY
            | PageTableFlags::EXECUTE_NEVER
            | PageTableFlags::ACCESS;
        let recursive_mapping_entry =
            recursive_mapping_entry_flags.bits() | page_table_address as u64;
        let mut page_table_handle =
            map_physical_memory(page_table_address, PAGE_SIZE, MemoryType::Normal);
        let final_entry: &mut [u8; 8] = (&mut page_table_handle[PAGE_SIZE - 8..])
            .try_into()
            .unwrap();
        *final_entry = recursive_mapping_entry.to_ne_bytes();
    }
    unsafe {
        asm::write_ttbr0(page_table_address as u64);
    }
}

pub fn is_valid_user_address(address: usize) -> bool {
    address < LOWER_RECURSIVE_MAPPING_ADDRESS as usize
}
