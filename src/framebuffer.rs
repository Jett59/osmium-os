pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bytes_per_pixel: u32,
    // The indices of the bytes for each color value.
    // If each color has more than 1 byte for its value, the supplied index should be the high order byte.
    // If they are not byte-aligned, tough luck.
    pub red_byte: u8,
    pub green_byte: u8,
    pub blue_byte: u8,
    pub pixels: &'static [u8],
}

static mut FRAME_BUFFER: FrameBuffer = FrameBuffer {
    // By setting width and height to 0 we ensure that no-one will ever try to write to the framebuffer.
    width: 0,
    height: 0,
    pitch: 0,
    bytes_per_pixel: 0,
    red_byte: 0,
    green_byte: 0,
    blue_byte: 0,
    pixels: &[],
};

pub fn init(frame_buffer: FrameBuffer) {
    unsafe {
        assert!(
            FRAME_BUFFER.bytes_per_pixel == 0,
            "FrameBuffer already initialized"
        );
        FRAME_BUFFER = frame_buffer;
    }
}
