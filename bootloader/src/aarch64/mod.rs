mod paging;
mod registers;

pub use paging::PageAllocator;

pub fn check_environment() {
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
        let mut flags = paging::PageTableFlags::VALID | paging::PageTableFlags::NORMAL_MEMORY;
        if !writable {
            flags.insert(paging::PageTableFlags::READ_ONLY);
        }
        if !executable {
            flags.insert(paging::PageTableFlags::EXECUTE_NEVER);
        }
        self.0
            .map(allocator, virtual_address, physical_address, length, flags);
    }
}
