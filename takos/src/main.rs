#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

extern crate alloc;


use takobl_api::BootData;

use takos::{println, hlt_loop, async_task::{executor::Executor, Task}, keyboard::{keyboard_driver, KeyboardEvent}};
use takos::keyboard::get_keyboard_event_receiver;
use thingbuf::mpsc::Receiver;

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

async fn print_number() {
    println!("async number: 42");

}

async fn print_keyboard_events(receiver: Receiver<KeyboardEvent>) {
    while let Some(key_event) = receiver.recv().await {
        println!("Key event: {:?}", key_event);
    }
}

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

    let mut executor = Executor::new();
    executor.spawn(Task::new(print_number()));
    executor.spawn(Task::new(keyboard_driver()));
    let reciever = get_keyboard_event_receiver();
    executor.spawn(Task::new(print_keyboard_events(reciever)));
    executor.run();

    // println!("Allocating more memory!");
    // for i in 0..10000 {
    //   let frame = FRAME_ALLOCATOR.lock().allocate_frame().expect("Couldn't allocate");
    //   let addr = 0x4000_1000_0000 + i * 0x1000;
    //   map_writable_page(addr, frame);
    // }
    // println!("Success!");

    // hlt_loop();
}