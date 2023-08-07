use core::fmt::Write;

use alloc::vec;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
    display::{ColorRGB, FrameBuffer},
    keyboard::{
        self,
        keycodes::{KeyCode, KeyState},
    },
};

const TEXT_BUFFER_SIZE: usize = 64;

pub struct ConsoleWriter {
    buffer: FrameBuffer,
    width: usize,
    height: usize,
    x: usize,
    fg_color: ColorRGB,
    bg_color: ColorRGB,

    text_buffer: Vec<u8>,
    text_buffer_height: usize,
    scroll_index: usize,
    scroll_count: usize,
    bottom_attached: bool,
}

impl ConsoleWriter {
    pub fn new(buffer: FrameBuffer) -> ConsoleWriter {
        let width = buffer.text_width();
        let height = buffer.text_height();
        ConsoleWriter {
            width,
            height,
            buffer,
            x: 0,
            fg_color: ColorRGB::from_hex(0xFFFFFF),
            bg_color: ColorRGB::from_hex(0x000000),

            text_buffer: vec![0x00; width * height * TEXT_BUFFER_SIZE],
            text_buffer_height: height * TEXT_BUFFER_SIZE,
            scroll_index: 0,
            scroll_count: 0,
            bottom_attached: true,
        }
    }

    pub fn frame_buffer(&mut self) -> &mut FrameBuffer {
        &mut self.buffer
    }

    fn write_newline(&mut self) {
        self.x = 0;
        self.scroll_count += 1;
        if self.bottom_attached {
            if self.scroll_count >= self.scroll_index + self.height {
                let offset = (self.scroll_count - self.height + 1) - self.scroll_index;
                self.scroll_index += offset;
                self.buffer.text_move_up(1, self.bg_color);
            } else {
                self.scroll_index = 0;
            }
        }
    }

    fn put_symbol(&mut self, byte: u8) {
        let y = self.scroll_count;
        let x = self.x;
        let text_buffer_index = (y % self.text_buffer_height) * self.width + x;
        self.text_buffer[text_buffer_index] = byte;
        if y >= self.scroll_index && y < self.scroll_index + self.height {
            let screen_y = y - self.scroll_index;
            self.buffer
                .put_symbol(x, screen_y, self.fg_color, self.bg_color, byte);
        }
    }

    fn write_symbol(&mut self, byte: u8) {
        self.put_symbol(byte);
        self.x += 1;
        if self.x >= self.width - 1 {
            self.put_symbol(0x10);
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

    fn scroll_up(&mut self) {
        let buffer_start = if self.scroll_count < self.text_buffer_height {
            0
        } else {
            self.scroll_count - self.text_buffer_height
        };
        if self.scroll_index > buffer_start {
            self.bottom_attached = false;
            self.scroll_index -= 1;
            self.buffer.text_move_down(1, self.bg_color);
            for x in 0..self.width {
                let text_buffer_index =
                    (self.scroll_index % self.text_buffer_height) * self.width + x;
                let byte = self.text_buffer[text_buffer_index];
                self.buffer
                    .put_symbol(x, 0, self.fg_color, self.bg_color, byte);
            }
        }
    }

    fn scroll_down(&mut self) {
        if !self.bottom_attached && self.scroll_index + self.height <= self.scroll_count {
            self.bottom_attached = self.scroll_index + self.height == self.scroll_count;
            self.scroll_index += 1;

            self.buffer.text_move_up(1, self.bg_color);
            let y = self.scroll_index + self.height - 1;
            for x in 0..self.width {
                let text_buffer_index = (y % self.text_buffer_height) * self.width + x;
                let byte = self.text_buffer[text_buffer_index];
                self.buffer
                    .put_symbol(x, self.height - 1, self.fg_color, self.bg_color, byte);
            }
        }
    }

    fn page_up(&mut self) {
        for _ in 0..self.height {
            self.scroll_up();
        }
    }

    fn page_down(&mut self) {
        for _ in 0..self.height {
            self.scroll_down();
        }
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<ConsoleWriter> =
        Mutex::new(ConsoleWriter::new(FrameBuffer::empty()));
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

pub async fn console_scroll_handler() {
    let keyboard_event_reciever = keyboard::get_keyboard_event_receiver();
    while let Some(event) = keyboard_event_reciever.recv().await {
        if event.state != KeyState::Pressed {
            continue;
        }
        match event.key {
            KeyCode::UpArrow | KeyCode::W => WRITER.lock().scroll_up(),
            KeyCode::DownArrow | KeyCode::S => WRITER.lock().scroll_down(),
            KeyCode::PageUp => WRITER.lock().page_up(),
            KeyCode::PageDown => WRITER.lock().page_down(),
            _ => {
                println!("{:?}", event);
            }
        }
    }
}
