use super::keycodes::{KeyEvent, KeyCode};

enum ScancodeState {
    Idle,
    E0,
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
        0x5C => KeyCode::BackSlash,
        0x66 => KeyCode::Backspace,
        0x29 => KeyCode::Space,
        0x0D => KeyCode::Tab,
        0x14 => KeyCode::CapsLock,

        0x12 => KeyCode::LeftShift,
        0x11 => KeyCode::LeftCtrl,
        0x8B => KeyCode::LeftWin,
        0x5C => KeyCode::LeftAlt,
        0x66 => KeyCode::RightShift,
        0x29 => KeyCode::RightCtrl,
        0x0D => KeyCode::RightWin,
        0x14 => KeyCode::RightAlt,

        _ => return None,
    })
}

impl ScancodeState {
    fn handle_idle(&mut self, scancode: u8) -> Option<KeyEvent> {
        scancode_to_key(scancode)
            .map(|key| KeyEvent::Pressed(key))
    }
}