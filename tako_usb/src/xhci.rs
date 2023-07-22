mod registers;
mod contexts;
pub mod trb;

use core::pin::Pin;

use alloc::{boxed::Box, vec::Vec};
use spin::Mutex;
use x86_64::VirtAddr;
use x86_64::structures::paging::Translate;

use crate::controller::UsbController;

use self::contexts::{DeviceContext, DeviceContextBaseAddressArray};
use self::trb::{TrbRing, EventRing, Trb};
use self::registers::Registers;

pub struct Xhci<T: Translate + 'static> {
    pub registers: Registers,
    pub device_contexts: Vec<Pin<Box<DeviceContext>>>,
    pub dcbaa: Pin<Box<DeviceContextBaseAddressArray>>,
    pub command_ring: TrbRing,
    pub event_ring: EventRing,
    pub translator: &'static Mutex<T>,
}

impl<T: Translate> Xhci<T> {
    pub fn new(pci_base: *mut u8, translator: &'static Mutex<T>) -> Self {
        Self {
            registers: unsafe{Registers::new(pci_base)},
            device_contexts: Vec::new(),
            dcbaa: Box::pin(DeviceContextBaseAddressArray::new()),
            command_ring: TrbRing::new(2, &translator),
            event_ring: EventRing::new(4, &translator),
            translator,
        }
    }
}

impl<T: Translate> UsbController for Xhci<T> {
    fn initialize(&mut self) {
        self.registers.operational.config().write(0x10);

        let dcbaap = self.dcbaa.as_ref().get_ref() as *const _ as u64;
        let dcbaap = self.translator.lock().translate_addr(VirtAddr::new(dcbaap)).unwrap();
        self.registers.operational.dcbaap().write(dcbaap.as_u64());

        let crdp = self.command_ring.first_trb() as *const _ as u64;
        let crdp = self.translator.lock().translate_addr(VirtAddr::new(crdp)).unwrap();
        self.registers.operational.crcr().write(crdp.as_u64() | 0x1);

        self.registers.runtime.erstsz(0).write(self.event_ring.size() as u32);

        let erdp = self.event_ring.first_trb() as *const _ as u64;
        let erdp = self.translator.lock().translate_addr(VirtAddr::new(erdp)).unwrap();
        self.registers.runtime.erdp(0).write(erdp.as_u64());

        let erstba = &self.event_ring.segment_table()[0] as *const _ as u64;
        let erstba = self.translator.lock().translate_addr(VirtAddr::new(erstba)).unwrap();
        self.registers.runtime.erstba(0).write(erstba.as_u64());

        self.registers.operational.usbcmd().write(0x0000_0001);
    }

    fn poll_event(&self) -> &Trb {
        self.event_ring.current_event()
    }
}