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
    pub pixels: &'static mut [u8],
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
    pixels: &mut [],
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

static FONT: &[u8] = include_bytes!("font.psf");

#[repr(C)]
struct PsfHeader {
    _magic: u32,
    _version: u32,
    header_size: u32,
    _flags: u32,
    glyph_count: u32,
    character_bytes: u32,
    height: u32,
    width: u32,
}

fn get_font_header() -> &'static PsfHeader {
    unsafe { &*(FONT.as_ptr() as *const PsfHeader) }
}

/// Draws the given character on the screen at the given position (in pixels).
///
/// This function doesn't support unicode, which is a deliberate design decision as using it would needlessly complicate this function, which is only designed for kernel logging anyway.
pub fn draw_character(character: char, x: usize, y: usize) {
    let font_header = get_font_header();
    let character = if (character as u32) < font_header.glyph_count {
        character
    } else {
        '\0'
    };
    unsafe {
        if FRAME_BUFFER.width < x + font_header.width as usize
            || FRAME_BUFFER.height < y + font_header.height as usize
        {
            return;
        }
    }
    let top_left_pixel =
        unsafe { y * FRAME_BUFFER.pitch + x * FRAME_BUFFER.bytes_per_pixel as usize };
    let bytes_per_row = (font_header.width + 7) / 8;
    for glyph_y in 0..font_header.height {
        for glyph_x in 0..font_header.width {
            let byte_index = (glyph_y * bytes_per_row + glyph_x / 8) as usize
                + character as usize * font_header.character_bytes as usize;
            let byte = FONT[font_header.header_size as usize + byte_index];
            // Since the character is stored most-significant-bit-first, we have to do this little bit of math to convert our index into a bit index.
            let bit_index = 7 - glyph_x % 8;
            let bit = byte & (1 << bit_index) != 0;
            // We will just fill the framebuffer with a uniform color (all color bytes the same).
            // This is a bit limiting, but I think it is ok.
            let color = if bit { 0xFF } else { 0x00 };
            let pixel_index = unsafe {
                top_left_pixel
                    + glyph_y as usize * FRAME_BUFFER.pitch
                    + glyph_x as usize * FRAME_BUFFER.bytes_per_pixel as usize
            };
            unsafe {
                FRAME_BUFFER.pixels[pixel_index + FRAME_BUFFER.red_byte as usize] = color;
                FRAME_BUFFER.pixels[pixel_index + FRAME_BUFFER.green_byte as usize] = color;
                FRAME_BUFFER.pixels[pixel_index + FRAME_BUFFER.blue_byte as usize] = color;
            }
        }
    }
}

pub fn get_character_dimensions() -> (usize, usize) {
    let font_header = get_font_header();
    (font_header.width as usize, font_header.height as usize)
}
