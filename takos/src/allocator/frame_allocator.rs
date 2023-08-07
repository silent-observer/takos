use spin::Mutex;
use takobl_api::FreeMemoryMap;
use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB};

use super::{fresh_frame_allocator::FreshFrameAllocator, used_frame_allocator::UsedFrameAllocator};
use lazy_static::lazy_static;

pub struct TakosFrameAllocator {
    fresh_frame_allocator: FreshFrameAllocator,
    used_frame_allocator: UsedFrameAllocator,
}

impl TakosFrameAllocator {
    pub fn new() -> Self {
        Self {
            fresh_frame_allocator: FreshFrameAllocator::new(),
            used_frame_allocator: UsedFrameAllocator::new(),
        }
    }

    pub fn set_free_memory_map(&mut self, fmm: FreeMemoryMap) {
        self.fresh_frame_allocator.set_free_memory_map(fmm);
    }
}

unsafe impl FrameAllocator<Size4KiB> for TakosFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if let Some(frame) = self.used_frame_allocator.allocate_frame() {
            Some(frame)
        } else {
            self.fresh_frame_allocator.allocate_frame()
        }
    }
}

impl FrameDeallocator<Size4KiB> for TakosFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        self.used_frame_allocator.deallocate_frame(frame);
    }
}

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<TakosFrameAllocator> =
        Mutex::new(TakosFrameAllocator::new());
}

pub fn init_frame_allocator(free_memory_map: FreeMemoryMap) {
    FRAME_ALLOCATOR.lock().set_free_memory_map(free_memory_map);
}

#[test_case]
fn test_frame_allocator() {
    use crate::{print, println};
    print!("test_frame_allocator... ");
    let frame_1 = FRAME_ALLOCATOR.lock().allocate_frame().unwrap();

    let frame_2 = FRAME_ALLOCATOR.lock().allocate_frame().unwrap();

    unsafe { FRAME_ALLOCATOR.lock().deallocate_frame(frame_1) };

    let frame_3 = FRAME_ALLOCATOR.lock().allocate_frame().unwrap();

    assert!(frame_3.start_address().as_u64() == frame_1.start_address().as_u64());

    println!("[ok]");
}
