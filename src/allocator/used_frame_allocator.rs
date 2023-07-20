use core::ptr::null_mut;

use takobl_api::PHYSICAL_MEMORY_OFFSET;
use x86_64::{structures::paging::{PhysFrame, Size4KiB, FrameAllocator, FrameDeallocator}, PhysAddr};

struct FreeFrameListNode {
    next: *mut FreeFrameListNode,
}

struct FreeFrameList {
    first: *mut FreeFrameListNode
}

unsafe impl Send for FreeFrameList {}
unsafe impl Sync for FreeFrameList {}

pub struct UsedFrameAllocator {
    free_frame_list: FreeFrameList,
}

impl UsedFrameAllocator {
    pub fn new() -> Self {
        Self {
            free_frame_list: FreeFrameList {
                first: null_mut(),
            }
        }
    }

    fn add_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let frame_addr = frame.start_address().as_u64() + PHYSICAL_MEMORY_OFFSET;
        let node = frame_addr as *mut FreeFrameListNode;
        unsafe {
            (*node).next = self.free_frame_list.first;
            self.free_frame_list.first = node;
        }
    }

    fn get_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let node = self.free_frame_list.first;
        self.free_frame_list.first = unsafe{node.as_ref()?.next};
        Some(PhysFrame::from_start_address(PhysAddr::new(node as u64 - PHYSICAL_MEMORY_OFFSET)).unwrap())
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