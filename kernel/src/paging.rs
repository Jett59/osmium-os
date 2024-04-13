pub use crate::arch_api::paging::{get_physical_address, map_page, unmap_page, PAGE_SIZE};
use crate::physical_memory_manager::PAGES_PER_BLOCK;

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
}

pub fn map_block(
    virtual_address: usize,
    physical_address: usize,
    memory_type: MemoryType,
    permissions: PagePermissions,
) {
    for i in 0..PAGES_PER_BLOCK {
        map_page(
            virtual_address + i * PAGE_SIZE,
            physical_address + i * PAGE_SIZE,
            memory_type,
            permissions,
        );
    }
}

pub fn unmap_block(virtual_address: usize) {
    for i in 0..PAGES_PER_BLOCK {
        unmap_page(virtual_address + i * PAGE_SIZE);
    }
}

pub fn change_page_permissions(
    virtual_address: usize,
    memory_type: MemoryType,
    permissions: PagePermissions,
) {
    let physical_address = get_physical_address(virtual_address);
    unmap_page(virtual_address);
    map_page(virtual_address, physical_address, memory_type, permissions);
}

pub fn change_block_permissions(
    virtual_address: usize,
    memory_type: MemoryType,
    permissions: PagePermissions,
) {
    for i in 0..PAGES_PER_BLOCK {
        change_page_permissions(virtual_address + i * PAGE_SIZE, memory_type, permissions);
    }
}
