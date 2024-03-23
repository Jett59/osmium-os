use core::mem::size_of;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootRequestTagType {
    StackPointer = 0,
    MemoryMap = 1,
    FrameBuffer = 2,
    Acpi = 3,
    InitialRamdisk = 4,
}

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct TagHeader {
    pub tag_type: BootRequestTagType,
    pub size: u16,
    pub flags: u16,
}

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct StackPointerTag {
    pub tag_type: BootRequestTagType, // = BootRequestTagType::StackPointer
    pub size: u16,                    // = 24 (64-bit) or 16 (32-bit)
    pub flags: u16,
    pub base: *mut u8,
    pub memory_size: usize,
}

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct MemoryMapTag {
    pub tag_type: BootRequestTagType, // = BootRequestTagType::MemoryMap
    pub size: u16,                    // = 24 (64-bit) or 16 (32-bit)
    pub flags: u16,
    pub base: *const u8,
    pub memory_size: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryMapEntry {
    pub address: *mut u8,
    pub size: usize,
    pub memory_type: MemoryMapEntryType,
}

impl TryFrom<&[u8]> for &MemoryMapEntry {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != size_of::<MemoryMapEntry>() {
            return Err("Invalid memory map entry size");
        }
        unsafe {
            let result = &*(value.as_ptr() as *const MemoryMapEntry);
            MemoryMapEntryType::try_from(result.memory_type as u32)?;
            Ok(result)
        }
    }
}

impl TryFrom<&mut [u8]> for &mut MemoryMapEntry {
    type Error = &'static str;

    fn try_from(value: &mut [u8]) -> Result<Self, Self::Error> {
        if value.len() != size_of::<MemoryMapEntry>() {
            return Err("Invalid memory map entry size");
        }
        unsafe {
            let result = &mut *(value.as_ptr() as *mut MemoryMapEntry);
            MemoryMapEntryType::try_from(result.memory_type as u32)?;
            Ok(result)
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMapEntryType {
    Reserved = 0,
    Available = 1,
    EfiRuntime = 2,
    AcpiReclaimable = 3,
    Kernel = 4,
}

impl TryFrom<u32> for MemoryMapEntryType {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MemoryMapEntryType::Reserved),
            1 => Ok(MemoryMapEntryType::Available),
            2 => Ok(MemoryMapEntryType::EfiRuntime),
            3 => Ok(MemoryMapEntryType::AcpiReclaimable),
            4 => Ok(MemoryMapEntryType::Kernel),
            _ => Err("Invalid memory map entry type"),
        }
    }
}

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct FrameBufferTag {
    pub tag_type: BootRequestTagType, // = BootRequestTagType::FrameBuffer
    pub size: u16,                    // 40 (64-bit) or 32 (32-bit)
    pub flags: u16,
    pub address: usize,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bits_per_pixel: u32,
    pub red_byte: u8,
    pub green_byte: u8,
    pub blue_byte: u8,
}

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct AcpiTag {
    pub tag_type: BootRequestTagType, // = BootRequestTagType::Acpi
    pub size: u16,                    // 16 (64-bit) or 12 (32-bit)
    pub flags: u16,
    pub rsdt: usize,
}

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct InitialRamdiskTag {
    pub tag_type: BootRequestTagType, // = BootRequestTagType::MemoryMap
    pub size: u16,                    // = 24 (64-bit) or 16 (32-bit)
    pub flags: u16,
    pub base: *const u8,
    pub file_size: usize,
}
