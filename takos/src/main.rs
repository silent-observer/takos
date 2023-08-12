#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::{arch::asm, panic::PanicInfo};

extern crate alloc;

use alloc::string::String;
use log::info;
use takobl_api::BootData;

use tako_async::{
    executor::Executor,
    timer::{timer_executor, Timer},
    Task,
};
use takos::keyboard::{keyboard_driver, KeyboardEvent};
use takos::{console::console_scroll_handler, RAMDISK_FILESYSTEM};
use takos::{hlt_loop, println};
use takos::{keyboard::get_keyboard_event_receiver, multitask::scheduler::SCHEDULER};

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

async fn print_numbers() {
    for i in 0..100 {
        println!("async number: {}", i);
        Timer::new(10).await;
    }
}

async fn print_keyboard_events(receiver: Receiver<KeyboardEvent>) {
    while let Some(key_event) = receiver.recv().await {
        println!("Key event: {:?}", key_event);
    }
}

fn empty_task() -> ! {
    info!("From child thread 1");
    SCHEDULER.lock().switch_to_task(0);
    info!("From child thread 2");
    loop {
        unsafe { asm!("nop") };
    }
}

#[export_name = "_start"]
pub extern "C" fn _start(boot_data: &'static mut BootData) -> ! {
    takos::init(boot_data);

    #[cfg(debug_assertions)]
    unsafe {
        asm!("2: jmp 2b");
    }

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

    // println!("/:");
    // for (i, entry) in RAMDISK_FILESYSTEM
    //     .get()
    //     .unwrap()
    //     .dir_iter("/")
    //     .unwrap()
    //     .enumerate()
    // {
    //     println!("{}: {:?}", i, entry);
    // }

    // println!("/efi:");
    // for (i, entry) in RAMDISK_FILESYSTEM
    //     .get()
    //     .unwrap()
    //     .dir_iter("/efi")
    //     .unwrap()
    //     .enumerate()
    // {
    //     println!("{}: {:?}", i, entry);
    // }

    // println!("/efi/boot:");
    // for (i, entry) in RAMDISK_FILESYSTEM
    //     .get()
    //     .unwrap()
    //     .dir_iter("/efi/boot")
    //     .unwrap()
    //     .enumerate()
    // {
    //     println!("{}: {:?}", i, entry);
    // }

    // println!("/test.txt:");
    // println!(
    //     "{}",
    //     String::from_utf8(
    //         RAMDISK_FILESYSTEM
    //             .get()
    //             .unwrap()
    //             .read_file("/test.txt")
    //             .unwrap()
    //     )
    //     .unwrap()
    // );

    let task_id = SCHEDULER.lock().new_task(5, empty_task);
    info!("Created child thread");
    SCHEDULER.lock().switch_to_task(task_id);
    info!("From main thread");
    SCHEDULER.lock().switch_to_task(task_id);

    let mut executor = Executor::new();
    executor.spawn(Task::new(timer_executor()));
    // executor.spawn(Task::new(print_numbers()));
    executor.spawn(Task::new(keyboard_driver()));
    executor.spawn(Task::new(console_scroll_handler()));
    // let reciever = get_keyboard_event_receiver();
    // executor.spawn(Task::new(print_keyboard_events(reciever)));
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
