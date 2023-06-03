const PAGE_SHIFT: usize = 12;
const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
const ENTRY_COUNT: usize = 512;
const TABLE_MASK: usize = 0xFFFF_FFFF_FFFF_F000;

#[repr(C)]
pub struct PageTable {
    entries: [u64; ENTRY_COUNT],
}

impl PageTable {
    fn new() -> &'static mut Self {
        let frame = allocate_frame().expect("Failed to allocate frame for page table");
        let table: &'static mut Self = unsafe { &mut *(frame.start_address() as *mut Self) };
        for entry in &mut table.entries {
            *entry = 0;
        }
        table
    }
}

pub fn map_kernel(kernel_start: usize, kernel_end: usize) {
    let mut ttbr0_el1 = PageTable::new();

    let mut current_addr = kernel_start;
    while current_addr < kernel_end {
        let frame = PhysFrame::containing_address(current_addr);
        let index = (current_addr >> PAGE_SHIFT) & (ENTRY_COUNT - 1);
        let page_table = &mut ttbr0_el1.entries[index as usize];
        let phys_addr = frame.start_address();
        *page_table = (phys_addr as u64 & TABLE_MASK as u64) | 0b11; // Map page, read-write, EL0.
        current_addr += PAGE_SIZE;
    }

    // Switch to the new translation table.
    unsafe {
        asm!(
        "msr ttbr0_el1, {:x}",
        in(reg) ttbr0_el1 as *mut _ as u64,
        options(nomem, nostack)
        );
    }
}
