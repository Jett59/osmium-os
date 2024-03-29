mod paging;
mod registers;
mod transition;

pub use paging::PageAllocator;

pub use paging::page_align_down;
pub use paging::page_align_up;
pub use paging::PAGE_SIZE;

use crate::arch::registers::current_el;

pub fn check_environment() {
    uefi_services::println!("Running at {:?}", current_el());
    assert!(
        registers::current_el() == registers::ExceptionLevel::EL1
            || registers::current_el() == registers::ExceptionLevel::EL2,
        "Must be running at EL1 or EL2"
    );
}

pub struct PageTables(paging::PageTables);

impl PageTables {
    pub fn new<Allocator: PageAllocator>(allocator: &mut Allocator) -> Self {
        Self(paging::PageTables::new(allocator))
    }

    pub fn map<Allocator: PageAllocator>(
        &mut self,
        allocator: &mut Allocator,
        virtual_address: usize,
        physical_address: usize,
        length: usize,
        writable: bool,
        executable: bool,
    ) {
        let mut flags = paging::PageTableFlags::VALID
            | paging::PageTableFlags::NORMAL_MEMORY
            | paging::PageTableFlags::NOT_BLOCK
            | paging::PageTableFlags::ACCESS;
        if !writable {
            flags |= paging::PageTableFlags::READ_ONLY;
        }
        if !executable {
            flags |= paging::PageTableFlags::EXECUTE_NEVER;
        }
        self.0
            .map(allocator, virtual_address, physical_address, length, flags);
    }

    fn inner_mut(&mut self) -> &mut paging::PageTables {
        &mut self.0
    }
}

pub fn enter_kernel(
    entrypoint: usize,
    stack_base: usize,
    stack_size: usize,
    page_tables: &mut PageTables,
) -> ! {
    transition::enter_kernel(entrypoint, stack_base + stack_size, page_tables.inner_mut());
}
