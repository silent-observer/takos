mod registers;
mod contexts;
pub mod trb;
mod commands;
pub mod transfer;
mod device_info;

use core::pin::Pin;

use alloc::collections::BTreeMap;
use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;
use futures::channel::oneshot::Sender;
use futures::future::join_all;
use log::{info, error};
use spin::Mutex;
use tako_async::timer::Timer;

use crate::controller::{UsbController, MemoryInterface};
use crate::xhci::trb::{EnableSlotCommandTrb, CompletionCode, CommandCompletionEventTrb};

use self::contexts::{DeviceContext, DeviceContextBaseAddressArray, InputContext};
use self::device_info::DeviceInfo;
use self::trb::{TrbRing, EventRing, Trb, TrbType, DisableSlotCommandTrb};
use self::registers::Registers;

pub struct PortData {
    pub port: u8,
    pub slot_id: u8,
    transfer_ring: TrbRing,
    device_context: Pin<Box<DeviceContext>>,
    max_packet_size: u16,
    info: Option<DeviceInfo>
}

pub struct Xhci<Mem: MemoryInterface + 'static> {
    pub registers: Registers,
    pub dcbaa: Mutex<Pin<Box<DeviceContextBaseAddressArray>>>,
    pub scratchpad_array: Mutex<Pin<Box<[u64; 64]>>>,
    pub scratchpads: Mutex<Vec<Pin<Box<[u8; 4096]>>>>,

    pub command_ring: Mutex<TrbRing>,
    pub event_ring: Mutex<EventRing>,
    pub ports_data: Mutex<Vec<PortData>>,
    pub mem: &'static Mem,

    pending_event_senders: Mutex<BTreeMap<(TrbType, u64), Sender<Trb>>>,
}

impl<Mem: MemoryInterface + 'static> Xhci<Mem> {
    pub fn new(pci_base: *mut u8, mem: &'static Mem) -> Self {
        Self {
            registers: unsafe{Registers::new(pci_base)},
            dcbaa: Mutex::new(Box::pin(DeviceContextBaseAddressArray::new())),
            scratchpad_array: Mutex::new(Box::pin([0; 64])),
            scratchpads: Mutex::new(Vec::new()),
            command_ring: Mutex::new(TrbRing::new(2, mem)),
            event_ring: Mutex::new(EventRing::new(mem)),
            ports_data: Mutex::new(Vec::new()),
            mem,

            pending_event_senders: Mutex::new(BTreeMap::new()),
        }
    }
}

#[async_trait(?Send)]
impl<Mem: MemoryInterface + 'static> UsbController for Xhci<Mem> {
    fn initialize(&self) {
        if self.registers.operational.usbsts().read() & 0x1 == 0 {
            self.registers.operational.usbcmd().write(0x0);
        }
        while self.registers.operational.usbsts().read() & 0x1 == 0 {}
        self.registers.operational.usbcmd().write(0x2);
        while self.registers.operational.usbcmd().read() & 0x2 != 0 {}

        self.registers.operational.config().write(0x10);

        let dcbaap = self.dcbaa.lock().as_ref().get_ref() as *const _ as u64;
        let dcbaap = self.mem.to_physical(dcbaap).unwrap();
        self.registers.operational.dcbaap().write(dcbaap);

        let hcs_params_2 = self.registers.capabilities.hcs_params_2().read();
        let max_scratchpads = hcs_params_2 >> 27 & 0x1F | hcs_params_2 >> 16 & 0x3E0;
        
        assert!(max_scratchpads <= 64);
        self.scratchpad_array.lock();
        for i in 0..max_scratchpads {
            let b = Box::pin([0; 4096]);
            let addr = b.as_ref().get_ref() as *const _ as u64;
            let addr = self.mem.to_physical(addr).unwrap();
            self.scratchpads.lock().push(b);
            self.scratchpad_array.lock()[i as usize] = addr;
        }
        let scratchpad_array_addr = self.scratchpad_array.lock().as_ref().get_ref() as *const _ as u64;
        let scratchpad_array_addr = self.mem.to_physical(scratchpad_array_addr).unwrap();
        self.dcbaa.lock().0[0] = scratchpad_array_addr;

        let crdp = self.command_ring.lock().first_trb() as *const _ as u64;
        let crdp = self.mem.to_physical(crdp).unwrap();
        self.registers.operational.crcr().write(crdp | 0x1);

        self.registers.operational.dnctrl().write(0x1);
        self.registers.runtime.erstsz(0).write(0x1);

        let erdp = self.event_ring.lock().first_trb() as *const _ as u64;
        let erdp = self.mem.to_physical(erdp).unwrap();
        self.registers.runtime.erdp(0).write(erdp | 0x8);

        let erstba = self.event_ring.lock().segment_table() as *const _ as u64;
        let erstba = self.mem.to_physical(erstba).unwrap();
        self.registers.runtime.erstba(0).write(erstba);

        self.registers.operational.usbsts().write(0x0000_0008);
        self.registers.operational.usbcmd().write(0x0000_0001);
    }

    async fn initialize_devices(&self) {
        let max_slots = self.registers.capabilities.hcs_params_1().read() as u8;
        join_all((1..=max_slots).into_iter().map(|slot|
            self.free_slot(slot)
        )).await;

        let max_ports = self.registers.capabilities.hcs_params_1().read() >> 24;
        let ports: Vec<PortData> = join_all((1..=max_ports).into_iter().map(|port|
            self.initialize_device(port as u8)
        )).await.into_iter().flatten().collect();
        *self.ports_data.lock() = ports;

        join_all(self.ports_data.lock().iter_mut().map(|port_data|
            self.identify_device(port_data)
        )).await;
        // for port_data in self.ports_data.lock().iter_mut() {
        //     self.identify_device(port_data).await;
        // }
    }

    async fn run(&self) {
        self.handle_events().await;
    }
}

impl<Mem: MemoryInterface + 'static> Xhci<Mem> {
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
                match trb.trb_type() {
                    TrbType::TransferEvent |
                    TrbType::CommandCompletionEvent |
                    TrbType::PortStatusChangeEvent => {
                        self.handle_event_notification(trb);
                    }
                    _ => {
                        info!("Got event {:X?}!", trb);
                    }
                }
                event_ring.advance();
                self.registers.runtime.erdp(0).write(event_ring.get_current_addr(self.mem))
            }
            drop(event_ring);
            Timer::new(1).await;
        }
    }
}

impl<Mem: MemoryInterface + 'static> Xhci<Mem> {
    async fn free_slot(&self, slot: u8) {
        let response: CommandCompletionEventTrb =
            self.send_command(DisableSlotCommandTrb(slot).into()).await.try_into().unwrap();
        match response.code {
            CompletionCode::Success => {
                info!("Successfully disabled slot {}", slot);
            },
            CompletionCode::SlotNotEnabledError => {
                info!("Slot {} not enabled", slot);
            }
            _ => {
                error!("Couldn't disable slot {}: {:?}", slot, response);
            }
        }
        
    }

    async fn initialize_device(&self, port: u8) -> Option<PortData> {
        let portsc = self.registers.operational.portsc(port as usize).read();
        info!("Port {} = {:08X}", port, portsc);
        if portsc & 0x1 == 0 { return None; } // No device

        self.reset_port(port as u8).await;
        let portsc = self.registers.operational.portsc(port as usize).read();
        info!("Port {} -> {:08X}", port, portsc);

        let slot_id = self.enable_slot(port).await?;
        info!("Slot enable for port {}: slot_id = {}", port, slot_id);

        let port_data = self.address_device(slot_id, port).await?;

        Some(port_data)
    }

    async fn enable_slot(&self, port: u8) -> Option<u8> {
        let response: CommandCompletionEventTrb = 
            self.send_command(EnableSlotCommandTrb.into())
                .await
                .try_into()
                .expect("Couldn't allocate slot");
        if response.code != CompletionCode::Success {
            error!("Failed to enable slot for port {}: {:X?}", port, response);
            return None;
        }

        Some(response.slot_id)
    }

    async fn address_device(&self, slot_id: u8, port: u8) -> Option<PortData> {
        let portsc = self.registers.operational.portsc(port as usize).read();
        let port_speed = portsc >> 10 & 0xF;
        info!("Trying to address slot {} on port {}", slot_id, port);
        info!("Port speed: {}", port_speed);
        let max_packet_size = 8; //match port_speed {
        //     1 | 3 => 64,
        //     2 => 8,
        //     4..=7 => 512,
        //     _ => {
        //         error!("Unsupported port speed: {}", port_speed);
        //         return None;
        //     }
        // };

        let mut input_context = Box::new(InputContext::new());
        input_context.control_context.add_context_flags = 0x3;
        input_context.slot_context.set_route_string(0x0);
        input_context.slot_context.set_speed(port_speed as u8);
        input_context.slot_context.set_root_hub_port_number(port);
        input_context.slot_context.set_context_entries(0x1);
        info!("Constructed input_context");
        
        let mut result = PortData {
            port,
            slot_id,
            max_packet_size,
            transfer_ring: TrbRing::new(2, self.mem),
            device_context: Box::pin(DeviceContext::new()),
            info: None,
        };

        let ep = &mut input_context.endpoint_contexts[0];
        ep.set_ep_type(0x4); // Control
        ep.set_lsa(true);
        ep.set_max_packet_size(max_packet_size);
        ep.set_max_burst_size(0);
        info!("Current dequeue ptr {:08X}", result.transfer_ring.get_current_addr(self.mem));
        ep.set_tr_dequeue_ptr(result.transfer_ring.get_current_addr(self.mem));
        ep.set_dcs(true);
        //ep.set_interval(3);
        ep.set_max_pstreams(0);
        ep.set_mult(0);
        ep.set_cerr(3);
        ep.set_average_trb_length(8);

        result.device_context.as_mut().get_mut().slot_context = input_context.slot_context;
        result.device_context.as_mut().get_mut().endpoint_contexts[0] = input_context.endpoint_contexts[0];

        let device_context_addr = result.device_context.as_ref().get_ref() as *const _ as u64;
        let device_context_addr = self.mem.to_physical(device_context_addr).unwrap();

        self.dcbaa.lock().as_mut().get_mut().0[slot_id as usize] = device_context_addr;
        info!("Allocated everything, sending the first address command...");

        let response: CommandCompletionEventTrb =
            self.send_address_device_command(slot_id, &input_context, true).await.try_into().ok()?;
        if response.code != CompletionCode::Success {
            error!("Failed to address device for port {}: {:X?}", port, response);
            return None;
        }

        info!("Getting short descriptor...");
        let data = self.get_short_descriptor(&mut result).await?;
        info!("Got short descriptor {:X?}", data);

        info!("Sending the second address command...");

        let response: CommandCompletionEventTrb =
            self.send_address_device_command(slot_id, &input_context, false).await.try_into().ok()?;
        if response.code != CompletionCode::Success {
            error!("Failed to address device for port {}: {:X?}", port, response);
            return None;
        }

        let context = result.device_context.as_ref().get_ref().slot_context;
        info!("Slot {} = {:X?}", slot_id, context);

        Some(result)
    }

    async fn identify_device(&self, port_data: &mut PortData) {
        let data = self.examine_device(port_data).await;
        if data.is_some() {
            port_data.info = data;
        }
    }
}