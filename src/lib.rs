#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

#[cfg(test)]
use core::panic::PanicInfo;

use allocator::frame_allocator::init_frame_allocator;
use console::init_writer;
use display::{FrameBuffer, ColorRGB};
use gdt::init_gdt;
use interrupts::init_idt;
use paging::init_pat;
use pic::init_pics;
use takobl_api::BootData;

pub mod text;
pub mod display;
pub mod console;
pub mod interrupts;
pub mod allocator;
pub mod paging;
mod gdt;
mod pic;
pub mod keyboard;
pub mod async_task;

pub fn init(boot_data: &BootData) {
    init_gdt();
    init_idt();
    init_pat();
    init_frame_allocator(boot_data.free_memory_map.clone());

    let frame_buffer = FrameBuffer::new(&boot_data.frame_buffer);
    frame_buffer.fill(ColorRGB::from_hex(0x000000));
    init_writer(frame_buffer);

    init_pics();
    x86_64::instructions::interrupts::enable();
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}


#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

#[cfg(test)]
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    //exit_qemu(QemuExitCode::Success);
}

/// This function is called on panic.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[failed]");
    println!("Error: {}", info);
    hlt_loop();
}

#[cfg(test)]
#[export_name = "_start"]
pub extern "C" fn _start(boot_data: &'static mut BootData) -> ! {
    init(boot_data);
    println!("Testing!");
    #[cfg(test)]
    test_main();
    hlt_loop();
}