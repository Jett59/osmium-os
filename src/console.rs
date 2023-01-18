use alloc::boxed::Box;

use crate::{
    framebuffer::{self, get_character_dimensions, get_screen_dimensions},
    lazy_init::lazy_static,
};

pub fn get_console_dimensions() -> (usize, usize) {
    let screen_dimensions = get_screen_dimensions();
    let character_dimensions = get_character_dimensions();
    (
        screen_dimensions.0 / character_dimensions.0,
        screen_dimensions.1 / character_dimensions.1,
    )
}

lazy_static! {
    static ref CONSOLE_BACKBUFFER: Box<[char]> = {
        let console_dimensions = get_console_dimensions();
        unsafe { Box::new_zeroed_slice(console_dimensions.0 * console_dimensions.1).assume_init() }
    };
}

static mut X: usize = 0;
static mut Y: usize = 0;

fn possibly_scroll() {
    unsafe {
        let console_dimensions = get_console_dimensions();
        if Y >= console_dimensions.1 {
            for row in 1..console_dimensions.1 {
                let source = &CONSOLE_BACKBUFFER
                    [row * console_dimensions.0..(row + 1) * console_dimensions.0];
                let destination = &mut CONSOLE_BACKBUFFER
                    [(row - 1) * console_dimensions.0..row * console_dimensions.0];
                let source_line_length = source
                    .iter()
                    .take_while(|&&character| character != '\n')
                    .count();
                let old_destination_line_length = destination
                    .iter()
                    .take_while(|&&character| character != '\n')
                    .count();
                destination[..source_line_length].copy_from_slice(&source[..source_line_length]);
                // Only insert the \n if the line isn't full.
                if source_line_length < console_dimensions.0 {
                    destination[source_line_length] = '\n';
                }
                for x in 0..usize::max(old_destination_line_length, source_line_length) {
                    framebuffer::draw_character(
                        if x < source_line_length {
                            source[x]
                        } else {
                            ' '
                        },
                        x * get_character_dimensions().0,
                        (row - 1) * get_character_dimensions().1,
                    );
                }
            }
            // Now clear out the last row.
            for x in 0..console_dimensions.0 {
                CONSOLE_BACKBUFFER[(console_dimensions.1 - 1) * console_dimensions.0 + x] = ' ';
                framebuffer::draw_character(
                    ' ',
                    x * get_character_dimensions().0,
                    (console_dimensions.1 - 1) * get_character_dimensions().1,
                );
            }
            Y -= 1;
            X = 0;
        }
    }
}

pub fn write_character(character: char) {
    let x = unsafe { X };
    let y = unsafe { Y };
    unsafe {
        CONSOLE_BACKBUFFER[x + y * get_console_dimensions().0] = character;
    }
    if character == '\n' {
        unsafe {
            X = 0;
            Y += 1;
            possibly_scroll();
        }
    } else {
        framebuffer::draw_character(
            character,
            x * get_character_dimensions().0,
            y * get_character_dimensions().1,
        );
        unsafe {
            X += 1;
            if X >= get_console_dimensions().0 {
                X = 0;
                Y += 1;
                possibly_scroll();
            }
        }
    }
}

pub fn write_string(string: &str) {
    for character in string.chars() {
        write_character(character);
    }
}
