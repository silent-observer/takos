mod registers;
mod contexts;
pub mod trb;
mod commands;

use core::pin::Pin;

use alloc::collections::BTreeMap;
use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;
use futures::channel::oneshot::Sender;
use futures::future::join_all;
use log::info;
use spin::Mutex;
use tako_async::timer::Timer;
use x86_64::VirtAddr;
use x86_64::structures::paging::Translate;

use crate::controller::UsbController;

use self::contexts::{DeviceContext, DeviceContextBaseAddressArray};
use self::trb::{TrbRing, EventRing, Trb, TrbType};
use self::registers::Registers;

pub struct Xhci<T: Translate + 'static> {
    pub registers: Registers,
    pub device_contexts: Vec<Pin<Box<DeviceContext>>>,
    pub dcbaa: Pin<Box<DeviceContextBaseAddressArray>>,
    pub command_ring: Mutex<TrbRing>,
    pub event_ring: Mutex<EventRing>,
    pub translator: &'static Mutex<T>,

    pending_event_senders: Mutex<BTreeMap<(TrbType, u64), Sender<Trb>>>,
}

impl<T: Translate> Xhci<T> {
    pub fn new(pci_base: *mut u8, translator: &'static Mutex<T>) -> Self {
        Self {
            registers: unsafe{Registers::new(pci_base)},
            device_contexts: Vec::new(),
            dcbaa: Box::pin(DeviceContextBaseAddressArray::new()),
            command_ring: Mutex::new(TrbRing::new(2, &translator)),
            event_ring: Mutex::new(EventRing::new(&translator)),
            translator,

            pending_event_senders: Mutex::new(BTreeMap::new()),
        }
    }

    async fn initialize_device(&self, port: u8) {
        let portsc = self.registers.operational.portsc(port as usize).read();
        if portsc & 0x1 == 0 { return; } // No device

        self.reset_port(port as u8).await;
        let portsc = self.registers.operational.portsc(port as usize).read();
        info!("Port {} = {:08X}", port, portsc);
    }
}

#[async_trait(?Send)]
impl<T: Translate> UsbController for Xhci<T> {
    fn initialize(&self) {
        self.registers.operational.config().write(0x10);

        let dcbaap = self.dcbaa.as_ref().get_ref() as *const _ as u64;
        let dcbaap = self.translator.lock().translate_addr(VirtAddr::new(dcbaap)).unwrap();
        self.registers.operational.dcbaap().write(dcbaap.as_u64());

        let crdp = self.command_ring.lock().first_trb() as *const _ as u64;
        let crdp = self.translator.lock().translate_addr(VirtAddr::new(crdp)).unwrap();
        self.registers.operational.crcr().write(crdp.as_u64() | 0x1);

        self.registers.runtime.erstsz(0).write(0x1);

        let erdp = self.event_ring.lock().first_trb() as *const _ as u64;
        let erdp = self.translator.lock().translate_addr(VirtAddr::new(erdp)).unwrap();
        self.registers.runtime.erdp(0).write(erdp.as_u64() | 0x8);

        let erstba = self.event_ring.lock().segment_table() as *const _ as u64;
        let erstba = self.translator.lock().translate_addr(VirtAddr::new(erstba)).unwrap();
        self.registers.runtime.erstba(0).write(erstba.as_u64());

        self.registers.operational.usbsts().write(0x0000_0008);
        self.registers.operational.usbcmd().write(0x0000_0001);
    }

    async fn initialize_devices(&self) {
        let max_ports = self.registers.capabilities.hcs_params_1().read() >> 24;
        join_all((1..=max_ports).into_iter().map(|port|
            self.initialize_device(port as u8)
        )).await;
    }

    async fn run(&self) {
        self.handle_events().await;
    }
}

impl<T:Translate> Xhci<T> {
    pub fn port_status_change(&self, port: u8) {
        let portsc = self.registers.operational.portsc(port as usize).read();
        info!("Port {} status change: {:08X}", port, portsc);
        let write_portsc = portsc & 0xC3E0 | 0x20_0000;
        self.registers.operational.portsc(port as usize).write(write_portsc);
    }

    pub async fn handle_events(&self) {
        loop {
            let mut event_ring = self.event_ring.lock();
            while event_ring.has_event() {
                let trb = event_ring.current_event();
                info!("Got event {:X?}!", trb);
                match trb.trb_type() {
                    TrbType::CommandCompletionEvent | TrbType::PortStatusChangeEvent => {
                        self.handle_event_notification(trb);
                    }
                    _ => {}
                }
                event_ring.advance();
                self.registers.runtime.erdp(0).write(event_ring.get_current_addr(self.translator))
            }
            Timer::new(1).await;
        }
    }
}