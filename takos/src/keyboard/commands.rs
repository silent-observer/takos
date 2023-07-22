use futures::StreamExt;
use x86_64::instructions::port::Port;

use super::driver::ScancodeStream;

#[derive(Debug, Clone, Copy)]
pub struct LedState {
    scroll_lock: bool,
    num_lock: bool,
    caps_lock: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CommandError;

async fn should_resend(scancodes: &mut ScancodeStream) -> bool {
    for _ in 0..3 {
        let scancode = scancodes.next().await.unwrap();
        match scancode {
            0xFE => return true,
            0xFA => return false,
            _ => {}
        }
    }
    true
}


pub async fn set_led(scancodes: &mut ScancodeStream, led: LedState) -> Result<(), CommandError> {
    let mut port = Port::<u8>::new(0x60);
    for _ in 0..3 {
        unsafe{ port.write(0xED) };
        if should_resend(scancodes).await { continue; }
        let flags: u8 = if led.scroll_lock { 0x01 } else { 0x00 } |
            if led.num_lock { 0x02 } else { 0x00 } |
            if led.caps_lock { 0x04 } else { 0x00 };
        unsafe{ port.write(flags) };
        if should_resend(scancodes).await { continue; }
        return Ok(());
    }
    Err(CommandError)
}

pub async fn set_scancode_set(scancodes: &mut ScancodeStream, set: u8) -> Result<(), CommandError> {
    assert!(1 <= set && set <= 3);
    let mut port = Port::<u8>::new(0x60);
    for _ in 0..3 {
        unsafe{ port.write(0xF0) };
        if should_resend(scancodes).await { continue; }
        unsafe{ port.write(set) };
        if should_resend(scancodes).await { continue; }
        return Ok(());
    }
    Err(CommandError)
}

pub async fn get_scancode_set(scancodes: &mut ScancodeStream) -> Result<u8, CommandError> {
    let mut port = Port::<u8>::new(0x60);
    for _ in 0..3 {
        unsafe{ port.write(0xF0) };
        if should_resend(scancodes).await { continue; }
        unsafe{ port.write(0x00) };
        if should_resend(scancodes).await { continue; }
        let set_scancode = scancodes.next().await.unwrap();
        return Ok(set_scancode);
    }
    Err(CommandError)
}