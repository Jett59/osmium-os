use core::ops::{Index, IndexMut};

use bitflags::bitflags;

bitflags! {
    #[derive(Default, PartialEq, Eq, Copy, Clone)]
    pub struct PageTableFlags: u64 {
        const VALID = 1 << 0;
        const NOT_BLOCK = 1 << 1;

        const NORMAL_MEMORY = 1 << 2 | 3 << 8; // This must be programmed in the MAIR.

        const USER_ACCESSIBLE = 1 << 6;
        const READ_ONLY = 1 << 7;

        const ACCESS = 1 << 10;

        const PRIVILEGED_EXECUTE_NEVER = 1 << 53;
        const USER_EXECUTE_NEVER = 1 << 54;
    }
}

const PAGE_BITS: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_BITS;
const PAGE_TABLE_ENTRY_COUNT: usize = 512;

const ADDRESS_MASK: u64 = 0x0000_ffff_ffff_f000;

#[repr(C)]
pub struct PageTable {
    entries: [u64; PAGE_TABLE_ENTRY_COUNT],
}

pub trait PageAllocator {
    /// Allocates page_count contiguous pages.
    fn allocate(&mut self, page_count: usize) -> Option<*mut u8>;
}

impl<T> PageAllocator for T
where
    T: FnMut(usize) -> Option<*mut u8>,
{
    fn allocate(&mut self, page_count: usize) -> Option<*mut u8> {
        self(page_count)
    }
}

impl PageTable {
    fn zero(&mut self) {
        self.entries.fill(0);
    }

    pub fn new<Allocator: PageAllocator>(allocator: &mut Allocator) -> &'static mut Self {
        let page_table = unsafe { &mut *(allocator.allocate(1).unwrap() as *mut Self) };
        page_table.zero();
        page_table
    }

    pub fn is_valid(&self, index: usize) -> bool {
        self.entries[index] & PageTableFlags::VALID.bits() != 0
    }

    pub unsafe fn subtable(&mut self, index: usize) -> Option<&mut Self> {
        if !self.is_valid(index) {
            return None;
        }

        let address = self.entries[index] & ADDRESS_MASK;
        Some(&mut *(address as *mut Self))
    }

    pub fn create_subtable<Allocator: PageAllocator>(
        &mut self,
        allocator: &mut Allocator,
        index: usize,
    ) {
        if self.is_valid(index) {
            panic!("Subtable already exists");
        }

        let subtable = Self::new(allocator);
        let flags = PageTableFlags::VALID
            | PageTableFlags::NOT_BLOCK
            | PageTableFlags::NORMAL_MEMORY
            | PageTableFlags::ACCESS
            | PageTableFlags::USER_EXECUTE_NEVER;
        self.entries[index] = subtable as *mut _ as u64 | flags.bits();
    }
}

impl Index<usize> for PageTable {
    type Output = u64;

    fn index(&self, index: usize) -> &u64 {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut u64 {
        &mut self.entries[index]
    }
}

pub struct PageTables {
    low_half: &'static mut PageTable,
    high_half: &'static mut PageTable,
}

impl PageTables {
    pub fn new<Allocator: PageAllocator>(allocator: &mut Allocator) -> Self {
        let low_half = PageTable::new(allocator);
        let high_half = PageTable::new(allocator);

        Self {
            low_half,
            high_half,
        }
    }

    /// # Safety
    ///
    /// The addresses are required to be valid, and their not being so is not handled properly.
    unsafe fn map_page<Allocator: PageAllocator>(
        &mut self,
        allocator: &mut Allocator,
        virtual_address: usize,
        physical_address: usize,
        flags: PageTableFlags,
    ) {
        let level_0_index = (virtual_address >> 39) & 0o777;
        let level_1_index = (virtual_address >> 30) & 0o777;
        let level_2_index = (virtual_address >> 21) & 0o777;
        let level_3_index = (virtual_address >> 12) & 0o777;

        let level_0_page_table = if virtual_address >> 48 == 0 {
            &mut self.low_half
        } else {
            &mut self.high_half
        };
        if !level_0_page_table.is_valid(level_0_index) {
            level_0_page_table.create_subtable(allocator, level_0_index);
        }

        let level_1_page_table = level_0_page_table.subtable(level_0_index).unwrap();
        if !level_1_page_table.is_valid(level_1_index) {
            level_1_page_table.create_subtable(allocator, level_1_index);
        }

        let level_2_page_table = level_1_page_table.subtable(level_1_index).unwrap();
        if !level_2_page_table.is_valid(level_2_index) {
            level_2_page_table.create_subtable(allocator, level_2_index);
        }

        let level_3_page_table = level_2_page_table.subtable(level_2_index).unwrap();
        if level_3_page_table.is_valid(level_3_index) {
            panic!("Page already mapped");
        } else {
            level_3_page_table[level_3_index] = physical_address as u64 | flags.bits();
        }
    }

    pub fn map<Allocator: PageAllocator>(
        &mut self,
        allocator: &mut Allocator,
        virtual_address: usize,
        physical_address: usize,
        length: usize,
        flags: PageTableFlags,
    ) {
        if virtual_address % PAGE_SIZE != 0 {
            panic!("Virtual address must be page aligned");
        }
        if virtual_address >> 48 != 0 && virtual_address >> 48 != 0xffff {
            panic!("Virtual address must be canonical");
        }
        if physical_address % PAGE_SIZE != 0 {
            panic!("Physical address must be page aligned");
        }
        if physical_address > 0x0000_ffff_ffff_f000 {
            panic!("Physical address must be 48 bits or less");
        }
        if length % PAGE_SIZE != 0 {
            panic!("Length must be page aligned");
        }

        for offset in (0..length).step_by(PAGE_SIZE) {
            unsafe {
                self.map_page(
                    allocator,
                    virtual_address + offset,
                    physical_address + offset,
                    flags,
                );
            }
        }
    }

    pub fn upper(&self) -> &PageTable {
        self.high_half
    }
    pub fn upper_mut(&mut self) -> &mut PageTable {
        self.high_half
    }
}

pub fn page_align_down(address: usize) -> usize {
    address & !0xfff
}
pub fn page_align_up(address: usize) -> usize {
    page_align_down(address + PAGE_SIZE - 1)
}
