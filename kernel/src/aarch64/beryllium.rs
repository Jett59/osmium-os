#[repr(u32)]
pub enum BootRequestTagType {
    StackPointer = 0,
    MemoryMap = 1,
    FrameBuffer = 2,
}

#[repr(C, align(8))]
pub struct StackPointerTag {
    tag_type: BootRequestTagType, // = BootRequestTagType::StackPointer
    size: u16,                    // = 24 (64-bit) or 16 (32-bit)
    flags: u16,
    base: *mut u8,
    memory_size: usize,
}

#[repr(C, align(8))]
struct MemoryMapTag {
    tag_type: BootRequestTagType, // = BootRequestTagType::MemoryMap
    size: u16,                    // = 24 (64-bit) or 16 (32-bit)
    flags: u16,
    base: *mut u8,
    memory_size: usize,
}

#[repr(C)]
pub struct MemoryMapEntry {
    address: *mut u8,
    size: usize,
    memory_type: MemoryMapEntryType,
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
struct FrameBufferTag {
    tag_type: BootRequestTagType, // = BootRequestTagType::FrameBuffer
    size: u16,                    // 40 (64-bit) or 32 (32-bit)
    flags: u16,
    address: usize,
    width: u32,
    height: u32,
    pitch: u32,
    bits_per_pixel: u32,
    red_byte: u8,
    green_byte: u8,
    blue_byte: u8,
}
