pub use common::beryllium::*;

#[derive(Debug)]
pub struct BerylliumInfo<'lifetime> {
    pub stack_pointer: Option<&'lifetime StackPointerTag>,
    pub stack_pointer_offset: Option<usize>,
    pub memory_map: Option<&'lifetime mut MemoryMapTag>,
    pub memory_map_offset: Option<usize>,
    pub frame_buffer: Option<&'lifetime mut FrameBufferTag>,
    pub frame_buffer_offset: Option<usize>,
    pub acpi: Option<&'lifetime mut AcpiTag>,
    pub acpi_offset: Option<usize>,
    pub module: Option<&'lifetime mut ModuleTag>,
    pub module_offset: Option<usize>,
}

pub fn parse_tags(tags: &mut [u8]) -> BerylliumInfo {
    let mut result = BerylliumInfo {
        stack_pointer: None,
        stack_pointer_offset: None,
        memory_map: None,
        memory_map_offset: None,
        frame_buffer: None,
        frame_buffer_offset: None,
        acpi: None,
        acpi_offset: None,
        module: None,
        module_offset: None,
    };
    let mut remaining_bytes = tags;
    let mut current_offset = 0;
    while !remaining_bytes.is_empty() {
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
                result.stack_pointer_offset = Some(current_offset);
            }
            BootRequestTagType::MemoryMap => {
                if cfg!(target_pointer_width = "64") {
                    assert!(header.size == 24);
                } else {
                    assert!(header.size == 16);
                }
                result.memory_map =
                    Some(unsafe { &mut *(remaining_bytes.as_mut_ptr() as *mut MemoryMapTag) });
                result.memory_map_offset = Some(current_offset);
            }
            BootRequestTagType::FrameBuffer => {
                if cfg!(target_pointer_width = "64") {
                    assert!(header.size == 40);
                } else {
                    assert!(header.size == 32);
                }
                result.frame_buffer =
                    Some(unsafe { &mut *(remaining_bytes.as_mut_ptr() as *mut FrameBufferTag) });
                result.frame_buffer_offset = Some(current_offset);
            }
            BootRequestTagType::Acpi => {
                if cfg!(target_pointer_width = "64") {
                    assert!(header.size == 16);
                } else {
                    assert!(header.size == 12);
                }
                result.acpi = Some(unsafe { &mut *(remaining_bytes.as_mut_ptr() as *mut AcpiTag) });
                result.acpi_offset = Some(current_offset);
            }
            BootRequestTagType::Module => {
                if cfg!(target_pointer_width = "64") {
                    assert!(header.size == 24);
                } else {
                    assert!(header.size == 16);
                }
                result.module =
                    Some(unsafe { &mut *(remaining_bytes.as_mut_ptr() as *mut ModuleTag) });
                result.module_offset = Some(current_offset);
            }
        }
        remaining_bytes = &mut remaining_bytes[header.size as usize..];
        current_offset += header.size as usize;
    }
    result
}
