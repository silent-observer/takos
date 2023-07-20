#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use allocator::frame_allocator::init_frame_allocator;
use console::init_writer;
use display::{FrameBuffer, ColorRGB};
use interrupts::init_idt;
use takobl_api::BootData;

pub mod text;
pub mod display;
pub mod console;
pub mod interrupts;
pub mod allocator;
pub mod paging;


pub fn init(boot_data: &BootData) {
    let frame_buffer = FrameBuffer::new(&boot_data.frame_buffer);
    frame_buffer.fill(ColorRGB::from_hex(0x000000));
    init_writer(frame_buffer);

    init_idt();
    init_frame_allocator(boot_data.free_memory_map.clone());
}