use core::{marker::PhantomPinned, pin::Pin};

use alloc::vec;
use alloc::{vec::Vec, boxed::Box};
use spin::Mutex;
use x86_64::{structures::paging::Translate, VirtAddr};


#[derive(Debug, Copy, Clone)]
#[repr(C, align(16))]
pub struct Trb {
    parameter: u64,
    status: u32,
    control: u32,
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

    pub fn enqueue_trb(&mut self, trb: Trb) {
        let old_trb = self.data[self.enqueue_segment].as_mut().trb(self.enqueue_index);
        *old_trb = trb;
        if self.cycle_state {
            old_trb.status |= 0x1;
        } else {
            old_trb.status &= !0x1;
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

#[derive(Debug, Clone)]
pub struct EventRing {
    data: Pin<Box<[TrbRingSegment]>>,
    segment_table: Pin<Box<[(u64, u64)]>>,
    dequeue_segment: usize,
    dequeue_index: u8,
    cycle_state: bool,
}

impl EventRing {
    pub fn new(segments: usize, translator: &Mutex<impl Translate>) -> Self {
        assert!(segments > 0);
        let data = vec![TrbRingSegment::new(); segments].into_boxed_slice();
        let data = Box::into_pin(data);

        let mut segment_table = vec![(0, 0); segments].into_boxed_slice();
        let translator = translator.lock();
        for i in 0..segments {
            let addr = &data.as_ref().get_ref()[i] as *const _ as u64;
            segment_table[i] = (translator.translate_addr(VirtAddr::new(addr)).unwrap().as_u64(), 256);
        }

        Self {
            data,
            segment_table: Box::into_pin(segment_table),
            dequeue_segment: 0,
            dequeue_index: 0,
            cycle_state: true,
        }
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn first_trb(&self) -> &Trb {
        &self.data.as_ref().get_ref()[0].data[0]
    }

    pub fn segment_table(&self) -> &[(u64, u64)] {
        &self.segment_table
    }

    pub fn current_event(&self) -> &Trb {
        &self.data.as_ref().get_ref()[self.dequeue_segment].data[self.dequeue_index as usize]
    }
}