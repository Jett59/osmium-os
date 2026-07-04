use core::arch::asm;

use bitflags::bitflags;

use crate::{
    paging::{MemoryType, PagePermissions},
    unsafe_impl::{
        arch::asm,
        memory::{AllocatedMemoryToken, MemoryToken, PhysicalMemoryToken, VirtualMemoryToken},
    },
};

/// The recursive page index for the upper half (kernel) is set to 0 by the bootloader.
const UPPER_RECURSIVE_MAPPING_INDEX: usize = 0;
const UPPER_RECURSIVE_MAPPING_ADDRESS: *mut u64 = 0xffff_0000_0000_0000 as *mut u64;

/// For the lower half, we set it to the top of the lower half address space (index 511).
const LOWER_RECURSIVE_MAPPING_INDEX: usize = 511;
const LOWER_RECURSIVE_MAPPING_ADDRESS: *mut u64 = 0x0000_ff80_0000_0000 as *mut u64;

const PHYSICAL_PAGE_MASK: u64 = 0x0000_ffff_ffff_f000;

bitflags! {
    struct PageTableFlags: u64 {
        const VALID = 1 << 0;
        const NOT_BLOCK = 1 << 1;

        // Programmed in the MAIR by the bootloader. MAIR[0]=Device-NGNRNE, MAIR[1]=Normal write-back
        const NORMAL_MEMORY = 1 << 2 | 3 << 8;
        const DEVICE_MEMORY = 0 << 2;

        const USER_ACCESSIBLE = 1 << 6;
        const READ_ONLY = 1 << 7;

        const ACCESS = 1 << 10;

        const PRIVILEGED_EXECUTE_NEVER = 1 << 53;
        const USER_EXECUTE_NEVER = 1 << 54;
    }
}

impl PageTableFlags {
    fn from_type_permissions(memory_type: MemoryType, permissions: PagePermissions) -> Self {
        let mut flags = PageTableFlags::VALID
            | PageTableFlags::NOT_BLOCK
            | PageTableFlags::ACCESS
            | match memory_type {
                MemoryType::Normal => PageTableFlags::NORMAL_MEMORY,
                MemoryType::Device => PageTableFlags::DEVICE_MEMORY,
            };

        if permissions.user {
            flags |= PageTableFlags::USER_ACCESSIBLE;
            flags |= PageTableFlags::PRIVILEGED_EXECUTE_NEVER;
        } else {
            flags |= PageTableFlags::USER_EXECUTE_NEVER;
        }
        if !permissions.writable {
            flags |= PageTableFlags::READ_ONLY;
        }
        if !permissions.executable {
            flags |= PageTableFlags::PRIVILEGED_EXECUTE_NEVER;
            flags |= PageTableFlags::USER_EXECUTE_NEVER;
        }

        flags
    }
}

pub struct PageTableIndices {
    upper_half: bool,
    level_0_index: usize,
    level_1_index: usize,
    level_2_index: usize,
    level_3_index: usize,
}

impl PageTableIndices {
    pub fn new(address: usize) -> Self {
        Self {
            upper_half: address >= 0xffff_0000_0000_0000,
            level_0_index: (address >> 39) & 0x1ff,
            level_1_index: (address >> 30) & 0x1ff,
            level_2_index: (address >> 21) & 0x1ff,
            level_3_index: (address >> 12) & 0x1ff,
        }
    }

    pub fn address(&self) -> usize {
        let mut address = 0;
        if self.upper_half {
            address |= 0xffff_0000_0000_0000;
        }
        address |= self.level_0_index << 39;
        address |= self.level_1_index << 30;
        address |= self.level_2_index << 21;
        address |= self.level_3_index << 12;
        address
    }

    fn calculate_page_table_entry_address(&self) -> *mut u64 {
        let offset = self.level_3_index
            + self.level_2_index * 512
            + self.level_1_index * 512 * 512
            + self.level_0_index * 512 * 512 * 512;
        if self.upper_half {
            // SAFETY: the `new` function ensures that the indices are in the valid range of 0..512, so the offset will always be within the bounds of the recursive mapping.
            unsafe { UPPER_RECURSIVE_MAPPING_ADDRESS.add(offset) }
        } else {
            // SAFETY: same as above
            unsafe { LOWER_RECURSIVE_MAPPING_ADDRESS.add(offset) }
        }
    }
}

fn recursive_mapping_index(upper_half: bool) -> usize {
    if upper_half {
        UPPER_RECURSIVE_MAPPING_INDEX
    } else {
        LOWER_RECURSIVE_MAPPING_INDEX
    }
}

pub fn write_page_table_mapping(
    memory_type: MemoryType,
    permissions: PagePermissions,
    physical_address: PhysicalMemoryToken,
    virtual_address: VirtualMemoryToken,
) -> AllocatedMemoryToken {
    assert!(
        virtual_address.address().is_multiple_of(4096),
        "Virtual address must be page-aligned"
    );
    assert_eq!(
        virtual_address.size(),
        4096,
        "Virtual address must be a single page"
    );
    assert!(
        physical_address.address().is_multiple_of(4096),
        "Physical address must be page-aligned"
    );
    assert_eq!(
        physical_address.size(),
        4096,
        "Physical address must be a single page"
    );

    let indices = PageTableIndices::new(virtual_address.address());
    let flags = PageTableFlags::from_type_permissions(memory_type, permissions);
    let physical_address = physical_address.address() as u64;
    let entry = flags.bits() | physical_address & PHYSICAL_PAGE_MASK;
    let entry_address = indices.calculate_page_table_entry_address();
    // SAFETY: the pointer is guaranteed to point to a page table entry.
    // The physical and virtual addresses are guaranteed to be valid pages owned by the caller.
    // You must trust me that I got the right format of a page table entry.
    unsafe { *entry_address = entry };
    asm::dsb_ish();
    asm::isb();
    // SAFETY: the mapping is now present, so the caller can safely use the address.
    unsafe { AllocatedMemoryToken::new(virtual_address.address(), 4096) }
}

#[must_use]
pub fn take_page_table_mapping(
    allocated_address: AllocatedMemoryToken,
) -> (PhysicalMemoryToken, VirtualMemoryToken) {
    assert!(
        allocated_address.address().is_multiple_of(4096),
        "Allocated address must be page-aligned"
    );
    assert_eq!(
        allocated_address.size(),
        4096,
        "Allocated address must be a single page"
    );

    let indices = PageTableIndices::new(allocated_address.address());
    let entry_address = indices.calculate_page_table_entry_address();
    // SAFETY: the token proves that the page is mapped, so the page table entry must exist.
    let entry = unsafe { *entry_address };
    // SAFETY: since we now own the allocation, we are allowed to de-allocate it
    unsafe { *entry_address = 0 };
    // SAFETY: invalidating the TLB is never unsafe.
    unsafe {
        asm!("tlbi vaae1is, {}", in (reg) allocated_address.address() / 4096, options(nomem, nostack))
    };
    asm::dsb_ish();
    asm::isb();

    let flags = PageTableFlags::from_bits_truncate(entry);
    assert!(
        flags.contains(PageTableFlags::VALID),
        "Taking a page which is not mapped!"
    );
    let physical_address = entry & PHYSICAL_PAGE_MASK;
    // SAFETY: the physical and virtual addresses are guaranteed to be owned by the caller.
    unsafe {
        (
            PhysicalMemoryToken::new(physical_address as usize, 4096),
            VirtualMemoryToken::new(allocated_address.address(), 4096),
        )
    }
}
