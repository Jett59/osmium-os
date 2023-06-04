use alloc::boxed::Box;
use common::font::{get_character_dimensions, get_glyph_bitmap, get_glyph_count};

use crate::{
    framebuffer::{
        get_bytes_per_pixel, get_pixel_row, get_rgb_byte_positions, get_screen_dimensions,
    },
    lazy_init::lazy_static,
};

// We cache the rendered versions of the characters here since they will be redrawn rather a lot (especially during scrolling).
fn get_character_cache_offset(glyph_index: usize) -> usize {
    let (character_width, character_height) = get_character_dimensions();
    glyph_index * character_width * character_height * get_bytes_per_pixel()
}

lazy_static! {
    static ref CHARACTER_CACHE: Box<[u8]> = {
        let glyph_count = get_glyph_count();
        let mut result =
            unsafe { Box::new_zeroed_slice(get_character_cache_offset(glyph_count)).assume_init() };
        for i in 0..glyph_count {
            unsafe {
                render_character(
                    char::from_u32_unchecked(i as u32),
                    &mut result[get_character_cache_offset(i)..get_character_cache_offset(i + 1)],
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
    let glyph_bytes = get_glyph_bitmap(character);
    let (character_width, character_height) = get_character_dimensions();
    let bytes_per_row = (character_width + 7) / 8;
    let bytes_per_pixel = get_bytes_per_pixel();
    let (red_byte, green_byte, blue_byte) = get_rgb_byte_positions();
    for glyph_y in 0..character_height {
        for glyph_x in 0..character_width {
            let byte = glyph_bytes[glyph_y * bytes_per_row + glyph_x / 8];
            // Since the character is stored most-significant-bit-first, we have to do this little bit of math to convert our index into a bit index.
            let bit_index = 7 - glyph_x % 8;
            let bit = byte & (1 << bit_index) != 0;
            // We will just fill the buffer with a uniform color (all color bytes the same).
            // This is a bit limiting, but I think it is ok.
            let color = if bit { 0xFF } else { 0x00 };
            let pixel_index = (glyph_y * character_width + glyph_x) * bytes_per_pixel;
            buffer[pixel_index + red_byte as usize] = color;
            buffer[pixel_index + green_byte as usize] = color;
            buffer[pixel_index + blue_byte as usize] = color;
        }
    }
}

pub fn draw_character(character: char, x: usize, y: usize) {
    let (character_width, character_height) = get_character_dimensions();
    let (screen_width, screen_height) = get_screen_dimensions();
    if x + character_width > screen_width || y + character_height > screen_height {
        return;
    }
    let mut character_cache_offset = get_character_cache_offset(character as usize);
    if character_cache_offset >= unsafe { CHARACTER_CACHE.len() } {
        character_cache_offset = get_character_cache_offset(0);
    }
    let character_cache = unsafe { &CHARACTER_CACHE[character_cache_offset..] };
    let bytes_per_pixel = get_bytes_per_pixel();
    for row in 0..character_height {
        let row_pixel_cache = &character_cache[row * character_width * bytes_per_pixel
            ..(row + 1) * character_width * bytes_per_pixel];
        let row_pixels = get_pixel_row(x, y + row, character_width);
        row_pixels.copy_from_slice(row_pixel_cache);
    }
}
