use core::{pin::Pin, task::{Context, Poll}};

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, task::AtomicWaker, StreamExt};
use x86_64::instructions::port::Port;

use crate::{println, print, keyboard::commands::COMMAND_HANDLER};

use super::commands::Command;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100))
            .expect("Cannot initialize scancode queue twice!");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE.try_get().expect("not initialized");
        
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            },
            None => Poll::Pending,
        }
    }
}

pub fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Ok(_) = queue.push(scancode) {
            WAKER.wake();
        } else {
            println!("WARNING: Scancode queue full!");
        }
    } else {
        println!("WARNING: Scancode queue uninitialized")
    }
}

pub async fn init_ps2_controller(scancodes: &mut ScancodeStream) {
    let mut control_port = Port::<u8>::new(0x64);
    let mut data_port = Port::<u8>::new(0x60);
    
    unsafe{ control_port.write(0x20) }; // Read Controller Configuration Byte 
    let ccb = scancodes.next().await.unwrap();
    print!("CCB: 0x{:02X}", ccb);
    
    let ccb_no_translation = ccb & !0x40;
    unsafe{ control_port.write(0x60) }; // Write Controller Configuration Byte 
    loop { // Wait until ready
        let status = unsafe{ control_port.read() };
        if status & 0x02 == 0 { break; }
    }
    unsafe {data_port.write(ccb_no_translation)};
    loop { // Wait until ready
        let status = unsafe{ control_port.read() };
        if status & 0x02 == 0 { break; }
    }
}

pub async fn keyboard_driver() {
    let mut scancodes = ScancodeStream::new();
    
    init_ps2_controller(&mut scancodes).await;
    
    COMMAND_HANDLER.lock().send_command(Command::SetScanCodeSet(3));
    while let Some(scancode) = scancodes.next().await {
        print!("{:02X} ", scancode);
        COMMAND_HANDLER.lock().handle_scancode(scancode);
    }
}