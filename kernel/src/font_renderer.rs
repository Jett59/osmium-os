use crate::lazy_init::lazy_static;
use alloc::boxed::Box;
use common::font::{get_character_dimensions, get_glyph_count, render_character};
use common::framebuffer::{get_bytes_per_pixel, get_pixel_row, get_screen_dimensions, PixelFormat};

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
                    PixelFormat::default(),
                )
            };
        }
        result
    };
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
