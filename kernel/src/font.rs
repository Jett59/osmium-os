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
