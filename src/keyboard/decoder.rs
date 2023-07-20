use alloc::{vec::Vec, boxed::Box};
use futures_util::{StreamExt, future::join_all};
use spin::Mutex;
use thingbuf::mpsc::{Sender, Receiver, self};

use crate::println;

use super::{keycodes::{KeyEvent, KeyCode}, driver::ScancodeStream};

enum ScancodeState {
    Idle,
    E0,
    F0,
    E0F0,
    
    PrintScreen(u8),
    PrintScreenRelease(u8),
    Pause(u8),
}

fn scancode_to_key(byte: u8) -> Option<KeyCode> {
    Some(match byte {
        0x1C => KeyCode::A,
        0x32 => KeyCode::B,
        0x21 => KeyCode::C,
        0x23 => KeyCode::D,
        0x24 => KeyCode::E,
        0x2B => KeyCode::F,
        0x34 => KeyCode::G,
        0x33 => KeyCode::H,
        0x43 => KeyCode::I,
        0x3B => KeyCode::J,
        0x42 => KeyCode::K,
        0x4B => KeyCode::L,
        0x3A => KeyCode::M,
        0x31 => KeyCode::N,
        0x44 => KeyCode::O,
        0x4D => KeyCode::P,
        0x15 => KeyCode::Q,
        0x2D => KeyCode::R,
        0x1B => KeyCode::S,
        0x2C => KeyCode::T,
        0x3C => KeyCode::U,
        0x2A => KeyCode::V,
        0x1D => KeyCode::W,
        0x22 => KeyCode::X,
        0x35 => KeyCode::Y,
        0x1A => KeyCode::Z,

        0x45 => KeyCode::Number0,
        0x16 => KeyCode::Number1,
        0x1E => KeyCode::Number2,
        0x26 => KeyCode::Number3,
        0x25 => KeyCode::Number4,
        0x2E => KeyCode::Number5,
        0x36 => KeyCode::Number6,
        0x3D => KeyCode::Number7,
        0x3E => KeyCode::Number8,
        0x46 => KeyCode::Number9,

        0x0E => KeyCode::BackTick,
        0x4E => KeyCode::Minus,
        0x55 => KeyCode::Equals,
        0x5D => KeyCode::BackSlash,
        0x66 => KeyCode::Backspace,
        0x29 => KeyCode::Space,
        0x0D => KeyCode::Tab,
        0x58 => KeyCode::CapsLock,

        0x12 => KeyCode::LeftShift,
        0x14 => KeyCode::LeftCtrl,
        0x11 => KeyCode::LeftAlt,
        0x59 => KeyCode::RightShift,
        0x5A => KeyCode::Enter,
        0x76 => KeyCode::Escape,

        0x05 => KeyCode::F1,
        0x06 => KeyCode::F2,
        0x04 => KeyCode::F3,
        0x0C => KeyCode::F4,
        0x03 => KeyCode::F5,
        0x0B => KeyCode::F6,
        0x83 => KeyCode::F7,
        0x0A => KeyCode::F8,
        0x01 => KeyCode::F9,
        0x09 => KeyCode::F10,
        0x78 => KeyCode::F11,
        0x07 => KeyCode::F12,

        0x7E => KeyCode::ScrollLock,

        0x54 => KeyCode::LeftBracket,
        0x5B => KeyCode::RightBracket,
        0x4C => KeyCode::Semicolon,
        0x52 => KeyCode::Apostrophe,
        0x41 => KeyCode::Comma,
        0x49 => KeyCode::Period,
        0x4A => KeyCode::Slash,

        0x77 => KeyCode::NumLock,
        0x7C => KeyCode::NumPadMul,
        0x7B => KeyCode::NumPadSub,
        0x79 => KeyCode::NumPadAdd,
        0x71 => KeyCode::NumPadPeriod,
        
        0x70 => KeyCode::NumPad0,
        0x69 => KeyCode::NumPad1,
        0x72 => KeyCode::NumPad2,
        0x7A => KeyCode::NumPad3,
        0x6B => KeyCode::NumPad4,
        0x73 => KeyCode::NumPad5,
        0x74 => KeyCode::NumPad6,
        0x6C => KeyCode::NumPad7,
        0x75 => KeyCode::NumPad8,
        0x7D => KeyCode::NumPad9,

        _ => return None,
    })
}

fn scancode_to_key_extended(byte: u8) -> Option<KeyCode> {
    Some(match byte {
        0x1F => KeyCode::LeftWin,
        0x14 => KeyCode::RightCtrl,
        0x27 => KeyCode::RightWin,
        0x11 => KeyCode::RightAlt,
        0x2F => KeyCode::Menu,
        
        0x70 => KeyCode::Insert,
        0x6C => KeyCode::Home,
        0x7D => KeyCode::PageUp,
        0x71 => KeyCode::Delete,
        0x69 => KeyCode::End,
        0x7A => KeyCode::PageDown,

        0x75 => KeyCode::UpArrow,
        0x6B => KeyCode::LeftArrow,
        0x72 => KeyCode::DownArrow,
        0x74 => KeyCode::RightArrow,

        0x4A => KeyCode::NumPadDiv,
        0x5A => KeyCode::NumPadEnter,

        _ => return None,
    })
}

impl ScancodeState {
    fn handle_idle(&mut self, scancode: u8) -> Option<KeyEvent> {
        if scancode == 0xF0 {
            *self = ScancodeState::F0;
            None
        } else if scancode == 0xE0 {
            *self = ScancodeState::E0;
            None
        } else if scancode == 0xE1 {
            *self = ScancodeState::Pause(1);
            None
        }else {
            scancode_to_key(scancode)
                .map(|key| KeyEvent::Pressed(key))
        }
    }
    fn handle_f0(&mut self, scancode: u8) -> Option<KeyEvent> {
        *self = ScancodeState::Idle;
        scancode_to_key(scancode)
            .map(|key| KeyEvent::Released(key))
    }
    fn handle_e0(&mut self, scancode: u8) -> Option<KeyEvent> {
        if scancode == 0xF0 {
            *self = ScancodeState::E0F0;
            None
        } else if scancode == 0x12 {
            *self = ScancodeState::PrintScreen(2);
            None
        } else {
            *self = ScancodeState::Idle;
            scancode_to_key_extended(scancode)
                .map(|key| KeyEvent::Pressed(key))
        }
    }
    fn handle_e0f0(&mut self, scancode: u8) -> Option<KeyEvent> {
        if scancode == 0x7C {
            *self = ScancodeState::PrintScreenRelease(3);
            None
        } else {
            *self = ScancodeState::Idle;
            scancode_to_key_extended(scancode)
                .map(|key| KeyEvent::Released(key))
        }
    }

    fn handle_print_screen(&mut self, scancode: u8, state: u8) -> Option<KeyEvent> {
        const PRINT_SCREEN_CODE: [u8; 4] = [0xE0, 0x12, 0xE0, 0x7C];
        assert!(state == 2 || state == 3);
        if scancode == PRINT_SCREEN_CODE[state as usize] {
            if state == 3 {
                *self = ScancodeState::Idle;
                Some(KeyEvent::Pressed(KeyCode::PrintScreen))
            } else {
                *self = ScancodeState::PrintScreen(state + 1);
                None
            }
        } else {
            *self = ScancodeState::Idle;
            None
        }
    }

    fn handle_print_screen_release(&mut self, scancode: u8, state: u8) -> Option<KeyEvent> {
        const PRINT_SCREEN_RELEASE_CODE: [u8; 6] = [0xE0, 0xF0, 0x7C, 0xE0, 0xF0, 0x12];
        assert!(state >= 3 && state < 6);
        if scancode == PRINT_SCREEN_RELEASE_CODE[state as usize] {
            if state == 5 {
                *self = ScancodeState::Idle;
                Some(KeyEvent::Released(KeyCode::PrintScreen))
            } else {
                *self = ScancodeState::PrintScreenRelease(state + 1);
                None
            }
        } else {
            *self = ScancodeState::Idle;
            None
        }
    }

    fn handle_pause(&mut self, scancode: u8, state: u8) -> Option<KeyEvent> {
        const PAUSE_CODE: [u8; 8] = [0xE1, 0x14, 0x77, 0xE1, 0xF0, 0x14, 0xF0, 0x77];
        assert!(state >= 1 && state < 8);
        if scancode == PAUSE_CODE[state as usize] {
            if state == 7 {
                *self = ScancodeState::Idle;
                Some(KeyEvent::Pressed(KeyCode::PauseBreak))
            } else {
                *self = ScancodeState::Pause(state + 1);
                None
            }
        } else {
            *self = ScancodeState::Idle;
            None
        }
    }

    fn handle(&mut self, scancode: u8) -> Option<KeyEvent> {
        match *self {
            ScancodeState::Idle => self.handle_idle(scancode),
            ScancodeState::E0 => self.handle_e0(scancode),
            ScancodeState::F0 => self.handle_f0(scancode),
            ScancodeState::E0F0 => self.handle_e0f0(scancode),
            ScancodeState::PrintScreen(state) => self.handle_print_screen(scancode, state),
            ScancodeState::PrintScreenRelease(state) => self.handle_print_screen_release(scancode, state),
            ScancodeState::Pause(state) => self.handle_pause(scancode, state),
        }
    }
}

static KEYEVENT_SENDERS: Mutex<Vec<Sender<KeyEvent>>> = Mutex::new(Vec::new());

pub async fn keycode_decoder(scancodes: &mut ScancodeStream) {
    let mut state = ScancodeState::Idle;
    while let Some(scancode) = scancodes.next().await {
        let key_event = state.handle(scancode);
        if let Some(key_event) = key_event {
            join_all(
                KEYEVENT_SENDERS.lock()
                    .iter()
                    .map(|s| s.send(key_event))
            ).await
                .into_iter()
                .for_each(|x| x.unwrap());
        }
    }
}

pub fn get_keyevent_receiver() -> Receiver<KeyEvent> {
    let (tx, rx) = mpsc::channel(32);
    KEYEVENT_SENDERS.lock().push(tx);
    rx
}