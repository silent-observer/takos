#![no_std]
#![no_main]

mod text;
pub mod display;
mod console;

use core::{panic::PanicInfo, fmt::Write};

use console::{init_writer, WRITER};
use display::{FrameBuffer, ColorRGB};
use takobl_api::BootData;

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
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
    let frame_buffer = FrameBuffer::new(&boot_data.frame_buffer);
    frame_buffer.fill(ColorRGB::from_hex(0x000000));
    init_writer(frame_buffer);
    
    println!("Hello world!");
    println!("This is testing!");
    println!("{}", CAT);

    loop {}
}