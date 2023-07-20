use lazy_static::lazy_static;

use x86_64::{structures::{idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode}, tss::TaskStateSegment}, registers::control::Cr2, VirtAddr};

use crate::println;
use crate::gdt::DOUBLE_FAULT_IST_INDEX;

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT ({:?})\n{:#?}", error_code, stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    let addr = Cr2::read();
    panic!("EXCEPTION: PAGE FAULT at {:?}, {:?}\n{:#?}\n", addr, error_code, stack_frame);
}

extern "x86-interrupt" fn gpf_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: GENERAL PROTECTION FAULT ({:?})\n{:#?}", error_code, stack_frame);
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
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}