use lazy_static::lazy_static;
use takobl_api::PHYSICAL_MEMORY_OFFSET;
use x86_64::VirtAddr;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{OffsetPageTable, PageTable};

lazy_static!{
    pub static ref PAGE_TABLE: OffsetPageTable<'static> = unsafe {
        let (page_table_addr, _) = Cr3::read();
        let page_table = page_table_addr.start_address().as_u64() + PHYSICAL_MEMORY_OFFSET;
        let page_table = page_table as *mut PageTable;
        let page_table: &'static mut PageTable = page_table.as_mut().unwrap();
        OffsetPageTable::new(page_table, VirtAddr::new(PHYSICAL_MEMORY_OFFSET))
    };
}