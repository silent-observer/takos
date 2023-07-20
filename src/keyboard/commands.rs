use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;

use crate::println;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum Command {
    SetLed {
        scroll_lock: bool,
        num_lock: bool,
        caps_lock: bool,
    },
    GetScanCodeSet,
    SetScanCodeSet(u8),
}

pub struct CommandHandler {
    last_command: Option<Command>,
    command_state: u8,
}

impl CommandHandler {
    pub fn new() -> Self {
        Self { 
            last_command: None,
            command_state: 0,
        }
    }

    pub fn send_command(&mut self, command: Command) {
        if self.last_command.is_some() {
            println!("WARNING: PS/2 command lost")
        } else {
            self.last_command = Some(command);
            let mut port = Port::<u8>::new(0x60);
            match self.last_command.as_ref().unwrap() {
                &Command::SetLed { scroll_lock, num_lock, caps_lock } => unsafe {
                        port.write(0xED);
                        // let flags: u8 = if scroll_lock { 0x01 } else { 0x00 } |
                        //     if num_lock { 0x02 } else { 0x00 } |
                        //     if caps_lock { 0x04 } else { 0x00 };
                        // port.write(flags);
                        self.command_state = 0;
                },
                &Command::GetScanCodeSet => unsafe {
                    port.write(0xF0);
                    //port.write(0x00);
                    self.command_state = 0;
                },
                &Command::SetScanCodeSet(set) => unsafe {
                    assert!(1 <= set && set <= 3);
                    port.write(0xF0);
                    //port.write(set);
                    self.command_state = 0;
                },
            }
        }
    }

    pub fn handle_scancode(&mut self, scancode: u8) -> bool {
        if let Some(command) = self.last_command {
            if scancode == 0xFE {
                self.last_command = None;
                self.send_command(command);
                return true
            }
            else {
                let mut port = Port::<u8>::new(0x60);
                match command {
                    Command::SetLed { scroll_lock, num_lock, caps_lock } => {
                        if self.command_state == 0 && scancode == 0xFA {
                            let flags: u8 = if scroll_lock { 0x01 } else { 0x00 } |
                                if num_lock { 0x02 } else { 0x00 } |
                                if caps_lock { 0x04 } else { 0x00 };
                            unsafe{ port.write(flags) };
                            self.command_state = 1;
                            return true;
                        } else if self.command_state == 1 && scancode == 0xFA {
                            self.last_command = None;
                            return true;
                        }
                    }
                    Command::SetScanCodeSet(set) => {
                        if self.command_state == 0 && scancode == 0xFA {
                            unsafe{ port.write(set) };
                            self.command_state = 1;
                            return true;
                        } else if self.command_state == 1 && scancode == 0xFA {
                            self.last_command = None;
                            self.send_command(Command::GetScanCodeSet);
                            return true;
                        }
                    }
                    Command::GetScanCodeSet => {
                        if self.command_state == 0 && scancode == 0xFA {
                            unsafe{ port.write(0x00) };
                            self.command_state = 1;
                            return true;
                        } else if self.command_state == 1 && scancode == 0xFA {
                            self.command_state = 2;
                            return true;
                        } else if self.command_state == 2 {
                            self.last_command = None;
                            println!("Scan code set is {:02X}", scancode);
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }
}

lazy_static! {
    pub static ref COMMAND_HANDLER: Mutex<CommandHandler> = Mutex::new(CommandHandler::new());
}