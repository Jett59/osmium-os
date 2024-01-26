use core::{
    mem::size_of,
    ops::{Deref, DerefMut},
};

use crate::{
    arch_api::{asm::memory_barrier, paging::MemoryType},
    heap::{map_physical_memory, PhysicalAddressHandle},
};

#[derive(Copy, Clone)]
pub struct MmioPointer<T> {
    ptr: *mut T,
}

impl<T> MmioPointer<T> {
    pub fn new(ptr: *mut T) -> Self {
        Self { ptr }
    }

    /// # Safety
    /// Since this function is essentially a wrapper around a raw pointer, it is inherently unsafe.
    /// Additionally, MMIO accesses may have side effects, so this function is unsafe for that reason as well.
    pub unsafe fn read(&self) -> T {
        let result = unsafe { self.ptr.read_volatile() };
        memory_barrier();
        result
    }

    /// # Safety
    /// Since this function is essentially a wrapper around a raw pointer, it is inherently unsafe.
    /// Additionally, MMIO accesses may have side effects, so this function is unsafe for that reason as well.
    pub unsafe fn write(&mut self, value: T) {
        unsafe { self.ptr.write_volatile(value) };
        memory_barrier();
    }
}

pub struct MmioRange {
    start: *mut u8,
    size: usize,
}

impl MmioRange {
    pub fn new(start: *mut u8, size: usize) -> Self {
        Self { start, size }
    }

    /// # Safety
    /// The same rules apply as for performing arithmetic on a raw pointer.
    pub unsafe fn at_offset<T>(&self, offset: usize) -> MmioPointer<T> {
        assert!(offset + size_of::<T>() <= self.size);
        MmioPointer::new(unsafe { self.start.add(offset) as *mut T })
    }

    pub fn resize(&mut self, new_size: usize) {
        self.size = new_size;
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

pub struct MmioMemoryHandle {
    physical_memory_handle: PhysicalAddressHandle,
    mmio_range: MmioRange,
}

impl MmioMemoryHandle {
    /// # Safety
    /// see `heap::map_physical_memory`.
    pub unsafe fn new(physical_address: usize, size: usize) -> Self {
        let mut handle = map_physical_memory(physical_address, size, MemoryType::Device);
        let mmio_range = MmioRange::new(
            PhysicalAddressHandle::as_mut_ptr(&mut handle),
            PhysicalAddressHandle::size(&handle),
        );
        Self {
            physical_memory_handle: handle,
            mmio_range,
        }
    }
}

impl Deref for MmioMemoryHandle {
    type Target = MmioRange;

    fn deref(&self) -> &Self::Target {
        &self.mmio_range
    }
}

impl DerefMut for MmioMemoryHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mmio_range
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mmio_pointer_test() {
        let mut memory = [1u8, 2u8, 3u8, 4u8];
        let mut mmio_pointer = MmioPointer::<u32>::new(memory.as_mut_ptr() as *mut u32);
        // SAFETY: It is safe to dereference this pointer, since we are allowed to construct a u32 from arbitrary bytes (assuming the pointer is aligned, which local variables always are).
        #[cfg(target_endian = "little")]
        assert_eq!(unsafe { mmio_pointer.read() }, 0x04030201);
        #[cfg(target_endian = "big")]
        assert_eq!(unsafe { mmio_pointer.read() }, 0x01020304);

        unsafe { mmio_pointer.write(0xdeadbeef) };
        #[cfg(target_endian = "little")]
        assert_eq!(memory, [0xef, 0xbe, 0xad, 0xde]);
        #[cfg(target_endian = "big")]
        assert_eq!(memory, [0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn mmio_range_test() {
        let mut memory = [1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8];

        let mmio_range = MmioRange::new(memory.as_mut_ptr(), memory.len());

        // SAFETY: The offset is within the bounds of the memory range.
        let mut mmio_pointer = unsafe { mmio_range.at_offset::<u32>(0) };

        #[cfg(target_endian = "little")]
        assert_eq!(unsafe { mmio_pointer.read() }, 0x04030201);
        #[cfg(target_endian = "big")]
        assert_eq!(unsafe { mmio_pointer.read() }, 0x01020304);

        unsafe { mmio_pointer.write(0xdeadbeef) };
        #[cfg(target_endian = "little")]
        assert_eq!(memory[..4], [0xef, 0xbe, 0xad, 0xde]);
        #[cfg(target_endian = "big")]
        assert_eq!(memory[..4], [0xde, 0xad, 0xbe, 0xef]);

        let mut mmio_pointer = unsafe { mmio_range.at_offset::<u32>(4) };

        #[cfg(target_endian = "little")]
        assert_eq!(unsafe { mmio_pointer.read() }, 0x08070605);
        #[cfg(target_endian = "big")]
        assert_eq!(unsafe { mmio_pointer.read() }, 0x05060708);

        unsafe { mmio_pointer.write(0x0badc0de) };
        #[cfg(target_endian = "little")]
        assert_eq!(memory[4..], [0xde, 0xc0, 0xad, 0x0b]);
        #[cfg(target_endian = "big")]
        assert_eq!(memory[4..], [0x0b, 0xad, 0xc0, 0xde]);
    }
}
