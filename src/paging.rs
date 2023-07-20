use lazy_static::lazy_static;
use spin::Mutex;
use takobl_api::PHYSICAL_MEMORY_OFFSET;
use x86_64::VirtAddr;
use x86_64::registers::control::Cr3;
use x86_64::registers::model_specific::Msr;
use x86_64::structures::paging::{OffsetPageTable, PageTable, PhysFrame, Size4KiB};

lazy_static!{
    pub static ref PAGE_TABLE: Mutex<OffsetPageTable<'static>> = unsafe {
        let (page_table_addr, _) = Cr3::read();
        let page_table = page_table_addr.start_address().as_u64() + PHYSICAL_MEMORY_OFFSET;
        let page_table = page_table as *mut PageTable;
        let page_table: &'static mut PageTable = page_table.as_mut().unwrap();
        Mutex::new(OffsetPageTable::new(page_table, VirtAddr::new(PHYSICAL_MEMORY_OFFSET)))
    };
}

pub fn map_writable_page(virtual_address: u64, frame: PhysFrame) {
    use x86_64::structures::paging::{Mapper, Page, PageTableFlags};
    use crate::allocator::frame_allocator::FRAME_ALLOCATOR;

    unsafe {
        PAGE_TABLE.lock().map_to(
            Page::from_start_address(VirtAddr::new(virtual_address)).unwrap(),
            frame,
            PageTableFlags::PRESENT.union(PageTableFlags::WRITABLE),
            &mut *FRAME_ALLOCATOR.lock()).expect("Failed to map").flush();
    }
}

pub fn init_pat() {
    let mut pat = Msr::new(0x277);
    unsafe {
        pat.write(0x00_07_04_06_00_07_01_06);
    }
}

#[test_case]
fn test_page_table() {
    use crate::allocator::frame_allocator::FRAME_ALLOCATOR;
    use crate::{print, println};
    use x86_64::structures::paging::{Mapper, FrameAllocator, PageTableFlags, Page};
    print!("test_page_table... ");

    let frame = FRAME_ALLOCATOR.lock().allocate_frame().unwrap();
    const ADDR: u64 = 0xABCDE000;
    map_writable_page(ADDR, frame);
    
    let ptr = ADDR as *mut u8;
    unsafe {
       *ptr = 42;
    }
    let data = unsafe{*ptr};
    assert_eq!(data, 42);

    unsafe {
        *ptr = 123;
    }
    let data = unsafe{*ptr};
    assert_eq!(data, 123);
    
    println!("[ok]");
}