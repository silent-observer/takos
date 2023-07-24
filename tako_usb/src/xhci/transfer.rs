pub mod standard;

use alloc::vec::Vec;
use spin::Mutex;
use x86_64::{structures::paging::Translate, VirtAddr};

use super::{trb::{DataTransferDirection, Trb, DataTrb, NormalTrb, TypeOfRequest, Recipient, SetupTrb, StatusTrb, TrbRing, TrbType}, Xhci, commands::PendingEventFuture};

fn construct_data_td(data: &[u8], dir: DataTransferDirection, max_packet_size: usize, translator: &Mutex<impl Translate>) -> Vec<Trb> {
    let mut buffers: Vec<(u64, usize, usize)> = Vec::new();
    let mut offset = 0;
    let translator = translator.lock();
    while offset < data.len() {
        let start_virt_addr = &data[offset] as *const u8 as u64;
        let start_phys_addr = translator.translate_addr(VirtAddr::new(start_virt_addr)).unwrap().as_u64();
        let size = 0x1000 - (start_phys_addr & 0xFFF) as usize;
        let size = size.min(data.len() - offset);
        buffers.push((start_phys_addr, size, offset));
        offset += size;
    }

    let trb_count = buffers.len();
    let packet_count = (data.len() + max_packet_size - 1) / max_packet_size;

    let trbs = buffers.into_iter().enumerate().map(|(i, (virt_addr, size, offset))| {
        let packets_transferred = offset / max_packet_size;
        let td_size = packet_count - packets_transferred;
        let td_size = td_size.min(31);
        let td_size = if i == trb_count - 1 {0} else {td_size as u8};

        let mut trb: Trb = if i == 0 {
            DataTrb::new(virt_addr, size as u32, td_size, dir).into()
        } else {
            NormalTrb::new(virt_addr, size as u32, td_size).into()
        };
        if i != trb_count - 1 {
            trb.control |= 0x10;
        }
        trb
    }).collect();
    trbs
}

pub struct ControlTransferBuilder<'a, T: Translate + 'static> {
    translator: &'static Mutex<T>,
    max_packet_size: usize,

    data: Option<&'a [u8]>,
    direction: Option<DataTransferDirection>,

    type_of_request: Option<TypeOfRequest>,
    recipient: Option<Recipient>,
    request: Option<u8>,
    value: Option<u16>,
    index: Option<u16>,
}

impl<'a, T: Translate + 'static> ControlTransferBuilder<'a, T> {
    pub fn new(translator: &'static Mutex<T>, max_packet_size: usize) -> Self {
        Self {
            translator,
            max_packet_size,

            data: None,
            direction: None,

            type_of_request: None,
            recipient: None,
            request: None,
            value: None,
            index: None,
        }
    }
    
    pub fn with_data(&mut self, data: &'a [u8]) -> &mut Self {
        self.data = Some(data);
        self
    }

    pub fn direction(&mut self, dir: DataTransferDirection) -> &mut Self {
        self.direction = Some(dir);
        self
    }

    pub fn type_of_request(&mut self, type_of_request: TypeOfRequest) -> &mut Self {
        self.type_of_request = Some(type_of_request);
        self
    }

    pub fn recipient(&mut self, recipient: Recipient) -> &mut Self {
        self.recipient = Some(recipient);
        self
    }

    pub fn request(&mut self, request: u8) -> &mut Self {
        self.request = Some(request);
        self
    }

    pub fn value(&mut self, value: u16) -> &mut Self {
        self.value = Some(value);
        self
    }

    pub fn index(&mut self, index: u16) -> &mut Self {
        self.index = Some(index);
        self
    }

    pub fn build(self) -> Vec<Trb> {
        let mut trbs: Vec<Trb> = Vec::new();
        let length = self.data.map_or(0, |data| data.len());
        let setup = SetupTrb {
            dir: self.direction.expect("Transfer direction is missing"),
            type_of_request: self.type_of_request.expect("Type of request is missing"),
            recipient: self.recipient.expect("Recepient is missing"),
            request: self.request.expect("Request is missing"),
            value: self.value.unwrap_or(0),
            index: self.index.unwrap_or(0),
            length: length as u16,
        };
        trbs.push(setup.into());

        if length > 0 {
            let data = self.data.unwrap();
            trbs.extend(construct_data_td(
                data, 
                self.direction.unwrap(),
                self.max_packet_size,
                self.translator
            ));
        }

        let status_dir = if length > 0 {
            self.direction.unwrap().opposite()
        } else {
            DataTransferDirection::DeviceToHost
        };
        trbs.push(StatusTrb(status_dir).into());
        trbs
    }
}

impl<T: Translate> Xhci<T> {
    pub fn send_transfer(&self, slot_id: u8, transfer_ring: &mut TrbRing, trbs: &[Trb]) -> PendingEventFuture {
        let event_ring = self.event_ring.lock();
        let (last_trb, other_trbs) = trbs.split_last().unwrap();
        for trb in other_trbs {
            transfer_ring.enqueue_trb(*trb);
        }
        let addr = transfer_ring.get_current_addr(&self.translator);
        let future = self.new_pending_event(TrbType::TransferEvent, addr);

        transfer_ring.enqueue_trb(*last_trb);
        self.registers.doorbell.ring_device_control(slot_id);
        drop(event_ring);
        future
    }
}