#[derive(Debug, Copy, Clone)]
pub struct ColorRGB(u8, u8, u8);

use core::ptr::null_mut;

use takobl_api::FrameBufferData;
use x86_64::{VirtAddr, structures::paging::{PhysFrame, Size4KiB, FrameAllocator}, PhysAddr};

use crate::{paging::{PAGE_TABLE, map_writable_page}, allocator::frame_allocator::FRAME_ALLOCATOR};

#[derive(Debug)]
pub struct FrameBuffer {
    base_addr: *mut u8,
    double_buffer: *mut u8,
    width: usize,
    height: usize,
    stride: usize,
}

unsafe impl Send for FrameBuffer {}

impl FrameBuffer {
    pub fn empty() -> FrameBuffer {
        FrameBuffer {
            base_addr: null_mut(),
            double_buffer: null_mut(),
            width: 0,
            height: 0,
            stride: 0,
        }
    }

    pub fn is_init(&self) -> bool {
        !self.base_addr.is_null()
    }

    pub fn new(data: &FrameBufferData) -> FrameBuffer {
        use x86_64::structures::paging::{Mapper, Page, PageTableFlags};
        let physical_address = data.buffer_addr;
        const VIRTUAL_ADDRESS: u64 = 0x4000_0000_0000;
        const VIRTUAL_ADDRESS_DOUBLE: u64 = 0x4000_1000_0000;
        let size = 4 * data.height * data.stride;
        let pages = ((size + 4095) / 4096) as u64;
        for i in 0..pages {
            let virt = VIRTUAL_ADDRESS + i * 4096;
            let phys = physical_address as u64 + i * 4096;
            unsafe {
                let page = Page::<Size4KiB>::from_start_address(VirtAddr::new(virt)).unwrap();
                PAGE_TABLE.lock().map_to(
                    page,
                    PhysFrame::from_start_address(PhysAddr::new(phys)).unwrap(), 
                    PageTableFlags::PRESENT.union(PageTableFlags::WRITABLE).union(PageTableFlags::WRITE_THROUGH),
                    &mut *FRAME_ALLOCATOR.lock()
                ).unwrap().flush();
            }

        }
        for i in 0..pages {
            let virt = VIRTUAL_ADDRESS_DOUBLE + i * 4096;
            let frame = FRAME_ALLOCATOR.lock().allocate_frame().unwrap();
            map_writable_page(virt, frame);
        }

        FrameBuffer {
            base_addr: VIRTUAL_ADDRESS as *mut u8,
            double_buffer: VIRTUAL_ADDRESS_DOUBLE as *mut u8,
            width: data.width,
            height: data.height,
            stride: data.stride,
        }
    }

    pub fn fill(&self, color: ColorRGB) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn scroll_up(&self, offset: usize, fill_color: ColorRGB) {
        assert!(offset < self.height);
        assert!(offset != 0);
        for y in 0..self.height-offset {
            let dest_index = y * self.stride;
            let src_index = (y + offset) * self.stride;
            unsafe{
                core::ptr::copy_nonoverlapping(
                    self.double_buffer.add(src_index * 4),
                    self.double_buffer.add(dest_index * 4),
                    self.width * 4)
            }
        }
        unsafe{
            core::ptr::copy_nonoverlapping(
                self.double_buffer,
                self.base_addr,
                self.stride * self.height * 4)
        }
        for y in self.height-offset..self.height {
            for x in 0..self.width {
                self.put_pixel(x, y, fill_color);
            }
        }
    }

    pub fn put_pixel(&self, x: usize, y: usize, color: ColorRGB) {
        assert!(x < self.width);
        assert!(y < self.height);
        unsafe {
            let index = y * self.stride + x;
            self.base_addr.add(index * 4).write_volatile(color.2);
            self.base_addr.add(index * 4 + 1).write_volatile(color.1);
            self.base_addr.add(index * 4 + 2).write_volatile(color.0);
            self.double_buffer.add(index * 4).write(color.2);
            self.double_buffer.add(index * 4 + 1).write(color.1);
            self.double_buffer.add(index * 4 + 2).write(color.0);
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }
}

impl ColorRGB {
    pub fn from_hex(hex: u32) -> ColorRGB {
        ColorRGB(
            (hex >> 16) as u8,
            (hex >> 8) as u8,
            hex as u8,
        )
    }
}