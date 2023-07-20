#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

use takobl_api::BootData;

use takos::{println, allocator::frame_allocator::FRAME_ALLOCATOR, paging::PAGE_TABLE};
use x86_64::{structures::paging::{FrameAllocator, FrameDeallocator, Mapper, Page, PageTableFlags}, VirtAddr};

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

    let frame_1 = FRAME_ALLOCATOR.lock().allocate_frame().unwrap();
    println!("Frame 1 allocated: {:?}", frame_1);
    let addr = VirtAddr::new(0xABCDE000);
    unsafe {
        PAGE_TABLE.lock().map_to(
            Page::from_start_address(addr).unwrap(),
            frame_1,
            PageTableFlags::PRESENT.union(PageTableFlags::WRITABLE),
            &mut *FRAME_ALLOCATOR.lock()).expect("Failed to map").flush();
    }
    println!("Frame 1 mapped to {:?}", addr);
    
    let ptr = addr.as_u64() as *mut u8;
    unsafe {
       *ptr = 42;
    }
    let data = unsafe{*ptr};
    println!("Read: {}", data);
    
    let frame_2 = FRAME_ALLOCATOR.lock().allocate_frame().unwrap();
    println!("Frame 2 allocated: {:?}", frame_2);

    unsafe {FRAME_ALLOCATOR.lock().deallocate_frame(frame_1)};
    println!("Frame 1 deallocated");

    let frame_3 = FRAME_ALLOCATOR.lock().allocate_frame().unwrap();
    println!("Frame 3 allocated: {:?}", frame_3);

    loop {}
}