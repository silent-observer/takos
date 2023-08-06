use super::{keycodes::{ KeyCode, KeyState}, layout::{layout_upper_case, layout_lower_case}};

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ModifierKeys: u8 {
        const SHIFT = 0x01;
        const ALT = 0x02;
        const CTRL = 0x04;
        const WIN = 0x08;
        const CAPS_LOCK = 0x10;
        const NUM_LOCK = 0x20;
        const SCROLL_LOCK = 0x40;
        const UPPER_CASE = 0x80;
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct KeyboardEvent {
    pub c: Option<char>,
    pub key: KeyCode,
    pub state: KeyState,
    pub modifiers: ModifierKeys,
}

pub struct TyperState {
    left_shift: bool,
    left_alt: bool,
    left_ctrl: bool,
    left_win: bool,

    right_shift: bool,
    right_alt: bool,
    right_ctrl: bool,
    right_win: bool,

    caps_lock: bool,
    num_lock: bool,
    scroll_lock: bool,
}

impl TyperState {
    pub fn new() -> Self {
        Self {
            left_shift: false,
            left_alt: false,
            left_ctrl: false,
            left_win: false,
            right_shift: false,
            right_alt: false,
            right_ctrl: false,
            right_win: false,
            caps_lock: false,
            num_lock: false,
            scroll_lock: false,
        }
    }

    fn get_modifiers(&self) -> ModifierKeys {
        let mut modifiers = ModifierKeys::empty();
        if self.left_shift || self.right_shift {
            modifiers.insert(ModifierKeys::SHIFT | ModifierKeys::UPPER_CASE);
        }
        if self.left_alt || self.right_alt {
            modifiers.insert(ModifierKeys::ALT);
        }
        if self.left_ctrl || self.right_ctrl {
            modifiers.insert(ModifierKeys::CTRL);
        }
        if self.left_win || self.right_win {
            modifiers.insert(ModifierKeys::WIN);
        }
        if self.caps_lock {
            modifiers.insert(ModifierKeys::CAPS_LOCK);
            modifiers.toggle(ModifierKeys::UPPER_CASE);
        }
        if self.num_lock {
            modifiers.insert(ModifierKeys::NUM_LOCK);
        }
        if self.scroll_lock {
            modifiers.insert(ModifierKeys::SCROLL_LOCK);
        }
        modifiers
    }

    fn form_keyboard_event(&self, key: KeyCode, state: KeyState) -> KeyboardEvent {
        let modifiers = self.get_modifiers();

        let c = if modifiers.contains(ModifierKeys::UPPER_CASE) {
            layout_upper_case(key)
        } else {
            layout_lower_case(key)
        };

        KeyboardEvent {
            c,
            key,
            state,
            modifiers,
        }
    }

    pub fn handle(&mut self, key: KeyCode, state: KeyState) -> KeyboardEvent {
        let is_pressed = state == KeyState::Pressed;
        match key {
            KeyCode::LeftShift => self.left_shift = is_pressed,
            KeyCode::LeftAlt => self.left_alt = is_pressed,
            KeyCode::LeftCtrl => self.left_ctrl = is_pressed,
            KeyCode::LeftWin => self.left_win = is_pressed,

            KeyCode::RightShift => self.right_shift = is_pressed,
            KeyCode::RightAlt => self.right_alt = is_pressed,
            KeyCode::RightCtrl => self.right_ctrl = is_pressed,
            KeyCode::RightWin => self.right_win = is_pressed,

            KeyCode::CapsLock => if !is_pressed {self.caps_lock = !self.caps_lock},
            KeyCode::NumLock => if !is_pressed {self.num_lock = !self.num_lock},
            KeyCode::ScrollLock => if !is_pressed {self.scroll_lock = !self.scroll_lock},

            _ => {},
        }
        self.form_keyboard_event(key, state)
    }
}