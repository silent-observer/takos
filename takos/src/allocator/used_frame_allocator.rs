use takobl_api::PHYSICAL_MEMORY_OFFSET;
use x86_64::{
    structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB},
    PhysAddr,
};

struct FreeFrameListNode {
    next: Option<&'static mut FreeFrameListNode>,
}

struct FreeFrameList {
    first: Option<&'static mut FreeFrameListNode>,
}

pub struct UsedFrameAllocator {
    free_frame_list: FreeFrameList,
}

impl UsedFrameAllocator {
    pub fn new() -> Self {
        Self {
            free_frame_list: FreeFrameList { first: None },
        }
    }

    fn add_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let frame_addr = frame.start_address().as_u64() + PHYSICAL_MEMORY_OFFSET;
        let node = frame_addr as *mut FreeFrameListNode;
        let node = unsafe { node.as_mut().unwrap() };

        node.next = self.free_frame_list.first.take();
        self.free_frame_list.first = Some(node);
    }

    fn get_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        match self.free_frame_list.first.take() {
            Some(node) => {
                self.free_frame_list.first = node.next.take();
                let addr = node as *mut FreeFrameListNode;
                Some(
                    PhysFrame::from_start_address(PhysAddr::new(
                        addr as u64 - PHYSICAL_MEMORY_OFFSET,
                    ))
                    .unwrap(),
                )
            }
            None => None,
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for UsedFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.get_frame()
    }
}

impl FrameDeallocator<Size4KiB> for UsedFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        self.add_frame(frame);
    }
}
