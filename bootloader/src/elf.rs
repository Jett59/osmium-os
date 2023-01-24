use core::{
    fmt::{Display, Formatter},
    mem::size_of,
};

use alloc::{
    fmt, format,
    string::{String, ToString},
    vec::Vec,
};

#[cfg(target_pointer_width = "64")]
#[repr(C)]
struct ElfHeader {
    signature: [u8; 4], // 0x7f, 'E', 'L', 'F'
    bits: u8,           // = 2: 64-bit
    endian: u8,         // = 1: little endian or 2: big endian
    header_version: u8, // = 1
    abi: u8,            // = 0: System V
    _padding: [u8; 8],
    file_type: u16, // = 2: executable
    machine: u16,   // = 0xb7: aarch64
    version: u32,   // = 1
    entrypoint: u64,
    program_header_offset: u64,
    section_header_offset: u64,
    flags: u32,
    header_size: u16,
    program_header_entry_size: u16,
    program_header_entry_count: u16,
    section_header_entry_size: u16,
    section_header_entry_count: u16,
    section_header_name_table_index: u16,
}

#[cfg(target_arch = "aarch64")]
const CURRENT_MACHINE_ID: u16 = 0xb7;
#[cfg(target_arch = "x86_64")]
const CURRENT_MACHINE_ID: u16 = 0x3e;

#[cfg(target_pointer_width = "64")]
const CURRENT_BITS: u8 = 2;
#[cfg(target_pointer_width = "32")]
const CURRENT_BITS: u8 = 1;

#[cfg(target_endian = "little")]
const CURRENT_ENDIAN: u8 = 1;
#[cfg(target_endian = "big")]
const CURRENT_ENDIAN: u8 = 2;

#[derive(Debug)]
pub enum ElfValidationError {
    InvalidHeader {
        field: &'static str,
        expected: String,
        actual: String,
    },
    InvalidProgramHeaderEntry {
        field: &'static str,
        expected: String,
        actual: String,
    },
    InvalidSectionHeaderEntry {
        field: &'static str,
        expected: String,
        actual: String,
    },
}

impl Display for ElfValidationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ElfValidationError::InvalidHeader {
                field,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Invalid ELF header: {} should be {}, but is {}",
                    field, expected, actual
                )
            }
            ElfValidationError::InvalidProgramHeaderEntry {
                field,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Invalid ELF program header entry: {} should be {}, but is {}",
                    field, expected, actual
                )
            }
            ElfValidationError::InvalidSectionHeaderEntry {
                field,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Invalid ELF section header entry: {} should be {}, but is {}",
                    field, expected, actual
                )
            }
        }
    }
}

fn validate_header(header: &ElfHeader) -> Result<(), ElfValidationError> {
    if header.signature != [0x7f, b'E', b'L', b'F'] {
        return Err(ElfValidationError::InvalidHeader {
            field: "signature",
            expected: "[7f, 45, 4c, 46]".to_string(),
            actual: format!("{:x?}", header.signature),
        });
    }
    if header.bits != CURRENT_BITS {
        return Err(ElfValidationError::InvalidHeader {
            field: "bits",
            expected: CURRENT_BITS.to_string(),
            actual: header.bits.to_string(),
        });
    }
    if header.endian != CURRENT_ENDIAN {
        return Err(ElfValidationError::InvalidHeader {
            field: "endian",
            expected: CURRENT_ENDIAN.to_string(),
            actual: header.endian.to_string(),
        });
    }
    if header.header_version != 1 {
        return Err(ElfValidationError::InvalidHeader {
            field: "header_version",
            expected: "1".to_string(),
            actual: header.header_version.to_string(),
        });
    }
    if header.abi != 0 {
        return Err(ElfValidationError::InvalidHeader {
            field: "abi",
            expected: "0".to_string(),
            actual: header.abi.to_string(),
        });
    }
    if header.file_type != 2 {
        return Err(ElfValidationError::InvalidHeader {
            field: "type",
            expected: "2".to_string(),
            actual: header.file_type.to_string(),
        });
    }
    if header.machine != CURRENT_MACHINE_ID {
        return Err(ElfValidationError::InvalidHeader {
            field: "machine",
            expected: format!("{:#x}", CURRENT_MACHINE_ID),
            actual: format!("{:#x}", header.machine),
        });
    }
    if header.version != 1 {
        return Err(ElfValidationError::InvalidHeader {
            field: "version",
            expected: "1".to_string(),
            actual: header.version.to_string(),
        });
    }
    Ok(())
}

#[cfg(target_pointer_width = "64")]
#[repr(C)]
struct ProgramHeaderEntry {
    program_type: u32,
    flags: u32,
    offset: u64,
    virtual_address: u64,
    _physical_address: u64, // We don't respect that.
    file_size: u64,
    memory_size: u64,
    alignment: u64,
}

// A version of the program header which is in a nicer format.
#[derive(Debug)]
pub struct LoadableSegment {
    readable: bool,
    writable: bool,
    executable: bool,
    offset: usize,
    virtual_address: usize,
    size_in_file: usize,
    size_in_memory: usize,
}

fn read_program_header(
    elf_header: &ElfHeader,
    bytes: &[u8],
) -> Result<Vec<LoadableSegment>, ElfValidationError> {
    let mut segments = Vec::with_capacity(elf_header.program_header_entry_count as usize);
    let program_header_offset = elf_header.program_header_offset as usize;
    let program_header_entry_size = elf_header.program_header_entry_size as usize;
    let program_header_entry_count = elf_header.program_header_entry_count as usize;
    for i in 0..program_header_entry_count {
        let entry_offset = program_header_offset + i * program_header_entry_size;
        if entry_offset + program_header_entry_size > bytes.len() {
            return Err(ElfValidationError::InvalidProgramHeaderEntry {
                field: "size",
                expected: format!("<= {}", bytes.len() - entry_offset),
                actual: format!("{}", program_header_entry_size),
            });
        }
        let entry = unsafe { &*(bytes[entry_offset..].as_ptr() as *const ProgramHeaderEntry) };
        if entry.program_type != 1 {
            continue;
        }
        if entry.offset as usize + entry.file_size as usize > bytes.len() {
            return Err(ElfValidationError::InvalidProgramHeaderEntry {
                field: "file_size",
                expected: format!("<= {}", bytes.len() - entry.offset as usize),
                actual: format!("{}", entry.file_size),
            });
        }
        segments.push(LoadableSegment {
            readable: entry.flags & 4 != 0,
            writable: entry.flags & 2 != 0,
            executable: entry.flags & 1 != 0,
            offset: entry.offset as usize,
            virtual_address: entry.virtual_address as usize,
            size_in_file: entry.file_size as usize,
            size_in_memory: entry.memory_size as usize,
        });
    }
    Ok(segments)
}

#[derive(Debug)]
pub struct ElfBinary {
    loadable_segments: Vec<LoadableSegment>,
}

pub fn load_elf(bytes: &[u8]) -> Result<ElfBinary, ElfValidationError> {
    if bytes.len() < size_of::<ElfHeader>() {
        return Err(ElfValidationError::InvalidHeader {
            field: "size",
            expected: format!(">= {}", size_of::<ElfHeader>()),
            actual: format!("{}", bytes.len()),
        });
    }
    let header = unsafe { &*(bytes.as_ptr() as *const ElfHeader) };
    validate_header(header)?;
    let loadable_segments = read_program_header(header, bytes)?;
    Ok(ElfBinary { loadable_segments })
}
