use core::{
    ffi::CStr,
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
    Header {
        field: &'static str,
        expected: String,
        actual: String,
    },
    ProgramHeaderEntry {
        field: &'static str,
        expected: String,
        actual: String,
        index: usize,
    },
    SectionHeaderEntry {
        field: &'static str,
        expected: String,
        actual: String,
        index: usize,
    },
}

impl Display for ElfValidationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ElfValidationError::Header {
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
            ElfValidationError::ProgramHeaderEntry {
                field,
                expected,
                actual,
                index,
            } => {
                write!(
                    f,
                    "Invalid ELF program header entry: {} should be {}, but is {} (index {})",
                    field, expected, actual, index
                )
            }
            ElfValidationError::SectionHeaderEntry {
                field,
                expected,
                actual,
                index,
            } => {
                write!(
                    f,
                    "Invalid ELF section header entry: {} should be {}, but is {} (index {})",
                    field, expected, actual, index
                )
            }
        }
    }
}

fn validate_header(header: &ElfHeader) -> Result<(), ElfValidationError> {
    if header.signature != [0x7f, b'E', b'L', b'F'] {
        return Err(ElfValidationError::Header {
            field: "signature",
            expected: "[7f, 45, 4c, 46]".to_string(),
            actual: format!("{:x?}", header.signature),
        });
    }
    if header.bits != CURRENT_BITS {
        return Err(ElfValidationError::Header {
            field: "bits",
            expected: CURRENT_BITS.to_string(),
            actual: header.bits.to_string(),
        });
    }
    if header.endian != CURRENT_ENDIAN {
        return Err(ElfValidationError::Header {
            field: "endian",
            expected: CURRENT_ENDIAN.to_string(),
            actual: header.endian.to_string(),
        });
    }
    if header.header_version != 1 {
        return Err(ElfValidationError::Header {
            field: "header_version",
            expected: "1".to_string(),
            actual: header.header_version.to_string(),
        });
    }
    if header.abi != 0 {
        return Err(ElfValidationError::Header {
            field: "abi",
            expected: "0".to_string(),
            actual: header.abi.to_string(),
        });
    }
    if header.file_type != 2 {
        return Err(ElfValidationError::Header {
            field: "type",
            expected: "2".to_string(),
            actual: header.file_type.to_string(),
        });
    }
    if header.machine != CURRENT_MACHINE_ID {
        return Err(ElfValidationError::Header {
            field: "machine",
            expected: format!("{:#x}", CURRENT_MACHINE_ID),
            actual: format!("{:#x}", header.machine),
        });
    }
    if header.version != 1 {
        return Err(ElfValidationError::Header {
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
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub file_offset: usize,
    pub virtual_address: usize,
    pub size_in_file: usize,
    pub size_in_memory: usize,
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
            return Err(ElfValidationError::ProgramHeaderEntry {
                field: "size",
                expected: format!("<= {}", bytes.len() - entry_offset),
                actual: format!("{}", program_header_entry_size),
                index: i,
            });
        }
        let entry = unsafe { &*(bytes[entry_offset..].as_ptr() as *const ProgramHeaderEntry) };
        if entry.program_type != 1 {
            continue;
        }
        if entry.offset as usize + entry.file_size as usize > bytes.len() {
            return Err(ElfValidationError::ProgramHeaderEntry {
                field: "file_size",
                expected: format!("<= {}", bytes.len() - entry.offset as usize),
                actual: format!("{}", entry.file_size),
                index: i,
            });
        }
        segments.push(LoadableSegment {
            readable: entry.flags & 4 != 0,
            writable: entry.flags & 2 != 0,
            executable: entry.flags & 1 != 0,
            file_offset: entry.offset as usize,
            virtual_address: entry.virtual_address as usize,
            size_in_file: entry.file_size as usize,
            size_in_memory: entry.memory_size as usize,
        });
    }
    Ok(segments)
}

#[cfg(target_pointer_width = "64")]
#[repr(C)]
struct SectionHeaderEntry {
    name: u32, // Offset into the section header string table.
    section_type: u32,
    flags: u64,
    virtual_address: u64,
    file_offset: u64,
    size: u64,
    link: u32, // Link to another section
    info: u32,
    address_alignment: u64,
    entry_size: u64, // Only applicable if this is a table.
}

// A version of the section header which is in a nicer format.
#[derive(Debug)]
pub struct Section {
    pub name: String,
    pub file_offset: usize,
    pub virtual_address: usize,
    pub size: usize,
    pub present_in_file: bool, // true = progbits, false = nobits
}

const SECTION_TYPE_PROGBITS: u32 = 1;
const SECTION_TYPE_NOBITS: u32 = 8;

fn read_section_header(
    elf_header: &ElfHeader,
    bytes: &[u8],
) -> Result<Vec<Section>, ElfValidationError> {
    let mut sections = Vec::with_capacity(elf_header.section_header_entry_count as usize);
    let section_header_offset = elf_header.section_header_offset as usize;
    let section_header_entry_size = elf_header.section_header_entry_size as usize;
    let section_header_entry_count = elf_header.section_header_entry_count as usize;
    let string_section_index = elf_header.section_header_name_table_index as usize;
    let string_section_offset =
        section_header_offset + string_section_index * section_header_entry_size;
    if string_section_offset + section_header_entry_size > bytes.len() {
        return Err(ElfValidationError::SectionHeaderEntry {
            field: "size",
            expected: format!("<= {}", bytes.len() - string_section_offset),
            actual: format!("{}", section_header_entry_size),
            index: string_section_index,
        });
    }
    let string_section_entry =
        unsafe { &*(bytes[string_section_offset..].as_ptr() as *const SectionHeaderEntry) };
    if string_section_entry.file_offset as usize + string_section_entry.size as usize > bytes.len()
    {
        return Err(ElfValidationError::SectionHeaderEntry {
            field: "size",
            expected: format!(
                "<= {}",
                bytes.len() - string_section_entry.file_offset as usize
            ),
            actual: format!("{}", string_section_entry.size),
            index: string_section_index,
        });
    }
    let string_section_bytes = &bytes[string_section_entry.file_offset as usize
        ..string_section_entry.file_offset as usize + string_section_entry.size as usize];
    for i in 0..section_header_entry_count {
        let entry_offset = section_header_offset + i * section_header_entry_size;
        if entry_offset + section_header_entry_size > bytes.len() {
            return Err(ElfValidationError::SectionHeaderEntry {
                field: "section_header_entry_size",
                expected: format!("<= {}", bytes.len() - entry_offset),
                actual: format!("{}", section_header_entry_size),
                index: i,
            });
        }
        let entry = unsafe { &*(bytes[entry_offset..].as_ptr() as *const SectionHeaderEntry) };
        if entry.section_type != SECTION_TYPE_PROGBITS && entry.section_type != SECTION_TYPE_NOBITS
        {
            continue;
        }
        // nobits entries don't need to fit into the file
        if entry.section_type == SECTION_TYPE_PROGBITS
            && entry.file_offset as usize + entry.size as usize > bytes.len()
        {
            return Err(ElfValidationError::SectionHeaderEntry {
                field: "size",
                expected: format!("<= {}", bytes.len() - entry.file_offset as usize),
                actual: format!("{}", entry.size),
                index: i,
            });
        }
        let name_offset = entry.name;
        if name_offset as usize >= string_section_bytes.len() {
            return Err(ElfValidationError::SectionHeaderEntry {
                field: "name",
                expected: format!("< {}", string_section_bytes.len()),
                actual: format!("{}", name_offset),
                index: i,
            });
        }
        let name = CStr::from_bytes_until_nul(&string_section_bytes[name_offset as usize..])
            .map_err(|_| ElfValidationError::SectionHeaderEntry {
                field: "name",
                expected: "valid UTF-8".to_string(),
                actual: "invalid UTF-8".to_string(),
                index: i,
            })?
            .to_str()
            .map_err(|_| ElfValidationError::SectionHeaderEntry {
                field: "name",
                expected: "valid UTF-8".to_string(),
                actual: "invalid UTF-8".to_string(),
                index: i,
            })?
            .to_string();
        sections.push(Section {
            name,
            file_offset: entry.file_offset as usize,
            virtual_address: entry.virtual_address as usize,
            size: entry.size as usize,
            present_in_file: entry.section_type == SECTION_TYPE_PROGBITS,
        });
    }
    Ok(sections)
}

#[derive(Debug)]
pub struct ElfBinary {
    pub loadable_segments: Vec<LoadableSegment>,
    pub sections: Vec<Section>,

    pub entrypoint: usize,
}

pub fn load_elf(bytes: &[u8]) -> Result<ElfBinary, ElfValidationError> {
    if bytes.len() < size_of::<ElfHeader>() {
        return Err(ElfValidationError::Header {
            field: "size",
            expected: format!(">= {}", size_of::<ElfHeader>()),
            actual: format!("{}", bytes.len()),
        });
    }
    let header = unsafe { &*(bytes.as_ptr() as *const ElfHeader) };
    validate_header(header)?;
    let loadable_segments = read_program_header(header, bytes)?;
    let sections = read_section_header(header, bytes)?;
    Ok(ElfBinary {
        loadable_segments,
        sections,
        entrypoint: header.entrypoint as usize,
    })
}
