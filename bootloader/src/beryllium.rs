#[repr(u32)]
#[derive(Debug, Clone)]
pub enum BootRequestTagType {
    StackPointer = 0,
    MemoryMap = 1,
    FrameBuffer = 2,
}

#[repr(C, align(8))]
#[derive(Debug)]
pub struct TagHeader {
    pub tag_type: BootRequestTagType,
    pub size: u16,
    pub flags: u16,
}

#[repr(C, align(8))]
#[derive(Debug, Clone)]
pub struct StackPointerTag {
    pub tag_type: BootRequestTagType, // = BootRequestTagType::StackPointer
    pub size: u16,                    // = 24 (64-bit) or 16 (32-bit)
    pub flags: u16,
    pub base: *mut u8,
    pub memory_size: usize,
}

#[repr(C, align(8))]
#[derive(Debug)]
pub struct MemoryMapTag {
    pub tag_type: BootRequestTagType, // = BootRequestTagType::MemoryMap
    pub size: u16,                    // = 24 (64-bit) or 16 (32-bit)
    pub flags: u16,
    pub base: *mut u8,
    pub memory_size: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct MemoryMapEntry {
    pub address: *mut u8,
    pub size: usize,
    pub memory_type: MemoryMapEntryType,
}

#[repr(u32)]
#[derive(Debug)]
pub enum MemoryMapEntryType {
    Reserved = 0,
    Available = 1,
    EfiRuntime = 2,
    AcpiReclaimable = 3,
    Kernel = 4,
}

#[repr(C, align(8))]
#[derive(Debug)]
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

#[derive(Debug)]
pub struct BerylliumInfo<'lifetime> {
    pub stack_pointer: Option<&'lifetime StackPointerTag>,
    pub memory_map: Option<&'lifetime mut MemoryMapTag>,
    pub frame_buffer: Option<&'lifetime mut FrameBufferTag>,
}

pub fn parse_tags(tags: &mut [u8]) -> BerylliumInfo {
    let mut result = BerylliumInfo {
        stack_pointer: None,
        memory_map: None,
        frame_buffer: None,
    };
    let mut remaining_bytes = tags;
    while remaining_bytes.len() > 0 {
        let header = unsafe { &*(remaining_bytes.as_ptr() as *const TagHeader) };
        assert!(header.size % 8 == 0);
        match header.tag_type {
            BootRequestTagType::StackPointer => {
                if cfg!(target_pointer_width = "64") {
                    assert!(header.size == 24);
                } else {
                    assert!(header.size == 16);
                }
                result.stack_pointer =
                    Some(unsafe { &*(remaining_bytes.as_ptr() as *const StackPointerTag) });
            }
            BootRequestTagType::MemoryMap => {
                if cfg!(target_pointer_width = "64") {
                    assert!(header.size == 24);
                } else {
                    assert!(header.size == 16);
                }
                result.memory_map =
                    Some(unsafe { &mut *(remaining_bytes.as_mut_ptr() as *mut MemoryMapTag) });
            }
            BootRequestTagType::FrameBuffer => {
                if cfg!(target_pointer_width = "64") {
                    assert!(header.size == 40);
                } else {
                    assert!(header.size == 32);
                }
                result.frame_buffer =
                    Some(unsafe { &mut *(remaining_bytes.as_mut_ptr() as *mut FrameBufferTag) });
            }
        }
        remaining_bytes = &mut remaining_bytes[header.size as usize..];
    }
    result
}
