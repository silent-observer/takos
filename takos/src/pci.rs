use core::fmt::Display;

use alloc::vec::Vec;
use alloc::vec;
use log::{info, error};
use spin::Mutex;
use x86_64::instructions::port::Port;

use crate::println;

#[derive(Debug, Copy, Clone)]
struct PciDeviceHandle {
    bus_number: u8,
    device_number: Option<u8>,
    function_number: Option<u8>,
}

#[derive(Debug, Copy, Clone)]
pub struct PciDeviceData {
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass_code: u8,
    pub prog_if: u8,
    pub revision_id: u8,
}

#[derive(Debug, Copy, Clone)]
pub enum BaseAddressRegister {
    Memory(u64),
    Io(u64),
}

#[derive(Debug, Clone)]
pub struct Header0 {
    pub bars: [BaseAddressRegister; 6]
}
#[derive(Debug, Clone)]
pub struct Header1 {}
#[derive(Debug, Clone)]
pub struct Header2 {}

#[derive(Debug, Clone)]
pub enum Header {
    Header0(Header0),
    Header1(Header1),
    Header2(Header2),
    Unknown(u8)
}

#[derive(Debug, Clone)]
pub struct PciDevice {
    handle: PciDeviceHandle,
    pub data: PciDeviceData,
    is_multifunction: bool,
    pub header: Header
}

impl PciDeviceHandle {
    pub fn new_bus(bus_number: u8) -> Self {
        Self {
            bus_number,
            device_number: None,
            function_number: None,
        }
    }
    pub fn new(bus_number: u8, device_number: u8, function_number: u8) -> Self {
        Self {
            bus_number,
            device_number: Some(device_number),
            function_number: Some(function_number),
        }
    }

    fn with_function(&self, function_number: u8) -> Self {
        Self {
            function_number: Some(function_number),
            ..*self
        }
    }

    fn with_device(&self, device_number: u8) -> Self {
        Self {
            device_number: Some(device_number),
            ..*self
        }
    }

    fn config_read(&self, address: u8) -> u32 {
        assert!(address & 0x3 == 0);
        let mut address_port = Port::<u32>::new(0xCF8);
        let mut data_port = Port::<u32>::new(0xCFC);
        let address_value = 0x80000000 |
            (self.bus_number as u32) << 16 |
            (self.device_number.unwrap_or(0) as u32) << 11 |
            (self.function_number.unwrap_or(0) as u32) << 8 |
            (address as u32);
        unsafe {
            address_port.write(address_value);
            data_port.read()
        }
    }

    fn header_type(&self) -> u8 {
        (self.config_read(0x0C) >> 16) as u8
    }

    pub fn exists(&self) -> bool {
        self.config_read(0x00) as u16 != 0xFFFF
    }

    fn get_device_data(&self) -> PciDeviceData {
        let reg0 = self.config_read(0x00);
        let reg2 = self.config_read(0x08);
        PciDeviceData {
            vendor_id: reg0 as u16,
            device_id: (reg0 >> 16) as u16,
            class_code: (reg2 >> 24) as u8,
            subclass_code: (reg2 >> 16) as u8,
            prog_if: (reg2 >> 8) as u8,
            revision_id: reg2 as u8,
        }
    }

    fn get_header(&self) -> (bool, Header) {
        let header_type = self.header_type();
        let is_multifunction = header_type & 0x80 != 0;
        let header_type = header_type & 0x7F;
        let header = match header_type {
            0x00 => {
                let mut bars = [BaseAddressRegister::Memory(0); 6];
                let mut i = 0;
                while i < 6 {
                    let bar_addr = 0x10 + i * 4;
                    let bar_value = self.config_read(bar_addr);
                    if bar_value & 0x1 == 0 {
                        let bar_type = (bar_value >> 1 & 0x3) as u8;
                        if bar_type == 2 {
                            let next_bar_value = self.config_read(bar_addr + 4) as u64;
                            let addr = (bar_value & !0xF) as u64 | next_bar_value << 32;
                            bars[i as usize] = BaseAddressRegister::Memory(addr);
                            i += 1;
                        } else {
                            bars[i as usize] = BaseAddressRegister::Memory((bar_value & !0xF).into());
                        }
                    } else {
                        bars[i as usize] = BaseAddressRegister::Io((bar_value & !0x3).into());
                    }

                    i += 1;
                }
                Header::Header0(Header0 {bars})
            },
            0x01 => Header::Header1(Header1 {}),
            0x02 => Header::Header2(Header2 {}),
            _ => {
                error!("Unknown header type: {:02X}", header_type);
                Header::Unknown(header_type)
            }
        };
        (is_multifunction, header)
    }

    fn enumerate_function(&self) -> Vec<PciDevice> {
        assert!(self.device_number.is_some());
        assert!(self.function_number.is_some());
        if !self.exists() {
            return Vec::new();
        }

        let data = self.get_device_data();
        let is_bus = data.class_code == 0x06 && data.subclass_code == 0x04;
        let (is_multifunction, header) = self.get_header();
        let mut result = vec![PciDevice {
            handle: *self,
            data,
            is_multifunction,
            header,
        }];

        if is_bus {
            let secondary_bus_number = (self.config_read(0x18) >> 8) as u8;
            let secondary_bus = PciDeviceHandle::new_bus(secondary_bus_number);
            result.append(&mut secondary_bus.enumerate_bus());
        }
        result
    }

    fn enumerate_device(&self) -> Vec<PciDevice> {
        assert!(self.device_number.is_some());
        assert!(self.function_number.is_none());
        if !self.exists() {
            return Vec::new();
        }

        let mut devices = Vec::new();
        let mut new_devices = self.with_function(0).enumerate_function();
        let is_multifunction = new_devices[0].is_multifunction;
        devices.append(&mut new_devices);

        if is_multifunction {
            for function_number in 1..8 {
                devices.append(&mut self.with_function(function_number).enumerate_function());
            }
        }
        devices
    }

    fn enumerate_bus(&self) -> Vec<PciDevice> {
        assert!(self.device_number.is_none());
        assert!(self.function_number.is_none());
        let mut result = Vec::new();
        for device_number in 0..32 {
            result.append(&mut self.with_device(device_number).enumerate_device());
        }
        result
    }
}

fn enumerate_all() -> Vec<PciDevice> {
    let main_bus = PciDeviceHandle::new_bus(0);
    let header_type = main_bus.header_type();
    if header_type & 0x80 == 0 {
        main_bus.enumerate_bus()
    } else {
        let mut result = Vec::new();
        for bus_number in 0..8 {
            let bus = PciDeviceHandle::new_bus(bus_number);
            if !bus.exists() {continue;}
            result.append(&mut bus.enumerate_bus());
        }
        result
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Header::Header0(header0) => {
                write!(f, "Header0{{ ")?;
                for (i, bar) in header0.bars.iter().enumerate() {
                    match bar {
                        BaseAddressRegister::Memory(addr) => 
                            if *addr != 0 {
                                write!(f, "bar{} mem 0x{:08X} ", i, *addr)?
                            },
                        BaseAddressRegister::Io(addr) => write!(f, "bar{} io 0x{:08X} ", i, *addr)?,
                    }
                }
                write!(f, "}}")
            }
            Header::Header1(_) => write!(f, "Header1"),
            Header::Header2(_) => write!(f, "Header2"),
            Header::Unknown(x) => write!(f, "Unknown({:02X})", x),
            
        }
        
    }
}

impl Display for PciDevice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PCI({:02X}:{:02X}.{:01X}, {:04X}-{:04X}, Class {:02X}-{:02X}-{:02X}, {})", 
            self.handle.bus_number,
            self.handle.device_number.unwrap_or(0),
            self.handle.function_number.unwrap_or(0),
            self.data.vendor_id,
            self.data.device_id,
            self.data.class_code,
            self.data.subclass_code,
            self.data.prog_if,
            self.header,
        )
    }
}

pub static PCI_DEVICES: Mutex<Vec<PciDevice>> = Mutex::new(Vec::new());

pub fn init_pci() {
    *PCI_DEVICES.lock() = enumerate_all();
    for device in PCI_DEVICES.lock().iter() {
        info!("Found device {}", device);
    }
}