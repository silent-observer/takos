use spin::Mutex;
use x86_64::instructions::port::PortWriteOnly;
use lazy_static::lazy_static;

const MASTER_PIC_COMMAND_PORT: u16 = 0x20;
const MASTER_PIC_DATA_PORT: u16 = 0x21;
const SLAVE_PIC_COMMAND_PORT: u16 = 0xA0;
const SLAVE_PIC_DATA_PORT: u16 = 0xA1;
pub const MASTER_PIC_OFFSET: u8 = 32;
pub const SLAVE_PIC_OFFSET: u8 = MASTER_PIC_OFFSET + 8;

pub struct PicChain {
    master_command: PortWriteOnly<u8>,
    master_data: PortWriteOnly<u8>,
    slave_command: PortWriteOnly<u8>,
    slave_data: PortWriteOnly<u8>,
}

impl PicChain {
    const fn new() -> Self {
        PicChain {
            master_command: PortWriteOnly::new(MASTER_PIC_COMMAND_PORT),
            master_data: PortWriteOnly::new(MASTER_PIC_DATA_PORT),
            slave_command: PortWriteOnly::new(SLAVE_PIC_COMMAND_PORT),
            slave_data: PortWriteOnly::new(SLAVE_PIC_DATA_PORT),
        }
    }

    fn init(&mut self) {
        unsafe {
            self.master_command.write(0x11);
            self.slave_command.write(0x11);
            self.master_data.write(MASTER_PIC_OFFSET);
            self.slave_data.write(SLAVE_PIC_OFFSET);
            self.master_data.write(4);
            self.slave_data.write(2);
            self.master_data.write(1);
            self.slave_data.write(1);
        }
    }

    pub fn notify_end_of_interrupt(&mut self, irq: u8) {
        unsafe {
            if irq >= SLAVE_PIC_OFFSET {
                self.slave_command.write(0x20);
            }
            self.master_command.write(0x20);
        }
    }
}

pub static PICS: Mutex<PicChain> = Mutex::new(PicChain::new());

pub fn init_pics() {
    PICS.lock().init();
}