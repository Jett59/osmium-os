use crate::framebuffer::PixelFormat;

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

pub fn get_character_dimensions() -> (usize, usize) {
    let font_header = get_font_header();
    (font_header.width as usize, font_header.height as usize)
}

pub fn get_glyph_count() -> usize {
    let font_header = get_font_header();
    font_header.glyph_count as usize
}

pub fn get_glyph_bitmap(character: char) -> &'static [u8] {
    let font_header = get_font_header();
    let character = if (character as u32) < font_header.glyph_count {
        character
    } else {
        '\0'
    };
    &FONT[font_header.header_size as usize
        + character as usize * font_header.character_bytes as usize
        ..font_header.header_size as usize
            + (character as usize + 1) * font_header.character_bytes as usize]
}

/// Renders the given character in the given buffer.
///
/// This function doesn't support the unicode table since it is only meant for kernel logging and besides, the character cache is too small for unicode.
pub fn render_character(character: char, buffer: &mut [u8], pixel_format: PixelFormat) {
    let glyph_bytes = get_glyph_bitmap(character);
    let (character_width, character_height) = get_character_dimensions();
    let bytes_per_row = (character_width + 7) / 8;
    let PixelFormat {
        bytes_per_pixel,
        red_byte,
        green_byte,
        blue_byte,
    } = pixel_format;
    for glyph_y in 0..character_height {
        for glyph_x in 0..character_width {
            let byte = glyph_bytes[glyph_y * bytes_per_row + glyph_x / 8];
            // Since the character is stored most-significant-bit-first, we have to do this little bit of math to convert our index into a bit index.
            let bit_index = 7 - glyph_x % 8;
            let bit = byte & (1 << bit_index) != 0;
            // We will just fill the buffer with a uniform color (all color bytes the same).
            // This is a bit limiting, but I think it is ok.
            let color = if bit { 0xFF } else { 0x00 };
            let pixel_index = (glyph_y * character_width + glyph_x) * bytes_per_pixel as usize;
            buffer[pixel_index + red_byte as usize] = color;
            buffer[pixel_index + green_byte as usize] = color;
            buffer[pixel_index + blue_byte as usize] = color;
        }
    }
}
