//! A basic reader for the initramfs.
//!
//! The initramfs is a simple TAR file containing all the necessary files to bring up the machine, load drivers and do whatever else.
//!
//! Although the initramfs is traditionally just for initialization, it may well be used as the root filesystem if the OS hasn't been installed.

use core::ops::Deref;

use alloc::{collections::BTreeMap, string::String};

use crate::{
    memory::{
        Array, DynamicallySized, DynamicallySizedItem, DynamicallySizedObjectIterator, Endianness,
        FromBytes, FromBytesError,
    },
    memory_struct,
};

#[derive(Debug)]
struct OctalString<const MAX_LENGTH: usize>(u32);

impl<const MAX_LENGTH: usize> FromBytes<'_> for OctalString<MAX_LENGTH> {
    fn from_bytes(
        _endianness: Endianness,
        bytes: &[u8],
    ) -> Result<Self, crate::memory::FromBytesError> {
        let mut result = 0;
        for byte in &bytes[..MAX_LENGTH] {
            if *byte == 0 {
                break;
            }
            if *byte < b'0' || *byte > b'7' {
                return Err(FromBytesError::InvalidMemory);
            }
            result *= 8;
            result += (byte - b'0') as u32;
        }
        Ok(Self(result))
    }

    const SIZE: usize = MAX_LENGTH;
}

impl<const MAX_LENGTH: usize> From<OctalString<MAX_LENGTH>> for u32 {
    fn from(octal_string: OctalString<MAX_LENGTH>) -> Self {
        octal_string.0
    }
}

impl<const MAX_LENGTH: usize> Deref for OctalString<MAX_LENGTH> {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

memory_struct! {
    struct FileHeader<'lifetime> {
        file_name: Array<'lifetime, u8, 100>,
        file_mode: OctalString<8>,
        owner_user_id: OctalString<8>,
        group_user_id: OctalString<8>,
        file_size: OctalString<12>,
        last_modification_time: OctalString<12>,
        checksum: OctalString<8>,
        file_type: u8,
        linked_file_name: Array<'lifetime, u8, 100>,
        ustar_indicator: Array<'lifetime, u8, 6>,
        ustar_version: Array<'lifetime, u8, 2>,
        owner_user_name: Array<'lifetime, u8, 32>,
        owner_group_name: Array<'lifetime, u8, 32>,
        device_major_number: OctalString<8>,
        device_minor_number: OctalString<8>,
        file_name_prefix: Array<'lifetime, u8, 155>,
    }
}

impl<'lifetime> DynamicallySized for FileHeader<'lifetime> {
    fn size(&self) -> usize {
        *self.file_size() as usize + 512
    }
    const ALIGNMENT: usize = 512;
}

pub fn read_initramfs(initramfs: &[u8]) -> BTreeMap<String, &[u8]> {
    let mut result = BTreeMap::new();
    for DynamicallySizedItem {
        value,
        value_memory,
    } in DynamicallySizedObjectIterator::<FileHeader>::new(Endianness::Native, initramfs)
    {
        if *value.file_size() != 0 {
            let file_name = &*value.file_name();
            // We need to find the null terminator, since String::from_utf8_lossy doesn't know where to stop.
            let null_terminator_index = file_name
                .iter()
                .position(|&byte| byte == 0)
                .unwrap_or(file_name.len());
            let file_name = String::from_utf8_lossy(&file_name[..null_terminator_index]);
            result.insert(
                file_name.into_owned(),
                &value_memory[512..512 + *value.file_size() as usize],
            );
        }
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;

    fn octal_string_test() {
        let octal_string = OctalString::<4>::from_bytes(Endianness::Native, b"3210");
        assert!(octal_string.is_ok());
        assert_eq!(octal_string.unwrap().0, 0o3210);

        let octal_string = OctalString::<4>::from_bytes(Endianness::Native, b"8765");
        assert!(octal_string.is_err());

        let octal_string = OctalString::<4>::from_bytes(Endianness::Native, b"12345");
        assert!(octal_string.is_ok());
        assert_eq!(octal_string.unwrap().0, 0o1234);

        let octal_string = OctalString::<4>::from_bytes(Endianness::Native, b"12\x003");
        assert!(octal_string.is_ok());
        assert_eq!(octal_string.unwrap().0, 0o12);
    }

    #[test]
    fn initramfs_test() {
        let test_initramfs_data = include_bytes!("test/initramfs.tar");
        let initramfs = read_initramfs(test_initramfs_data);

        assert_eq!(initramfs.len(), 2);
        assert!(initramfs.contains_key("test.txt"));
        assert!(initramfs.contains_key("yes/agree.txt"));

        let test_txt = initramfs.get("test.txt").unwrap();
        assert_eq!(test_txt.len(), 8);
        assert_eq!(test_txt, b"testing\n");
        let yes_agree_txt = initramfs.get("yes/agree.txt").unwrap();
        assert_eq!(yes_agree_txt.len(), 10);
        assert_eq!(yes_agree_txt, b"certainly\n");
    }
}
