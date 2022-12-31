struct VgaCell {
    character: u8,
    attribute: u8,
}

const VIDEO_MEMORY: *mut VgaCell = 0xb8000 as *mut VgaCell;

static mut X: usize = 0;
static mut Y: usize = 0;

pub fn clear() {
    unsafe {
        for i in 0..80 * 25 {
            let cell = VIDEO_MEMORY.offset(i as isize);
            (*cell).attribute = 0x07;
            (*cell).character = 0x20;
        }
        X = 0;
        Y = 0;
    }
}

pub fn write_character(character: char) {
    unsafe {
        if character == '\n' {
            X = 0;
            Y += 1;
            return;
        }
        let vga_cell = &mut *VIDEO_MEMORY.add(Y * 80 + X);
        vga_cell.character = character as u8;
        X += 1;
        if X == 80 {
            X = 0;
            Y += 1;
        }
    }
}

pub fn write_string(str: &str) {
    for c in str.chars() {
        write_character(c);
    }
}
