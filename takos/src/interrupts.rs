use lazy_static::lazy_static;

use x86_64::instructions::port::Port;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::console::WRITER;
use crate::{println, pic::{MASTER_PIC_OFFSET, PICS}};
use crate::gdt::DOUBLE_FAULT_IST_INDEX;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = MASTER_PIC_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    if WRITER.lock().frame_buffer().is_init() {
       panic!("EXCEPTION: DOUBLE FAULT ({:?})\n{:#?}", error_code, stack_frame);
    } else {
        loop {}
    }
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    let addr = Cr2::read();
    panic!("EXCEPTION: PAGE FAULT at {:?}, {:?}\n{:#?}\n", addr, error_code, stack_frame);
}

extern "x86-interrupt" fn gpf_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: GENERAL PROTECTION FAULT ({:?})\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    crate::async_task::timer::tick();
    //print!(".");
    PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
}

extern "x86-interrupt" fn keyboard_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::<u8>::new(0x60);
    let scancode = unsafe{port.read()};
    crate::keyboard::add_scancode(scancode);
    PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
}

lazy_static!{
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault.set_handler_fn(gpf_handler);
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}