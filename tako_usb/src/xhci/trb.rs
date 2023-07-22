use core::{marker::PhantomPinned, pin::Pin};

use alloc::vec;
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

impl Trb {
    fn empty() -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: 0,
        }
    }
    fn link_trb(addr: u64, is_last: bool) -> Self {
        Self {
            parameter: addr,
            status: 0,
            control: if is_last {6 << 10 | 0x2} else {6 << 10},
        }
    }

    pub fn trb_type(&self) -> u8 {
        (self.control >> 10) as u8 & 0x3F
    }

    pub fn noop_command() -> Self {
        Self {
            parameter: 0,
            status: 0,
            control: 23 << 10,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, align(16))]
struct TrbRingSegment {
    data: [Trb; 256],
    _pin: PhantomPinned
}

impl TrbRingSegment {
    fn new() -> Self {
        Self {
            data: [Trb::empty(); 256],
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
            let last_trb = data[i].as_mut().trb(255);
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
        if self.enqueue_index == 255 {
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
            size: 256
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
        if self.dequeue_index == 0 {
            self.cycle_state = !self.cycle_state;
        }
    }
}