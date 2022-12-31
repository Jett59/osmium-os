use core::slice;

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

pub unsafe fn slice_from_memory<'lifetime>(
    pointer: *const u8,
    length: usize,
) -> Option<&'lifetime [u8]> {
    if pointer.is_null() {
        return None;
    }
    Some(slice::from_raw_parts(pointer, length))
}
