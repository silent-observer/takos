use core::fmt::Write;

use spin::Mutex;
use lazy_static::lazy_static;

use crate::display::{FrameBuffer, ColorRGB};

pub struct ConsoleWriter {
    buffer: FrameBuffer,
    width: usize,
    height: usize,
    x: usize,
    fg_color: ColorRGB,
    bg_color: ColorRGB,
}

impl ConsoleWriter {
    pub fn new(buffer: FrameBuffer) -> ConsoleWriter {
        ConsoleWriter {
            width: buffer.text_width(),
            height: buffer.text_height(),
            buffer,
            x: 0,
            fg_color: ColorRGB::from_hex(0xFFFFFF),
            bg_color: ColorRGB::from_hex(0x000000),
        }
    }

    pub fn frame_buffer(&mut self) -> &mut FrameBuffer {
        &mut self.buffer
    }

    fn write_newline(&mut self) {
        self.buffer.text_scroll_up(1, self.bg_color);
        self.x = 0;
    }

    fn write_symbol(&mut self, byte: u8) {
        let y = self.height - 1;
        self.buffer.put_symbol(self.x, y, self.fg_color, self.bg_color, byte);
        self.x += 1;
        if self.x >= self.width - 1 {
            self.buffer.put_symbol(self.x, y, self.fg_color, self.bg_color, 0x10);
            self.write_newline();
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.write_newline(),
            0x20..=0x7E => self.write_symbol(byte),
            _ => self.write_symbol(0xA8),
        }
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<ConsoleWriter> = Mutex::new(ConsoleWriter::new(FrameBuffer::empty()));
}

pub fn init_writer(frame_buffer: FrameBuffer) {
    *WRITER.lock() = ConsoleWriter::new(frame_buffer);
}

impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::console::_print(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {{
        $crate::print!("{}\n", format_args!($($arg)*));
    }};
}