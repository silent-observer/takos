#![no_main]
#![no_std]

const PAGE_SIZE: u64 = 4096;
const MAX_FREE_MEMORY: usize = 63;

#[derive(Debug, Clone)]
pub struct FrameBufferData {
    pub buffer_addr: *mut u8,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
}

#[derive(Debug)]
pub struct BootData {
    pub frame_buffer: FrameBufferData,
    pub free_memory_map: FreeMemoryMap,
    pub loader_code: MemoryRegion,
    pub image_device_path: &'static str,
    pub ramdisk: &'static mut [u8],
}

#[derive(Debug, Copy, Clone)]
pub struct MemoryRegion {
    pub start: u64,
    pub pages: u64,
}

impl MemoryRegion {
    #[inline]
    pub fn end(&self) -> u64 {
        self.start + self.pages * PAGE_SIZE
    }

    #[inline]
    pub fn page_addr(&self, index: u64) -> u64 {
        self.start + index * PAGE_SIZE
    }
}

#[derive(Debug, Clone)]
pub struct FreeMemoryMap{
    pub count: usize,
    pub data: [MemoryRegion; MAX_FREE_MEMORY],
}


impl FrameBufferData {
    pub fn new() -> FrameBufferData {
        FrameBufferData {
            buffer_addr: core::ptr::null_mut(),
            width: 0,
            height: 0,
            stride: 0,
        }
    }
}

impl FreeMemoryMap {
    pub fn new() -> FreeMemoryMap {
        FreeMemoryMap {
            count: 0,
            data: [MemoryRegion { start: 0, pages: 0 }; MAX_FREE_MEMORY],
        }
    }

    pub fn add(&mut self, region: MemoryRegion) {
        self.data[self.count] = region;
        self.count += 1;
    }

    fn remove_arr(&mut self, index: usize) {
        assert!(self.count > 0);
        assert!(index < self.count);
        for i in index..self.count-1 {
            self.data[i] = self.data[i+1];
        }
        self.count -= 1;
    }

    fn insert(&mut self, index: usize, region: MemoryRegion) {
        assert!(self.count < self.data.len());
        for i in (index..self.count).rev() {
            self.data[i+1] = self.data[i];
        }
        self.data[index] = region;
        self.count += 1;
    }

    pub fn remove(&mut self, region: &MemoryRegion) {
        for i in 0..self.count {
            if self.data[i].start <= region.start {
                let this_end = self.data[i].end();
                let region_end = region.end();
                if this_end < region_end {
                    break;
                }
                if self.data[i].start == region.start && this_end == region_end {
                    self.remove_arr(i);
                } else if self.data[i].start == region.start {
                    self.data[i].start += region.pages * PAGE_SIZE;
                    self.data[i].pages -= region.pages;
                } else if this_end == region_end {
                    self.data[i].pages -= region.pages;
                } else {
                    let pages_before = (region.start - self.data[i].start) / PAGE_SIZE;
                    let pages_after = (this_end - region_end) / PAGE_SIZE;
                    self.data[i].pages = pages_before;
                    self.insert(i+1, MemoryRegion { start: region_end, pages: pages_after });
                }
                break;
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item=&MemoryRegion> {
        self.data[..self.count].iter()
    }
}


pub const PHYSICAL_MEMORY_OFFSET: u64 = 0xFFFF_C000_0000_0000;