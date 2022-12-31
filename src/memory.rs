pub trait Validateable {
    // Ensure that an instance of this type is valid. This is used to ensure that
    // objects which are created by reinterpreting some region of memory are in fact instances of the correct type.
    fn validate(&self) -> bool;
}

pub unsafe fn reinterpret_memory<T: Validateable>(memory: &[u8]) -> Option<&T> {
    if memory.len() < core::mem::size_of::<T>() {
        return None;
    }
    let ptr = memory.as_ptr() as *const T;
    let reference = unsafe { &*ptr };
    if reference.validate() {
        Some(reference)
    } else {
        None
    }
}

pub unsafe fn pointer_from_address(address: usize) -> *const u8 {
    address as *const u8
}
