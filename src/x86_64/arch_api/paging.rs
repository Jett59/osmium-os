pub const PAGE_SIZE: usize = 4096;

const RECURSIVE_PAGE_TABLE_INDEX: usize = 257; // We are in the last 2g so we can't use 511.

// 257 is the first entry in the higher half of the virtual address space.
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

pub fn initialize_paging_structures() {}
