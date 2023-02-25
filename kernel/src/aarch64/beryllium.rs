#[repr(u32)]
pub enum BootRequestTagType {
    StackPointer = 0,
    MemoryMap = 1,
    FrameBuffer = 2,
}

#[repr(C, align(8))]
pub struct StackPointerTag {
    pub tag_type: BootRequestTagType, // = BootRequestTagType::StackPointer
    pub size: u16,                    // = 24 (64-bit) or 16 (32-bit)
    pub flags: u16,
    pub base: *mut u8,
    pub memory_size: usize,
}

#[repr(C, align(8))]
pub struct MemoryMapTag {
    pub tag_type: BootRequestTagType, // = BootRequestTagType::MemoryMap
    pub size: u16,                    // = 24 (64-bit) or 16 (32-bit)
    pub flags: u16,
    pub base: *mut u8,
    pub memory_size: usize,
}

#[repr(C)]
pub struct MemoryMapEntry {
    pub address: *mut u8,
    pub size: usize,
    pub memory_type: MemoryMapEntryType,
}

#[repr(u32)]
pub enum MemoryMapEntryType {
    Reserved = 0,
    Available = 1,
    EfiRuntime = 2,
    AcpiReclaimable = 3,
    Kernel = 4,
}

#[repr(C, align(8))]
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
