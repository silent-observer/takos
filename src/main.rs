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
    
    println!("Hello world!");
    println!("This is testing!");
    println!("{}", CAT);

    println!("Free memory regions:");
    for region in boot_data.free_memory_map.iter() {
        println!("{:016X}-{:016X}", region.start, region.end());
    }

    let mut v1 = vec![1, 2, 3, 4, 5];
    let v2 = vec![10, 11];
    assert_eq!(v1[0], 1);
    assert_eq!(v1[1], 2);
    assert_eq!(v1[2], 3);
    assert_eq!(v1[3], 4);
    assert_eq!(v1[4], 5);
    assert_eq!(v2[0], 10);
    assert_eq!(v2[1], 11);
    println!("V1: {:?}", v1);
    println!("V2: {:?}", v2);

    loop {}
}