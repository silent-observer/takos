#![no_main]
#![no_std]

#[derive(Debug)]
pub struct FrameBufferData {
    pub buffer_addr: *mut u8,
    pub width: usize,
    pub height: usize,
    pub stride: usize,
}

#[derive(Debug)]
pub struct BootData {
    pub frame_buffer: FrameBufferData,
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

impl BootData {
    pub fn new() -> BootData {
        BootData {
            frame_buffer: FrameBufferData::new(),
        }
    }
}