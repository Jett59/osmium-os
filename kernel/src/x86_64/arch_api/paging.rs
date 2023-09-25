use core::arch::asm;

use crate::{buddy::BuddyAllocator, lazy_init::lazy_static, physical_memory_manager};

pub const PAGE_SIZE: usize = 4096;

const RECURSIVE_PAGE_TABLE_INDEX: usize = 256; // We are in the last 2g so we can't use 511.

// 256 is the first entry in the higher half of the virtual address space.
const RECURSIVE_PAGE_TABLE_POINTER: *mut u64 = 0xffff_8000_0000_0000 as *mut u64;

// Not the actual layout, but has all of the right fields.
struct PageTableEntry {
    present: bool,
    writeable: bool,
    user_accessible: bool,
    write_through: bool,
    cache_disabled: bool,
    accessed: bool,
    dirty: bool,
    huge_page: bool,
    global: bool,
    physical_address: u64,
}

fn construct_page_table_entry(data: PageTableEntry) -> u64 {
    let mut result = 0;
    if data.present {
        result |= 1;
    }
    if data.writeable {
        result |= 1 << 1;
    }
    if data.user_accessible {
        result |= 1 << 2;
    }
    if data.write_through {
        result |= 1 << 3;
    }
    if data.cache_disabled {
        result |= 1 << 4;
    }
    if data.accessed {
        result |= 1 << 5;
    }
    if data.dirty {
        result |= 1 << 6;
    }
    if data.huge_page {
        result |= 1 << 7;
    }
    if data.global {
        result |= 1 << 8;
    }
    result |= data.physical_address & 0x000f_ffff_ffff_f000;
    result
}

unsafe fn write_page_table_entry(
    entry: PageTableEntry,
    pml4_index: usize,
    pml3_index: usize,
    pml2_index: usize,
    pml1_index: usize,
) {
    let offset =
        pml1_index + pml2_index * 512 + pml3_index * 512 * 512 + pml4_index * 512 * 512 * 512;
    let entry = construct_page_table_entry(entry);
    *RECURSIVE_PAGE_TABLE_POINTER.add(offset) = entry;
}

fn deconstruct_page_table_entry(entry: u64) -> PageTableEntry {
    PageTableEntry {
        present: entry & 1 != 0,
        writeable: entry & (1 << 1) != 0,
        user_accessible: entry & (1 << 2) != 0,
        write_through: entry & (1 << 3) != 0,
        cache_disabled: entry & (1 << 4) != 0,
        accessed: entry & (1 << 5) != 0,
        dirty: entry & (1 << 6) != 0,
        huge_page: entry & (1 << 7) != 0,
        global: entry & (1 << 8) != 0,
        physical_address: entry & 0x000f_ffff_ffff_f000,
    }
}

unsafe fn read_page_table_entry(
    pml4_index: usize,
    pml3_index: usize,
    pml2_index: usize,
    pml1_index: usize,
) -> PageTableEntry {
    let offset =
        pml1_index + pml2_index * 512 + pml3_index * 512 * 512 + pml4_index * 512 * 512 * 512;
    let entry = *RECURSIVE_PAGE_TABLE_POINTER.add(offset);
    deconstruct_page_table_entry(entry)
}

unsafe fn get_page_table_entry_address(
    pml4_index: usize,
    pml3_index: usize,
    pml2_index: usize,
    pml1_index: usize,
) -> *mut u64 {
    let offset =
        pml1_index + pml2_index * 512 + pml3_index * 512 * 512 + pml4_index * 512 * 512 * 512;
    RECURSIVE_PAGE_TABLE_POINTER.add(offset)
}

struct PageTableIndices {
    pml4_index: usize,
    pml3_index: usize,
    pml2_index: usize,
    pml1_index: usize,
}

fn deconstruct_virtual_address(address: usize) -> PageTableIndices {
    PageTableIndices {
        pml4_index: (address >> 39) & 0x1ff,
        pml3_index: (address >> 30) & 0x1ff,
        pml2_index: (address >> 21) & 0x1ff,
        pml1_index: (address >> 12) & 0x1ff,
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
        if let Some(free_block) = PAGE_TABLE_ALLOCATION_POOL.allocate(physical_memory_manager::BLOCK_SIZE) {
            physical_memory_manager::mark_as_free(free_block);
        }
    }
}

fn ensure_page_table_exists(pml4_index: usize, pml3_index: usize, pml2_index: usize) {
    // Indexing with the first indices set to RECURSIVE_PAGE_TABLE_INDEX will give us the next layer up in the page tables.
    let pml4_entry = unsafe {
        read_page_table_entry(
            RECURSIVE_PAGE_TABLE_INDEX,
            RECURSIVE_PAGE_TABLE_INDEX,
            RECURSIVE_PAGE_TABLE_INDEX,
            pml4_index,
        )
    };
    if !pml4_entry.present {
        unsafe {
            write_page_table_entry(
                PageTableEntry {
                    present: true,
                    writeable: true,
                    user_accessible: pml4_index < 256,
                    write_through: false,
                    cache_disabled: false,
                    accessed: false,
                    dirty: false,
                    huge_page: false,
                    global: false,
                    physical_address: allocate_page_table() as u64,
                },
                RECURSIVE_PAGE_TABLE_INDEX,
                RECURSIVE_PAGE_TABLE_INDEX,
                RECURSIVE_PAGE_TABLE_INDEX,
                pml4_index,
            );
            let address = get_page_table_entry_address(
                RECURSIVE_PAGE_TABLE_INDEX,
                RECURSIVE_PAGE_TABLE_INDEX,
                pml4_index,
                0,
            );
            core::ptr::write_bytes(address as *mut u8, 0, 4096);
        }
    }
    let pml3_entry = unsafe {
        read_page_table_entry(
            RECURSIVE_PAGE_TABLE_INDEX,
            RECURSIVE_PAGE_TABLE_INDEX,
            pml4_index,
            pml3_index,
        )
    };
    if !pml3_entry.present {
        unsafe {
            write_page_table_entry(
                PageTableEntry {
                    present: true,
                    writeable: true,
                    user_accessible: pml4_index < 256,
                    write_through: false,
                    cache_disabled: false,
                    accessed: false,
                    dirty: false,
                    huge_page: false,
                    global: false,
                    physical_address: allocate_page_table() as u64,
                },
                RECURSIVE_PAGE_TABLE_INDEX,
                RECURSIVE_PAGE_TABLE_INDEX,
                pml4_index,
                pml3_index,
            );
            let address =
                get_page_table_entry_address(RECURSIVE_PAGE_TABLE_INDEX, pml4_index, pml3_index, 0);
            core::ptr::write_bytes(address as *mut u8, 0, 4096);
        }
    }
    let pml2_entry = unsafe {
        read_page_table_entry(
            RECURSIVE_PAGE_TABLE_INDEX,
            pml4_index,
            pml3_index,
            pml2_index,
        )
    };
    if !pml2_entry.present {
        unsafe {
            write_page_table_entry(
                PageTableEntry {
                    present: true,
                    writeable: true,
                    user_accessible: pml4_index < 256,
                    write_through: false,
                    cache_disabled: false,
                    accessed: false,
                    dirty: false,
                    huge_page: false,
                    global: false,
                    physical_address: allocate_page_table() as u64,
                },
                RECURSIVE_PAGE_TABLE_INDEX,
                pml4_index,
                pml3_index,
                pml2_index,
            );
            let address = get_page_table_entry_address(pml4_index, pml3_index, pml2_index, 0);
            core::ptr::write_bytes(address as *mut u8, 0, 4096);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MemoryType {
    Normal,
    Device,
}

pub fn map_page(virtual_address: usize, physical_address: usize, memory_type: MemoryType) {
    unsafe {
        let indices = deconstruct_virtual_address(virtual_address);
        ensure_page_table_exists(indices.pml4_index, indices.pml3_index, indices.pml2_index);
        let entry = read_page_table_entry(
            indices.pml4_index,
            indices.pml3_index,
            indices.pml2_index,
            indices.pml1_index,
        );
        if entry.present {
            panic!("Remapping a page which is already mapped!");
        }
        write_page_table_entry(
            PageTableEntry {
                present: true,
                writeable: true,
                user_accessible: virtual_address & (1 << 47) == 0,
                write_through: false,
                cache_disabled: memory_type == MemoryType::Device,
                accessed: false,
                dirty: false,
                huge_page: false,
                global: virtual_address & (1 << 47) != 0,
                physical_address: physical_address as u64,
            },
            indices.pml4_index,
            indices.pml3_index,
            indices.pml2_index,
            indices.pml1_index,
        );
    }
}

fn is_page_table_present(indices: &PageTableIndices) -> bool {
    // Check if the pml4 entry is there, then the pml3, then the pml2.
    let pml4_entry = unsafe {
        read_page_table_entry(
            RECURSIVE_PAGE_TABLE_INDEX,
            RECURSIVE_PAGE_TABLE_INDEX,
            RECURSIVE_PAGE_TABLE_INDEX,
            indices.pml4_index,
        )
    };
    if !pml4_entry.present {
        return false;
    }
    let pml3_entry = unsafe {
        read_page_table_entry(
            RECURSIVE_PAGE_TABLE_INDEX,
            RECURSIVE_PAGE_TABLE_INDEX,
            indices.pml4_index,
            indices.pml3_index,
        )
    };
    if !pml3_entry.present {
        return false;
    }
    let pml2_entry = unsafe {
        read_page_table_entry(
            RECURSIVE_PAGE_TABLE_INDEX,
            indices.pml4_index,
            indices.pml3_index,
            indices.pml2_index,
        )
    };
    if !pml2_entry.present {
        return false;
    }
    true
}

unsafe fn clear_page_cache(address: usize) {
    asm!("invlpg [{}]", in(reg) address, options(nostack));
}

pub fn unmap_page(virtual_address: usize) {
    unsafe {
        let indices = deconstruct_virtual_address(virtual_address);
        if !is_page_table_present(&indices) {
            panic!("Unmapping a page that isn't mapped!");
        }
        let entry = read_page_table_entry(
            indices.pml4_index,
            indices.pml3_index,
            indices.pml2_index,
            indices.pml1_index,
        );
        if !entry.present {
            panic!("Unmapping a page that isn't mapped!");
        }
        write_page_table_entry(
            PageTableEntry {
                present: false,
                writeable: false,
                user_accessible: false,
                write_through: false,
                cache_disabled: false,
                accessed: false,
                dirty: false,
                huge_page: false,
                global: false,
                physical_address: 0,
            },
            indices.pml4_index,
            indices.pml3_index,
            indices.pml2_index,
            indices.pml1_index,
        );
        clear_page_cache(virtual_address);
    }
}

pub fn get_physical_address(virtual_address: usize) -> usize {
    unsafe {
        let indices = deconstruct_virtual_address(virtual_address);
        if !is_page_table_present(&indices) {
            panic!("Attempt to get value of unmapped page!");
        }
        let entry = read_page_table_entry(
            indices.pml4_index,
            indices.pml3_index,
            indices.pml2_index,
            indices.pml1_index,
        );
        if !entry.present {
            panic!("Attempt to get value of unmapped page!");
        }
        entry.physical_address as usize + virtual_address % PAGE_SIZE
    }
}

pub(super) fn initialize_paging() {
    // We must remove some of the mappings the startup code used (there is one which maps the first gigabyte exactly like the last, and one which maps the first 512g likewise).
    // First remove the mapping of the low 512g:
    unsafe {
        unmap_page(get_page_table_entry_address(
            RECURSIVE_PAGE_TABLE_INDEX,
            RECURSIVE_PAGE_TABLE_INDEX,
            0,
            0,
        ) as usize);
    }
    // Then for the 511th pml4, 0th pml3:
    unsafe {
        unmap_page(get_page_table_entry_address(RECURSIVE_PAGE_TABLE_INDEX, 511, 0, 0) as usize);
    }
    // We'll do a quick sanity check: Mapping the first 4k of physical memory to some address and then compare that with the first 4k of the last 2g (where the kernel lives).
    map_page(4096, 0, MemoryType::Normal);
    let slice_in_low_memory = unsafe { core::slice::from_raw_parts(4096 as *const u8, 4096) };
    let slice_in_high_memory =
        unsafe { core::slice::from_raw_parts(0xffffffff80000000 as *const u8, 4096) };
    assert_eq!(slice_in_low_memory, slice_in_high_memory);
    unmap_page(4096);
}
