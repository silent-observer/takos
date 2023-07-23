use core::{marker::PhantomPinned, pin::Pin, mem::transmute};

use alloc::{vec::Vec, boxed::Box};
use spin::Mutex;
use x86_64::{structures::paging::Translate, VirtAddr};


#[derive(Debug, Copy, Clone)]
#[repr(C, align(16))]
pub struct Trb {
    pub parameter: u64,
    pub status: u32,
    pub control: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[allow(dead_code)]
pub enum TrbType {
    Empty = 0,
    Normal,
    Setup,
    Data,
    Status,
    Isoch,
    Link,
    EventData,
    NoOp,
    EnableSlotCommand,
    DisableSlotCommand,
    AddressDeviceCommand,
    ConfigureEndpointCommand,
    EvaluateContextCommand,
    ResetEndpointCommand,
    StopEndpointCommand,
    SetTrDequePointerCommand,
    ResetDeviceCommand,
    ForceEventCommand,
    NegotiateBandwidthCommand,
    SetLatencyToleranceValueCommand,
    GetPortBandwidthCommand,
    ForceHeaderCommand,
    NoOpCommand,
    GetExtendedPropertyCommand,
    SetExtendedPropertyCommand,
    TransferEvent = 32,
    CommandCompletionEvent,
    PortStatusChangeEvent,
    BandwidthRequestEvent,
    DoorbellEvent,
    HostControllerEvent,
    DeviceNotificationEvent,
    MfindexWrapEvent,
}

impl TrbType {
    pub const fn to_control(&self) -> u32 {
        ((*self as u8) as u32) << 10
    }

    pub fn from_control(control: u32) -> Self {
        let val = (control >> 10) as u8 & 0x3F;
        match val {
            0..=25 | 32..=39 => unsafe {transmute(val)}
            _ => panic!("Invalid TRB type!")
        }
    }
}

impl Trb {
    fn empty() -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: 0,
        }
    }
    fn link_trb(addr: u64, is_last: bool) -> Self {
        let control = TrbType::Link.to_control();
        let control = if is_last {control | 0x2} else {control};
        Self {
            parameter: addr,
            status: 0,
            control,
        }
    }

    pub fn trb_type(&self) -> TrbType {
        TrbType::from_control(self.control)
    }

    pub fn noop_command() -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: TrbType::NoOpCommand.to_control(),
        }
    }
}

const TRB_RING_SIZE: usize = 256;

#[derive(Debug, Copy, Clone)]
#[repr(C, align(16))]
struct TrbRingSegment {
    data: [Trb; TRB_RING_SIZE],
    _pin: PhantomPinned
}

impl TrbRingSegment {
    fn new() -> Self {
        Self {
            data: [Trb::empty(); TRB_RING_SIZE],
            _pin: PhantomPinned,
        }
    }

    fn trb(self: Pin<&mut Self>, i: u8) -> &mut Trb {
        unsafe {
            &mut self.get_unchecked_mut().data[i as usize]
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrbRing {
    data: Vec<Pin<Box<TrbRingSegment>>>,
    enqueue_segment: usize,
    enqueue_index: u8,
    cycle_state: bool,
}

impl TrbRing {
    pub fn new(segments: usize, translator: &Mutex<impl Translate>) -> Self {
        assert!(segments > 0);
        let mut data = Vec::with_capacity(segments);
        for _ in 0..segments {
            data.push(Box::pin(TrbRingSegment::new()));
        }
        for i in 0..segments {
            let next_index = if i == segments - 1 {0} else {i + 1};
            let next_virt_addr = VirtAddr::new(&data[next_index].data[0] as *const Trb as u64);
            let phys_addr = translator.lock().translate_addr(next_virt_addr).unwrap();
            let last_trb = data[i].as_mut().trb((TRB_RING_SIZE-1) as u8);
            *last_trb = Trb::link_trb(phys_addr.as_u64(), i == segments - 1);
        }
        Self {
            data,
            enqueue_segment: 0,
            enqueue_index: 0,
            cycle_state: true,
        }
    }

    pub fn get_current_addr(&self, translator: &Mutex<impl Translate>) -> u64 {
        let addr = &self.data[self.enqueue_segment].as_ref().data[self.enqueue_index as usize];
        let addr = addr as *const Trb as u64;
        let addr = translator.lock().translate_addr(VirtAddr::new(addr)).unwrap();
        addr.as_u64()
    }

    pub fn enqueue_trb(&mut self, trb: Trb) {
        let old_trb = self.data[self.enqueue_segment].as_mut().trb(self.enqueue_index);
        *old_trb = trb;
        if self.cycle_state {
            old_trb.control |= 0x1;
        } else {
            old_trb.control &= !0x1;
        }
        
        self.enqueue_index += 1;
        if self.enqueue_index as usize == TRB_RING_SIZE - 1 {
            let link_trb = self.data[self.enqueue_segment].as_mut().trb(self.enqueue_index);
            if self.cycle_state {
                link_trb.control |= 0x1;
            } else {
                link_trb.control &= !0x1;
            }

            self.enqueue_index = 0;
            self.enqueue_segment += 1;
            if self.enqueue_segment >= self.data.len() {
                self.enqueue_segment = 0;
                self.cycle_state = !self.cycle_state;
            }
        }
    }

    pub fn first_trb(&self) -> &Trb {
        &self.data[0].as_ref().get_ref().data[0]
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, align(64))]
pub struct ErstEntry {
    pub base_addr: u64,
    pub size: u16
}

#[derive(Debug, Clone)]
pub struct EventRing {
    data: Pin<Box<TrbRingSegment>>,
    segment_table: Pin<Box<ErstEntry>>,
    dequeue_index: u8,
    cycle_state: bool,
}

impl EventRing {
    pub fn new(translator: &Mutex<impl Translate>) -> Self {
        let data = Box::pin(TrbRingSegment::new());


        let addr = data.as_ref().get_ref() as *const _ as u64;
        let segment_table = ErstEntry{
            base_addr: translator.lock().translate_addr(VirtAddr::new(addr)).unwrap().as_u64(),
            size: TRB_RING_SIZE as u16
        };

        Self {
            data,
            segment_table: Box::pin(segment_table),
            dequeue_index: 0,
            cycle_state: true,
        }
    }

    pub fn first_trb(&self) -> &Trb {
        &self.data.as_ref().get_ref().data[0]
    }

    pub fn segment_table(&self) -> &ErstEntry {
        &self.segment_table
    }

    pub fn current_event(&self) -> &Trb {
        &self.data.as_ref().get_ref().data[self.dequeue_index as usize]
    }

    pub fn has_event(&self) -> bool {
        (self.current_event().control & 0x1 != 0) == self.cycle_state
    }

    pub fn advance(&mut self) {
        self.dequeue_index = self.dequeue_index.wrapping_add(1);
        if self.dequeue_index == TRB_RING_SIZE as u8 {
            self.dequeue_index = 0;
        }

        if self.dequeue_index == 0 {
            self.cycle_state = !self.cycle_state;
        }
    }

    pub fn get_current_addr(&self, translator: &Mutex<impl Translate>) -> u64 {
        let addr = &self.data.as_ref().data[self.dequeue_index as usize];
        let addr = addr as *const Trb as u64;
        let addr = translator.lock().translate_addr(VirtAddr::new(addr)).unwrap();
        addr.as_u64()
    }
}