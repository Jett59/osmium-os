use core::{mem::MaybeUninit, slice};

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
    let reference = &*ptr;
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

// For types which have a field which represents the size of the structure. This is often useful for lists of structures (in some sort of table) which may have any of a number of different types of field. In these cases, there is some mechanism for determining the size of the entry, either through a 'length' field or a type field, where the type implies a size.
pub trait DynamicallySized {
    fn size(&self) -> usize;

    // This is the bound to which offsets will be aligned in the buffer when the offset is incremented. It is useful if the size field doesn't account for manditory alignment of entries (as is sometimes the case).
    const ALIGNMENT: usize = 1;
}

pub struct DynamicallySizedItem<'lifetime, T: DynamicallySized> {
    pub value: &'lifetime T,
    pub value_memory: &'lifetime [u8], // Sized to the dynamic size of T
}

pub struct DynamicallySizedObjectIterator<'lifetime, T: DynamicallySized> {
    total_memory: &'lifetime [u8],
    current_offset: usize,
    _phantom: core::marker::PhantomData<T>, // To make the compiler happy about having T as a type parameter.
}

impl<'lifetime, T: DynamicallySized> DynamicallySizedObjectIterator<'lifetime, T>
where
    T: 'lifetime + Validateable,
{
    pub fn new(memory: &'lifetime [u8]) -> Self {
        Self {
            total_memory: memory,
            current_offset: 0,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<'lifetime, T: DynamicallySized> Iterator for DynamicallySizedObjectIterator<'lifetime, T>
where
    T: 'lifetime + Validateable,
{
    type Item = DynamicallySizedItem<'lifetime, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_offset >= self.total_memory.len() {
            return None;
        }
        let value_memory = &self.total_memory[self.current_offset..];
        let optional_value: Option<&'lifetime T> = unsafe { reinterpret_memory(value_memory) };
        let value = match optional_value {
            Some(value) => value,
            None => return None,
        };
        if value.size() > self.total_memory.len() - self.current_offset {
            return None;
        }
        let value_memory = &value_memory[..value.size()];
        let item = DynamicallySizedItem {
            value,
            value_memory,
        };
        self.current_offset += item.value.size();
        // Align current_offset up to the correct boundary.
        let alignment = T::ALIGNMENT;
        self.current_offset = (self.current_offset + alignment - 1) & !(alignment - 1);
        Some(item)
    }
}

pub fn align_address_down(address: usize, alignment: usize) -> usize {
    address & !(alignment - 1)
}
pub fn align_address_up(address: usize, alignment: usize) -> usize {
    (address + alignment - 1) & !(alignment - 1)
}

pub const fn constant_initialized_array<
    T,
    const SIZE: usize,
    InitializationFunction: ~const Fn() -> T,
>(
    initialization_function: &InitializationFunction,
) -> [T; SIZE] {
    unsafe {
        let mut result: [MaybeUninit<T>; SIZE] = MaybeUninit::uninit_array();
        // We can't use a for loop in a const function yet.
        let mut i = 0;
        while i < SIZE {
            result[i].write(initialization_function());
            i += 1;
        }
        MaybeUninit::array_assume_init(result)
    }
}

#[cfg(test)]
mod test {
    use core::sync::atomic::{AtomicU8, Ordering};

    use super::*;

    #[test]
    fn test_dynamic_sized_object_iterator() {
        #[repr(C, packed)]
        struct TestStruct {
            a: u8,
            b: u8,
            c: u8,
        }

        impl Validateable for TestStruct {
            fn validate(&self) -> bool {
                true
            }
        }

        impl DynamicallySized for TestStruct {
            fn size(&self) -> usize {
                3
            }
        }

        let memory = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let iterator: DynamicallySizedObjectIterator<TestStruct> =
            DynamicallySizedObjectIterator::new(&memory);
        let mut iterator = iterator.peekable();
        assert_eq!(iterator.peek().unwrap().value.a, 0);
        assert_eq!(iterator.next().unwrap().value.b, 1);
        assert_eq!(iterator.peek().unwrap().value.a, 3);
        assert_eq!(iterator.next().unwrap().value.c, 5);
        assert_eq!(iterator.peek().unwrap().value.a, 6);
        assert_eq!(iterator.next().unwrap().value.a, 6);
        assert_eq!(iterator.peek().unwrap().value.a, 9);
        assert_eq!(iterator.next().unwrap().value.b, 10);
        assert!(iterator.peek().is_none());
        assert!(iterator.next().is_none());
    }

    #[test]
    fn align_test() {
        assert_eq!(align_address_down(0, 8), 0);
        assert_eq!(align_address_up(0, 8), 0);
        assert_eq!(align_address_down(1, 8), 0);
        assert_eq!(align_address_up(1, 8), 8);
        assert_eq!(align_address_down(8, 8), 8);
        assert_eq!(align_address_up(8, 8), 8);
    }

    #[test]
    fn constant_array_test() {
        let array1: [u8; 42] = constant_initialized_array(&|| 1 * 10 + 2);
        assert_eq!(array1, [12; 42]);
        // Lets try it on a non-copy type (an atomic).
        let array2: [AtomicU8; 42] = constant_initialized_array(&|| AtomicU8::from(108));
        assert_eq!(array2[7].load(Ordering::SeqCst), 108);
    }
}
