use core::fmt::Write;
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::boxed::Box;
use alloc::vec;
use common::font::get_character_dimensions;
use common::framebuffer::get_screen_dimensions;
use spin::LazyLock;

use crate::font_renderer;

pub fn get_console_dimensions() -> (usize, usize) {
    let screen_dimensions = get_screen_dimensions();
    let character_dimensions = get_character_dimensions();
    (
        screen_dimensions.0 / character_dimensions.0,
        screen_dimensions.1 / character_dimensions.1,
    )
}

static CONSOLE_BACKBUFFER: LazyLock<spin::Mutex<Box<[char]>>> = LazyLock::new(|| {
    let console_dimensions = get_console_dimensions();
    spin::Mutex::new(vec!['\0'; console_dimensions.0 * console_dimensions.1].into_boxed_slice())
});

static X: AtomicUsize = AtomicUsize::new(0);
static Y: AtomicUsize = AtomicUsize::new(0);

fn possibly_scroll(console_backbuffer: &mut [char]) {
    let console_dimensions = get_console_dimensions();
    let y = Y.load(Ordering::SeqCst);
    if y >= console_dimensions.1 {
        let mut lines = console_backbuffer
            .chunks_exact_mut(console_dimensions.0)
            .peekable();
        for row in 1..console_dimensions.1 {
            let destination = lines.next().unwrap();
            let source = lines.peek().unwrap();
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
            for (x, source_char) in source
                .iter()
                .enumerate()
                // Only really works because source is longer than it needs to be.
                .take(usize::max(source_line_length, old_destination_line_length))
            {
                font_renderer::draw_character(
                    if x < source_line_length {
                        *source_char
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
            console_backbuffer[(console_dimensions.1 - 1) * console_dimensions.0 + x] = ' ';
            font_renderer::draw_character(
                ' ',
                x * get_character_dimensions().0,
                (console_dimensions.1 - 1) * get_character_dimensions().1,
            );
        }
        Y.store(console_dimensions.1 - 1, Ordering::SeqCst);
        X.store(0, Ordering::SeqCst);
    }
}

pub fn write_character(character: char) {
    let mut console_backbuffer = CONSOLE_BACKBUFFER.lock();
    let x = X.load(Ordering::SeqCst);
    let y = Y.load(Ordering::SeqCst);
    console_backbuffer[y * get_console_dimensions().0 + x] = character;
    if character == '\n' {
        X.store(0, Ordering::SeqCst);
        Y.store(y + 1, Ordering::SeqCst);
        possibly_scroll(&mut console_backbuffer);
    } else {
        font_renderer::draw_character(
            character,
            x * get_character_dimensions().0,
            y * get_character_dimensions().1,
        );
        if x + 1 >= get_console_dimensions().0 {
            X.store(0, Ordering::SeqCst);
            Y.store(y + 1, Ordering::SeqCst);
            possibly_scroll(&mut console_backbuffer);
        } else {
            X.store(x + 1, Ordering::SeqCst);
        }
    }
}

pub fn write_string(string: &str) {
    for character in string.chars() {
        write_character(character);
    }
}

pub struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_string(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => (::core::fmt::Write::write_fmt(&mut $crate::console::ConsoleWriter, format_args!($($arg)*)).unwrap());
}

pub use print;

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($fmt:expr) => ($crate::console::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::console::print!(concat!($fmt, "\n"), $($arg)*));
}

pub use println;
