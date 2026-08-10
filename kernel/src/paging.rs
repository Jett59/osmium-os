use crate::unsafe_impl::memory_token::{AllocatedMemoryToken, MemoryToken, PhysicalMemoryToken, VirtualMemoryToken};
pub use crate::unsafe_impl::paging::{
    PAGE_SIZE, create_page_mapping, is_valid_user_address, take_page_mapping,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MemoryType {
    Normal,
    Device,
}

#[derive(Clone, Copy, Debug)]
pub struct PagePermissions {
    pub user: bool,
    pub writable: bool,
    pub executable: bool,
}

impl PagePermissions {
    pub fn new(user: bool, writable: bool, executable: bool) -> Self {
        Self {
            user,
            writable,
            executable,
        }
    }

pub fn intersect(self, other: Self) -> Self {
        Self {
            user: self.user && other.user,
            writable: self.writable && other.writable,
            executable: self.executable && other.executable,
        }
    }

    pub const KERNEL_READ_ONLY: Self = Self {
        user: false,
        writable: false,
        executable: false,
    };
    pub const KERNEL_READ_WRITE: Self = Self {
        user: false,
        writable: true,
        executable: false,
    };
    pub const KERNEL_READ_EXECUTE: Self = Self {
        user: false,
        writable: false,
        executable: true,
    };
    pub const KERNEL_READ_WRITE_EXECUTE: Self = Self {
        user: false,
        writable: true,
        executable: true,
    };
    pub const USER_READ_ONLY: Self = Self {
        user: true,
        writable: false,
        executable: false,
    };
    pub const USER_READ_WRITE: Self = Self {
        user: true,
        writable: true,
        executable: false,
    };
    pub const USER_READ_EXECUTE: Self = Self {
        user: true,
        writable: false,
        executable: true,
    };
    pub const USER_READ_WRITE_EXECUTE: Self = Self {
        user: true,
        writable: true,
        executable: true,
    };
}

pub fn create_mapping<P: PhysicalMemoryToken>(
    permissions: PagePermissions,
    physical_address: P,
    virtual_address: VirtualMemoryToken,
) -> P::AllocatedToken {
    assert!(
        virtual_address.address().is_multiple_of(PAGE_SIZE),
        "Virtual address must be page-aligned"
    );
    assert!(
        physical_address.address().is_multiple_of(PAGE_SIZE),
        "Physical address must be page-aligned"
    );
    assert_eq!(
        virtual_address.size(),
        physical_address.size(),
        "Virtual and physical addresses must have the same size"
    );
    if virtual_address.size() == 0 {
        return P::AllocatedToken::empty(virtual_address.address());
    }
    virtual_address
        .chunks(PAGE_SIZE)
        .zip(physical_address.chunks(PAGE_SIZE))
        .map(|(virtual_page, physical_page)| {
            create_page_mapping(permissions, physical_page, virtual_page)
        })
        .reduce(MemoryToken::merge)
        .unwrap()
}

/// # Panics
/// Panics if the allocated address is not page-aligned or has a size of 0
/// Also panics if the address is not allocated to a contiguous range of physical memory.
pub fn take_mapping<A: AllocatedMemoryToken>(
    allocated_address: A,
) -> (A::PhysicalToken, VirtualMemoryToken) {
    assert!(
        allocated_address.address().is_multiple_of(PAGE_SIZE),
        "Allocated address must be page-aligned"
    );
    assert_ne!(
        allocated_address.size(),
        0,
        "Allocated address must have a non-zero size"
    );
    allocated_address
        .chunks(PAGE_SIZE)
        .map(take_page_mapping)
        .reduce(|(phys_acc, virt_acc), (phys_page, virt_page)| {
            (phys_acc.merge(phys_page), virt_acc.merge(virt_page))
        })
        .unwrap()
}

pub fn change_page_permissions<A: AllocatedMemoryToken>(
    allocated_address: A,
    permissions: PagePermissions,
) -> A {
    let (physical_address, virtual_address) = take_page_mapping(allocated_address);
    create_page_mapping(permissions, physical_address, virtual_address)
}

pub fn change_permissions<A: AllocatedMemoryToken>(
    allocated_memory: A,
    permissions: PagePermissions,
) -> A {
    let (physical_address, virtual_address) = take_mapping(allocated_memory);
    create_mapping(permissions, physical_address, virtual_address)
}
