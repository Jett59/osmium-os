use crate::arch::beryllium::BootRequestTagType;

// We include the stack pointer request tag here because I don't know where else it should go. TODO: maybe change this later?
#[repr(C, align(8))]
struct StackPointerTag {
    tag_type: BootRequestTagType, // = BootRequestTagType::StackPointer
    size: u16,                    // = 24 (64-bit) or 16 (32-bit)
    flags: u16,
    base: *mut u8,
    memory_size: usize,
}

const STACK_SIZE: usize = 8192;
static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
#[link_section = ".beryllium"]
static mut STACK_POINTER_TAG: StackPointerTag = StackPointerTag {
    tag_type: BootRequestTagType::StackPointer,
    size: core::mem::size_of::<StackPointerTag>() as u16,
    flags: 0,
    base: unsafe { STACK.as_mut_ptr() },
    memory_size: STACK_SIZE,
};

pub fn arch_init() {
    unimplemented!();
}
