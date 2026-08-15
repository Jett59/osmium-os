use core::sync::atomic::{AtomicBool, Ordering};

use alloc::{boxed::Box, vec};

use crate::{
    assert::const_assert,
    buddy::BuddyAllocator,
    memory::{align_address_down, align_address_up},
    paging::{PagePermissions, create_mapping, take_mapping},
    physical_memory_manager::{self, BLOCK_SIZE, LOG2_BLOCK_SIZE},
    unsafe_impl::memory_token::{
        AllocatedMemoryToken, AllocatedRwToken, MemoryToken, MemoryTokenView, MemoryTokenViewMut,
        PhysicalMemoryToken, PhysicalRoToken, PhysicalRwToken, VirtualMemoryToken,
    },
};

#[cfg(target_arch = "x86_64")]
const VIRTUAL_HEAP_START: usize = 0xffffa00000000000;
#[cfg(target_arch = "x86_64")]
const HEAP_SIZE: usize = 0x1000000000; // 64GB

#[cfg(target_arch = "aarch64")]
const VIRTUAL_HEAP_START: usize = 0xffffa00000000000;
#[cfg(target_arch = "aarch64")]
const HEAP_SIZE: usize = 0x1000000000; // 64GB

const LOG2_HEAP_SIZE: u8 = HEAP_SIZE.trailing_zeros() as u8;

const_assert!(
    1 << LOG2_HEAP_SIZE == HEAP_SIZE,
    "HEAP_SIZE must be a power of two"
);

static HEAP_VIRTUAL_MEMORY_ALLOCATOR: spin::Mutex<
    BuddyAllocator<VirtualMemoryToken, 256, LOG2_HEAP_SIZE, LOG2_BLOCK_SIZE>,
> = spin::Mutex::new(BuddyAllocator::new());

static INITIALIZED_HEAP: AtomicBool = AtomicBool::new(false);

fn allocate_virtual_memory(size: usize) -> Option<VirtualMemoryToken> {
    let mut allocator = HEAP_VIRTUAL_MEMORY_ALLOCATOR.lock();
    // Race conditions are avoided by the lock.
    if !INITIALIZED_HEAP.swap(true, Ordering::SeqCst) {
        // SAFETY: the kernel is guaranteed to have exclusive access to the virtual memory range, so this is safe.
        let token = unsafe { VirtualMemoryToken::new(VIRTUAL_HEAP_START, HEAP_SIZE) };
        allocator.add_entry(token);
    }
    allocator.allocate(size)
}
fn free_virtual_memory(token: VirtualMemoryToken) {
    let mut allocator = HEAP_VIRTUAL_MEMORY_ALLOCATOR.lock();
    allocator.free(token);
}

pub fn alloc_large(size: usize) -> Option<AllocatedRwToken> {
    let virtual_token = allocate_virtual_memory(size)?;
    let mut allocation = AllocatedRwToken::empty(virtual_token.address());
    for virtual_block in virtual_token.chunks(BLOCK_SIZE) {
        let physical_block = physical_memory_manager::allocate_block()?;
        let allocated_block = create_mapping(
            PagePermissions::KERNEL_READ_WRITE,
            physical_block,
            virtual_block,
        );
        allocation = allocation.merge(allocated_block);
    }
    Some(allocation)
}

pub fn free_large(token: AllocatedRwToken) {
    let mut virtual_token = VirtualMemoryToken::empty(token.address());
    for allocated_block in token.chunks(BLOCK_SIZE) {
        let (physical_block, virtual_block) = take_mapping(allocated_block);
        physical_memory_manager::mark_as_free(physical_block);
        virtual_token = virtual_token.merge(virtual_block);
    }
    free_virtual_memory(virtual_token);
}

pub struct PhysicalMemoryHandle<P: PhysicalMemoryToken> {
    allocation: P::AllocatedToken,
    extra_virtual_memory: VirtualMemoryToken,
    offset: usize,
    size: usize,
}

impl<P: PhysicalMemoryToken> PhysicalMemoryHandle<P> {
    pub fn memory(&self) -> MemoryTokenView<'_, P::AllocatedToken> {
        self.allocation.view(self.offset, self.size)
    }

    pub fn memory_mut(&mut self) -> MemoryTokenViewMut<'_, P::AllocatedToken> {
        self.allocation.view_mut(self.offset, self.size)
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.memory().as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.memory_mut().as_mut_ptr()
    }

    pub fn into_mut_ptr(mut self) -> *mut u8 {
        let ptr = self.as_mut_ptr();
        core::mem::forget(self);
        ptr
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl PhysicalMemoryHandle<PhysicalRoToken> {
    pub fn as_slice(&self) -> &[u8] {
        &self.allocation.as_slice()[self.offset..self.offset + self.size]
    }

    pub fn into_slice(mut self) -> &'static [u8] {
        let ptr = self.as_mut_ptr();
        let size = self.size();
        core::mem::forget(self);
        unsafe { core::slice::from_raw_parts_mut(ptr, size) }
    }
}

impl PhysicalMemoryHandle<PhysicalRwToken> {
    pub fn as_slice(&self) -> &[u8] {
        &self.allocation.as_slice()[self.offset..self.offset + self.size]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.allocation.as_mut_slice()[self.offset..self.offset + self.size]
    }

    pub fn into_mut_slice(mut self) -> &'static mut [u8] {
        let ptr = self.as_mut_ptr();
        let size = self.size();
        core::mem::forget(self);
        unsafe { core::slice::from_raw_parts_mut(ptr, size) }
    }
}

impl<P: PhysicalMemoryToken> Drop for PhysicalMemoryHandle<P> {
    fn drop(&mut self) {
        let allocation = self.allocation.take();
        let (_physical_token, virtual_token) = take_mapping(allocation);
        let true_virtual_token = virtual_token.merge(self.extra_virtual_memory.take());
        free_virtual_memory(true_virtual_token);
    }
}

pub fn map_physical_memory<P: PhysicalMemoryToken>(
    memory: P,
    permissions: PagePermissions,
) -> PhysicalMemoryHandle<P> {
    let physical_block_address = align_address_down(memory.address(), BLOCK_SIZE);
    let physical_block_size = align_address_up(
        memory.size() + (memory.address() - physical_block_address),
        BLOCK_SIZE,
    );
    // SAFETY: this is not really safe :(
    // We are effectively taking ownership of a region of memory which we do not own.
    // TODO: implement a special API under paging to allow this to be done safely.
    let physical_token = unsafe { P::new(physical_block_address, physical_block_size) };
    let virtual_token = allocate_virtual_memory(physical_block_size)
        .expect("Failed to allocate virtual memory for physical memory mapping");
    let (necessary_virtual_token, extra_virtual_token) =
        virtual_token.split_at(physical_block_size);
    let allocation = create_mapping(permissions, physical_token, necessary_virtual_token);
    PhysicalMemoryHandle {
        allocation,
        extra_virtual_memory: extra_virtual_token,
        offset: memory.address() - physical_block_address,
        size: memory.size(),
    }
}

// This could be done with unit tests, if test drivers for paging etc. were implemented.
// TODO: do this

pub fn sanity_check() {
    // The one thing we can't let happen is the optimizer to optimize out these checks, which would be trivial to do.
    // That is why we use #[inline(never)] all over the place.
    #[inline(never)]
    fn allocate_it<const SIZE: usize>() -> Box<[u8; SIZE]> {
        vec![0u8; SIZE].into_boxed_slice().try_into().unwrap()
    }
    #[inline(never)]
    fn check_it<const SIZE: usize>(value: *mut u8) {
        let address = value as usize;
        assert!(address >= VIRTUAL_HEAP_START);
        assert!(address + SIZE <= VIRTUAL_HEAP_START + HEAP_SIZE);
        // We should touch all of the memory to make sure it is all accessible (and writeable).
        unsafe {
            for i in 0..SIZE {
                value.add(i).write_volatile(0);
            }
            for i in 0..SIZE {
                assert!(value.add(i).read_volatile() == 0);
            }
        }
    }
    // Allocate 1mb (for the large allocations case).
    check_it::<0x100000>(allocate_it::<0x100000>().as_mut_ptr());
    // Allocate 1kb (for the small allocations case).
    check_it::<0x400>(allocate_it::<0x400>().as_mut_ptr());
}
