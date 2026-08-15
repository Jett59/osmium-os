use core::ptr::null_mut;

pub struct FrameBuffer {
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
    pub bytes_per_pixel: u8,
    // The indices of the bytes for each color value.
    // If each color has more than 1 byte for its value, the supplied index should be the high order byte.
    // If they are not byte-aligned, tough luck.
    pub red_byte: u8,
    pub green_byte: u8,
    pub blue_byte: u8,
    pub pixels: *mut u8,
}

static mut FRAME_BUFFER: FrameBuffer = FrameBuffer {
    // By setting width and height to 0 we ensure that no-one will try to write to the framebuffer until it is initialized properly.
    width: 0,
    height: 0,
    pitch: 0,
    bytes_per_pixel: 0,
    red_byte: 0,
    green_byte: 0,
    blue_byte: 0,
    pixels: null_mut(),
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

pub fn get_screen_dimensions() -> (usize, usize) {
    unsafe { (FRAME_BUFFER.width, FRAME_BUFFER.height) }
}

pub fn get_bytes_per_pixel() -> usize {
    unsafe { FRAME_BUFFER.bytes_per_pixel as usize }
}

pub fn get_rgb_byte_positions() -> (u8, u8, u8) {
    unsafe {
        (
            FRAME_BUFFER.red_byte,
            FRAME_BUFFER.green_byte,
            FRAME_BUFFER.blue_byte,
        )
    }
}

pub fn get_pixel_row(x: usize, y: usize) -> *mut u8 {
    unsafe {
        FRAME_BUFFER
            .pixels
            .add(y * FRAME_BUFFER.pitch + x * FRAME_BUFFER.bytes_per_pixel as usize)
    }
}

pub fn write_pixel_row(x: usize, y: usize, row: &[u8]) {
    assert!(x + row.len() / get_bytes_per_pixel() <= unsafe { FRAME_BUFFER.width });
    let row_pixels = get_pixel_row(x, y);
    unsafe {
        row_pixels.copy_from_nonoverlapping(row.as_ptr(), row.len());
    }
}

pub struct PixelFormat {
    pub bytes_per_pixel: u8,
    pub red_byte: u8,
    pub green_byte: u8,
    pub blue_byte: u8,
}

impl Default for PixelFormat {
    fn default() -> Self {
        let (red_byte, green_byte, blue_byte) = get_rgb_byte_positions();
        Self {
            bytes_per_pixel: get_bytes_per_pixel() as u8,
            red_byte,
            green_byte,
            blue_byte,
        }
    }
}
