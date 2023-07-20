use spin::Mutex;
use x86_64::instructions::port::Port;

use lazy_static::lazy_static;

use crate::print;

pub struct KeyboardDriver {
    port: Port<u8>,
}

impl KeyboardDriver {
    pub fn new() -> KeyboardDriver {
        KeyboardDriver {
            port: Port::new(0x60),
        }
    }

    #[inline]
    pub fn handle_interrupt(&mut self) {
        let scancode = unsafe { self.port.read() };
        print!("{:02X} ", scancode);
    }
}

lazy_static!{
    pub static ref KEYBOARD_DRIVER: Mutex<KeyboardDriver> = Mutex::new(KeyboardDriver::new());
}