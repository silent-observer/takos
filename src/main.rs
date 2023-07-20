#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

extern crate alloc;

use alloc::vec;
use takobl_api::BootData;

use takos::{println, hlt_loop, allocator::frame_allocator::FRAME_ALLOCATOR, paging::map_writable_page};
use x86_64::structures::paging::FrameAllocator;

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Kernel Panic: {}", info);
    hlt_loop();
}

const CAT: &str = r"
             *     ,MMM8&&&.            *
                  MMMM88&&&&&    .
                 MMMM88&&&&&&&
     *           MMM88&&&&&&&&
                 MMM88&&&&&&&&
                 'MMM88&&&&&&'
                   'MMM8&&&'      *
          |\___/|
          )     (             .              '
         =\     /=
           )===(       *
          /     \
          |     |
         /       \
         \       /
  _/\_/\_/\__  _/_/\_/\_/\_/\_/\_/\_/\_/\_/\_
  |  |  |  |( (  |  |  |  |  |  |  |  |  |  |
  |  |  |  | ) ) |  |  |  |  |  |  |  |  |  |
  |  |  |  |(_(  |  |  |  |  |  |  |  |  |  |
  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |";

#[export_name = "_start"]
pub extern "C" fn _start(boot_data: &'static mut BootData) -> ! {
    takos::init(boot_data);

    // unsafe {
    //     *(0xdeadbeef as *mut u8) = 42;
    // }
    
    println!("Hello world!");
    // println!("This is testing!");
    println!("{}", CAT);

    println!("Free memory regions:");
    for region in boot_data.free_memory_map.iter() {
        println!("{:016X}-{:016X}", region.start, region.end());
    }

    // println!("Allocating more memory!");
    // for i in 0..10000 {
    //   let frame = FRAME_ALLOCATOR.lock().allocate_frame().expect("Couldn't allocate");
    //   let addr = 0x4000_1000_0000 + i * 0x1000;
    //   map_writable_page(addr, frame);
    // }
    // println!("Success!");

    hlt_loop();
}