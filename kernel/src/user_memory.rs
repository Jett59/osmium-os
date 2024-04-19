use core::marker::PhantomData;

use crate::{
    arch_api::paging::is_valid_user_address,
    paging::{map_block, MemoryType, PagePermissions},
    physical_memory_manager::{allocate_block_address, BLOCK_SIZE},
};

pub fn allocate_user_memory_at(virtual_address: usize, size: usize, permissions: PagePermissions) {
    assert_eq!(
        virtual_address % BLOCK_SIZE,
        0,
        "virtual_address must be BLOCK_SIZE aligned"
    );
    assert!(
        is_valid_user_address(virtual_address),
        "Invalid virtual address {}",
        virtual_address
    );
    assert!(
        is_valid_user_address(virtual_address + size),
        "Invalid virtual address {}",
        virtual_address + size
    );

    for virtual_block_address in (virtual_address..virtual_address + size).step_by(BLOCK_SIZE) {
        let physical_address = allocate_block_address().expect("Out of memory");
        map_block(
            virtual_block_address,
            physical_address,
            MemoryType::Normal,
            permissions,
        );
    }
}

/// A handle to the current address space, used to read and write user memory
///
/// This is intended to (A) allow for architectures which make it hard to read memory across the privilege boundary, and (B) avoid dangling pointers when switching address spaces.
/// (A) is easy to achieve, since there is a central point where all user mode accesses occur, so we can implement the necessary logic there.
/// (B) is covered by the fact that the handle is tied to the lifetime of the address space handle, which has to be consumed to switch address spaces.
/// This means that all handles will be invalidated when the address space is switched, and code will have to get a new handle to the new address space.
pub struct UserAddressSpaceHandle(PhantomData<()>);

impl UserAddressSpaceHandle {
    pub fn memory(&self, address: usize, length: usize) -> UserMemoryHandle {
        assert!(is_valid_user_address(address));
        assert!(is_valid_user_address(address + length));
        UserMemoryHandle {
            pointer: address as *mut u8,
            length,
            address_space: self,
        }
    }

    /// # Safety
    /// The caller must be absolutely certain that there are no other handles on this processor.
    /// Otherwise it would be possible to switch address spaces without invalidating the handles.
    ///
    /// In particular, do *not* use this to switch address spaces in a function without ownership of the address space.
    pub unsafe fn new() -> Self {
        Self(PhantomData)
    }
}

pub struct UserMemoryHandle<'address_space> {
    pointer: *mut u8,
    length: usize,
    address_space: &'address_space UserAddressSpaceHandle,
}

impl<'address_space> UserMemoryHandle<'address_space> {
    pub fn read(&self, buffer: &mut [u8]) {
        assert_eq!(buffer.len(), self.length);
        unsafe {
            core::ptr::copy_nonoverlapping(self.pointer, buffer.as_mut_ptr(), self.length);
        }
    }

    pub fn write(&self, buffer: &[u8]) {
        assert_eq!(buffer.len(), self.length);
        unsafe {
            core::ptr::copy_nonoverlapping(buffer.as_ptr(), self.pointer, self.length);
        }
    }

    pub fn read_part(&self, offset: usize, length: usize, buffer: &mut [u8]) {
        assert!(offset + length <= self.length);
        assert_eq!(buffer.len(), length);
        unsafe {
            core::ptr::copy_nonoverlapping(self.pointer.add(offset), buffer.as_mut_ptr(), length);
        }
    }

    pub fn write_part(&self, offset: usize, buffer: &[u8]) {
        assert!(offset + buffer.len() <= self.length);
        unsafe {
            core::ptr::copy_nonoverlapping(buffer.as_ptr(), self.pointer.add(offset), buffer.len());
        }
    }

    pub fn write_bytes(&self, offset: usize, value: u8, length: usize) {
        assert!(offset + length <= self.length);
        unsafe {
            core::ptr::write_bytes(self.pointer.add(offset), value, length);
        }
    }
}
