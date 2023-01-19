use alloc::boxed::Box;

use crate::lazy_init::lazy_static;

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

// We cache the rendered versions of the characters here since they will be redrawn rather a lot (especially during scrolling).

fn get_character_cache_offset(glyph_index: usize) -> usize {
    let font_header = get_font_header();
    unsafe {
        glyph_index
            * font_header.width as usize
            * font_header.height as usize
            * FRAME_BUFFER.bytes_per_pixel as usize
    }
}

lazy_static! {
    static ref CHARACTER_CACHE: Box<[u8]> = {
        let font_header = get_font_header();
        let mut result = unsafe {
            Box::new_zeroed_slice(get_character_cache_offset(font_header.glyph_count as usize))
                .assume_init()
        };
        for i in 0..font_header.glyph_count as u32 {
            unsafe {
                render_character(
                    char::from_u32_unchecked(i),
                    &mut result[get_character_cache_offset(i as usize)
                        ..get_character_cache_offset(i as usize + 1)],
                )
            };
        }
        result
    };
}

/// Renders the given character in the given buffer.
///
/// This function doesn't support the unicode table since it is only meant for kernel logging and besides, the character cache is too small for unicode.
fn render_character(character: char, buffer: &mut [u8]) {
    let font_header = get_font_header();
    let character = if (character as u32) < font_header.glyph_count {
        character
    } else {
        '\0'
    };
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
                (glyph_y as usize * font_header.width as usize + glyph_x as usize)
                    * FRAME_BUFFER.bytes_per_pixel as usize
            };
            unsafe {
                buffer[pixel_index + FRAME_BUFFER.red_byte as usize] = color;
                buffer[pixel_index + FRAME_BUFFER.green_byte as usize] = color;
                buffer[pixel_index + FRAME_BUFFER.blue_byte as usize] = color;
            }
        }
    }
}

pub fn get_character_dimensions() -> (usize, usize) {
    let font_header = get_font_header();
    (font_header.width as usize, font_header.height as usize)
}

pub fn draw_character(character: char, x: usize, y: usize) {
    let (character_width, character_height) = get_character_dimensions();
    let character = if (character as u32) < get_font_header().glyph_count {
        character
    } else {
        '\0'
    };
    let (screen_width, screen_height) = get_screen_dimensions();
    if x + character_width > screen_width || y + character_height > screen_height {
        return;
    }
    let font_header = get_font_header();
    let character_cache_offset = get_character_cache_offset(character as usize);
    let character_cache = unsafe { &CHARACTER_CACHE[character_cache_offset..] };
    for row in 0..character_height {
        let row_pixel_cache = unsafe {
            &character_cache[row * character_width * FRAME_BUFFER.bytes_per_pixel as usize
                ..(row + 1) * character_width * FRAME_BUFFER.bytes_per_pixel as usize]
        };
        let row_pixels = unsafe {
            &mut FRAME_BUFFER.pixels[(y + row) * FRAME_BUFFER.pitch as usize
                + x * FRAME_BUFFER.bytes_per_pixel as usize
                ..(y + row) * FRAME_BUFFER.pitch as usize
                    + (x + font_header.width as usize) * FRAME_BUFFER.bytes_per_pixel as usize]
        };
        row_pixels.copy_from_slice(row_pixel_cache);
    }
}
