#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

#[cfg(test)]
use core::panic::PanicInfo;

use alloc::string::ToString;
use allocator::frame_allocator::init_frame_allocator;
use console::init_writer;
use display::{FrameBuffer, ColorRGB};
use gdt::init_gdt;
use interrupts::init_idt;
use ::log::info;
use paging::{init_pat, unmap_loader_code};
use pic::init_pics;
use takobl_api::BootData;

use crate::pci::init_pci;

pub mod text;
pub mod display;
pub mod console;
pub mod interrupts;
pub mod allocator;
pub mod paging;
mod gdt;
mod pic;
pub mod keyboard;
mod pci;
mod log;

pub fn init(boot_data: &BootData) {
    init_gdt();
    init_idt();
    init_pat();
    init_frame_allocator(boot_data.free_memory_map.clone());

    let frame_buffer = FrameBuffer::new(&boot_data.frame_buffer);
    frame_buffer.fill(ColorRGB::from_hex(0x000000));
    init_writer(frame_buffer);
    crate::log::init().expect("Couldn't initialize logger");

    let image_device_path = boot_data.image_device_path.to_string();

    unmap_loader_code(boot_data.loader_code);

    init_pics();
    x86_64::instructions::interrupts::enable();

    info!("Image device path: {}", image_device_path);
    init_pci();
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