use core::{mem::size_of, slice};

pub trait Validateable {
    // Ensure that an instance of this type is valid. This is used to ensure that
    // objects which are created by reinterpreting some region of memory are in fact instances of the correct type.
    fn validate(&self) -> bool;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
    Native,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FromBytesError {
    InvalidSize,
}

trait FromBytes<'lifetime>: Sized {
    fn from_bytes(endianness: Endianness, bytes: &'lifetime [u8]) -> Result<Self, FromBytesError>;

    const SIZE: usize;
}

macro_rules! impl_from_bytes {
    ($type:ty) => {
        impl FromBytes<'_> for $type {
            fn from_bytes(endianness: Endianness, bytes: &[u8]) -> Result<Self, FromBytesError> {
                match endianness {
                    Endianness::Little => Ok(Self::from_le_bytes(
                        bytes.try_into().map_err(|_| FromBytesError::InvalidSize)?,
                    )),
                    Endianness::Big => Ok(Self::from_be_bytes(
                        bytes.try_into().map_err(|_| FromBytesError::InvalidSize)?,
                    )),
                    Endianness::Native => Ok(Self::from_ne_bytes(
                        bytes.try_into().map_err(|_| FromBytesError::InvalidSize)?,
                    )),
                }
            }

            const SIZE: usize = size_of::<Self>();
        }
    };
}

impl_from_bytes!(u8);
impl_from_bytes!(u16);
impl_from_bytes!(u32);
impl_from_bytes!(u64);
impl_from_bytes!(u128);
impl_from_bytes!(i8);
impl_from_bytes!(i16);
impl_from_bytes!(i32);
impl_from_bytes!(i64);
impl_from_bytes!(i128);

/// Create a type which wraps a byte slice.
///
/// This allows for easily interpreting a region of memory as some specific structural type, without worrying about casting slices or anything.
/// It provides getters for each of the fields of the type.
///
/// All fields must implement the `FromBytes` trait (this includes all integer types automatically).
/// The created type also implements `FromBytes`, allowing for nesting.
/// Members are assumed to be packed (i.e. not automatically aligned to their natural boundaries).
#[macro_export]
macro_rules! memory_struct {
    ($visibility:vis struct $Name:ident<$lifetime:lifetime> {
        $(
            $field_name:ident: $field_type:ty
        ),* $(,)?
    }) => {
        $visibility struct $Name<$lifetime> {
            memory: &$lifetime [u8],
            endianness: $crate::memory::Endianness,
        }

        impl<'lifetime> $crate::memory::FromBytes<'lifetime> for $Name<'lifetime>
        where
            $($field_type: $crate::memory::FromBytes<'lifetime>),*
        {
            fn from_bytes(endianness: $crate::memory::Endianness, bytes: &'lifetime [u8]) -> Result<Self, $crate::memory::FromBytesError> {
                if bytes.len() < Self::SIZE {
                    return Err($crate::memory::FromBytesError::InvalidSize);
                }
                Ok(Self {
                    memory: bytes,
                    endianness,
                })
            }

            const SIZE: usize = $(
                (<$field_type as $crate::memory::FromBytes>::SIZE) +
            )* 0;
        }

        {
            #[repr(C, packed)]
            struct Layout<$lifetime> {
                $(
                    $field_name: [u8; <$field_type as $crate::memory::FromBytes>::SIZE]
                ),*
            }
            impl<'lifetime> $Name<'lifetime>
            where
                $($field_type: $crate::memory::FromBytes<'lifetime>),*
            {
                $(
                    $visibility fn $field_name(&self) -> $field_type {
                        let offset = core::mem::offset_of!(Layout, $field_name);
                        let bytes = &self.memory[offset..offset + <$field_type as $crate::memory::FromBytes>::SIZE];
                        $crate::memory::FromBytes::from_bytes(self.endianness, bytes).unwrap()
                    }
                )*
            }
        }
    };
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
    if alignment.is_power_of_two() {
        address & !(alignment - 1)
    } else {
        (address / alignment) * alignment
    }
}
pub fn align_address_up(address: usize, alignment: usize) -> usize {
    if alignment.is_power_of_two() {
        (address + alignment - 1) & !(alignment - 1)
    } else {
        ((address + alignment - 1) / alignment) * alignment
    }
}

#[cfg(test)]
mod test {
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
    fn memory_struct_test() {
        memory_struct! {
            pub struct TestStruct<'lifetime> {
                a: u8,
                b: u16,
                c: u32,
            }
        }

        let memory = [0, 1, 2, 3, 4, 5, 6, 7];
        let test_struct = TestStruct::from_bytes(Endianness::Little, &memory).unwrap();

        assert_eq!(test_struct.a(), 0);
        assert_eq!(test_struct.b(), 0x0201);
        assert_eq!(test_struct.c(), 0x06050403);

        memory_struct! {
            pub struct TestStruct2<'lifetime> {
                a: u8,
                b: TestStruct<'lifetime>,
                c: u32,
            }
        }

        assert_eq!(TestStruct2::SIZE, 12);
        assert!(TestStruct2::from_bytes(Endianness::Little, &memory).is_err());

        let memory = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

        let test_struct = TestStruct2::from_bytes(Endianness::Little, &memory).unwrap();

        assert_eq!(test_struct.a(), 0);
        assert_eq!(test_struct.b().a(), 1);
        assert_eq!(test_struct.b().b(), 0x0302);
        assert_eq!(test_struct.b().c(), 0x07060504);
        assert_eq!(test_struct.c(), 0x0b0a0908);
    }
}
