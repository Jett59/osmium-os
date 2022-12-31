use crate::x86_64::vga;

static FONT: &'static [u8] = include_bytes!("../../font.psf");

pub fn clear() {
    vga::clear();
}
pub fn write_character(character: char) {
    vga::write_character(character);
}
pub fn write_string(string: &str) {
    vga::write_string(string);
}
