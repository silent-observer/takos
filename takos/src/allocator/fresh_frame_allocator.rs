use takobl_api::FreeMemoryMap;
use x86_64::{
    structures::paging::{FrameAllocator, PhysFrame, Size4KiB},
    PhysAddr,
};

pub struct FreshFrameAllocator {
    initial_free_memory_map: FreeMemoryMap,

    current_region: usize,
    next_page_index: u64,
}

impl FreshFrameAllocator {
    pub fn new() -> Self {
        Self {
            initial_free_memory_map: FreeMemoryMap::new(),
            current_region: 0,
            next_page_index: 0,
        }
    }

    pub fn set_free_memory_map(&mut self, fmm: FreeMemoryMap) {
        self.initial_free_memory_map = fmm;
    }

    fn next_free_page(&mut self) -> Option<u64> {
        if self.current_region == self.initial_free_memory_map.count {
            return None;
        }

        let current_region = &self.initial_free_memory_map.data[self.current_region];
        let result = current_region.page_addr(self.next_page_index);
        self.next_page_index += 1;
        if self.next_page_index == current_region.pages {
            self.next_page_index = 0;
            self.current_region += 1;
        }
        Some(result)
    }
}

unsafe impl FrameAllocator<Size4KiB> for FreshFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let addr = self.next_free_page()?;
        Some(PhysFrame::from_start_address(PhysAddr::new(addr)).unwrap())
    }
}
