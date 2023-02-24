use crate::arch::beryllium::BootRequestTagType;
use crate::arch_api::stack::Stack;
use core::mem::size_of;

// We include the stack pointer request tag here because I don't know where else it should go. TODO: maybe change this later?
static mut STACK: Stack = Default::default();
#[link_section = ".beryllium"]
#[no_mangle]
pub static mut STACK_POINTER_TAG: StackPointerTag = StackPointerTag {
    tag_type: BootRequestTagType::StackPointer,
    size: size_of::<StackPointerTag>() as u16,
    flags: 0,
    base: unsafe { STACK.as_mut_ptr() },
    memory_size: size_of::<Stack>(),
};

pub fn arch_init() {
    unimplemented!();
}
