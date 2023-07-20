use core::{pin::Pin, task::{Context, Poll}, sync::atomic::{AtomicBool, Ordering}};

use conquer_once::spin::OnceCell;
use thingbuf::StaticThingBuf;
use futures_util::{Stream, task::AtomicWaker, StreamExt};
use x86_64::instructions::port::Port;

use crate::{println, print, keyboard::decoder::keycode_decoder};

use super::commands;

static SCANCODE_QUEUE: StaticThingBuf<u8, 100> = StaticThingBuf::new();
static WAKER: AtomicWaker = AtomicWaker::new();
pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        static SCANCODE_STREAM_TAKEN: AtomicBool = AtomicBool::new(false);
        let prev = SCANCODE_STREAM_TAKEN.swap(true, Ordering::Relaxed);
        assert!(!prev);
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(scancode) = SCANCODE_QUEUE.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(cx.waker());
        match SCANCODE_QUEUE.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            },
            None => Poll::Pending,
        }
    }
}

pub fn add_scancode(scancode: u8) {
    if let Ok(_) = SCANCODE_QUEUE.push(scancode) {
        WAKER.wake();
    } else {
        println!("WARNING: Scancode queue full!");
    }
}

pub async fn init_ps2_controller(scancodes: &mut ScancodeStream) {
    let mut control_port = Port::<u8>::new(0x64);
    let mut data_port = Port::<u8>::new(0x60);
    
    unsafe{ control_port.write(0x20) }; // Read Controller Configuration Byte 
    let ccb = scancodes.next().await.unwrap();
    
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
    
    commands::set_scancode_set(&mut scancodes, 2).await.expect("Couldn't set scancode set");
    let scancode_set = commands::get_scancode_set(&mut scancodes).await.expect("Couldn't get scancode set");
    println!{"Scancode set {}", scancode_set};
    keycode_decoder(&mut scancodes).await;
    // while let Some(scancode) = scancodes.next().await {
    //     print!("{:02X} ", scancode);
    // }
}