#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

extern crate alloc;

use alloc::vec;
use takobl_api::BootData;

use takos::println;

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Kernel Panic: {}", info);
    loop {}
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
    
    // println!("Hello world!");
    // println!("This is testing!");
    // println!("{}", CAT);

    // println!("Free memory regions:");
    // for region in boot_data.free_memory_map.iter() {
    //     println!("{:016X}-{:016X}", region.start, region.end());
    // }

    loop {}
}