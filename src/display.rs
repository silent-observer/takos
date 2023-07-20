#[derive(Debug, Copy, Clone)]
pub struct ColorRGB(u8, u8, u8);

use core::ptr::null_mut;

use takobl_api::FrameBufferData;

const PHYSICAL_MEMORY_OFFSET: usize = 32 * 1024 * 1024 * 1024 * 1024;

#[derive(Debug)]
pub struct FrameBuffer {
    base_addr: *mut u8,
    width: usize,
    height: usize,
    stride: usize,
}

unsafe impl Send for FrameBuffer {}

impl FrameBuffer {
    pub fn empty() -> FrameBuffer {
        FrameBuffer {
            base_addr: null_mut(),
            width: 0,
            height: 0,
            stride: 0,
        }
    }

    pub fn new(data: &FrameBufferData) -> FrameBuffer {
        FrameBuffer {
            base_addr: unsafe{data.buffer_addr.add(PHYSICAL_MEMORY_OFFSET as usize)},
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
                    self.base_addr.add(src_index * 4),
                    self.base_addr.add(dest_index * 4),
                    self.width * 4)
            }
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