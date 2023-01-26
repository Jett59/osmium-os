#[repr(u32)]
pub enum BootRequestTagType {
    StackPointer = 0,
    MemoryMap = 1,
    FrameBuffer = 2,
}
