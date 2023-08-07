use alloc::vec::Vec;
use log::info;
use takobl_api::{FreeMemoryMap, MemoryRegion, PHYSICAL_MEMORY_OFFSET};
use uefi::{
    prelude::BootServices,
    table::boot::{AllocateType, MemoryMap, MemoryType},
};
use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, PageTable, PhysFrame, Size4KiB,
};
use x86_64::{
    structures::paging::{Page, PageTableFlags, Size1GiB},
    PhysAddr, VirtAddr,
};

struct UefiFrameAllocator<'a> {
    bs: &'a BootServices,
    free_memory_map: FreeMemoryMap,
}

impl<'a> UefiFrameAllocator<'a> {
    fn new(bs: &'a BootServices, memory_map: &MemoryMap<'_>) -> Self {
        let free_memory_map = create_free_memory_map(memory_map);
        Self {
            bs,
            free_memory_map,
        }
    }

    fn register(&mut self, start_addr: u64, pages: u64) {
        self.free_memory_map.remove(&MemoryRegion {
            start: start_addr,
            pages,
        });
    }

    fn allocate(&mut self, pages: u64) -> Option<u64> {
        let addr = self
            .bs
            .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1)
            .ok()?;
        self.register(addr, pages);
        Some(addr)
    }
}

fn create_free_memory_map(map: &MemoryMap<'_>) -> FreeMemoryMap {
    let mut result = FreeMemoryMap::new();
    let mut prev_region: Option<MemoryRegion> = None;
    for entry in map.entries() {
        match entry.ty {
            MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA
            | MemoryType::RUNTIME_SERVICES_CODE
            | MemoryType::RUNTIME_SERVICES_DATA
            | MemoryType::LOADER_CODE
            | MemoryType::LOADER_DATA
            | MemoryType::CONVENTIONAL => {
                if let Some(ref mut prev) = prev_region {
                    let prev_end = prev.end();
                    if prev_end == entry.phys_start {
                        prev.pages += entry.page_count;
                    } else {
                        result.add(*prev);
                        *prev = MemoryRegion {
                            start: entry.phys_start,
                            pages: entry.page_count,
                        }
                    }
                } else {
                    prev_region = Some(MemoryRegion {
                        start: entry.phys_start,
                        pages: entry.page_count,
                    })
                }
            }
            _ => {}
        }
    }
    if let Some(prev) = prev_region {
        result.add(prev);
    }
    result
}

fn get_memory_map<'a>(bs: &BootServices, buffer: &'a mut Vec<u8>) -> MemoryMap<'a> {
    let memory_map_size = bs.memory_map_size();
    info!("Memory map size: {}", memory_map_size.map_size);
    let buffer_size = memory_map_size.map_size + 4 * memory_map_size.entry_size;
    buffer.resize(buffer_size, 0);

    let mut memory_map = bs
        .memory_map(&mut buffer[..])
        .expect("Couldn't get memory map");
    memory_map.sort();
    memory_map
}

unsafe impl<'a> FrameAllocator<Size4KiB> for UefiFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let addr = self.allocate(1)?;
        PhysFrame::from_start_address(PhysAddr::new(addr)).ok()
    }
}

pub const KERNEL_STACK_GUARD_PAGE: u64 = 0xFFFF_FFFF_FFF0_0000;
pub const KERNEL_STACK_START: u64 = KERNEL_STACK_GUARD_PAGE + 0x1000;
pub const KERNEL_STACK_END: u64 = 0xFFFF_FFFF_FFFF_FFF0;
pub const KERNEL_STACK_PAGES: u64 = 0x100;

pub struct PageTableBuilder<'a> {
    pt: OffsetPageTable<'static>,
    frame_allocator: UefiFrameAllocator<'a>,
    loader_code: Option<MemoryRegion>,
}

impl<'a> PageTableBuilder<'a> {
    pub fn new(bs: &'a BootServices) -> Self {
        let page_table_addr = bs
            .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1)
            .unwrap();
        unsafe { bs.set_mem(page_table_addr as *mut u8, 0x1000, 0u8) };
        let page_table = unsafe { (page_table_addr as *mut PageTable).as_mut().unwrap() };
        let pt = unsafe { OffsetPageTable::new(page_table, VirtAddr::new(0)) };
        let mut buffer = Vec::new();
        let memory_map = get_memory_map(bs, &mut buffer);
        let frame_allocator = UefiFrameAllocator::new(bs, &memory_map);

        let mut result = PageTableBuilder {
            pt,
            frame_allocator,
            loader_code: None,
        };
        result.identity_map_loader_code(&memory_map);
        info!("Identity map... OK!");
        result
    }

    pub fn map_page(&mut self, virtual_addr: u64, physical_addr: u64, flags: PageTableFlags) {
        let page = Page::<Size4KiB>::from_start_address(VirtAddr::new(virtual_addr)).unwrap();
        let frame = PhysFrame::from_start_address(PhysAddr::new(physical_addr)).unwrap();
        unsafe {
            self.pt
                .map_to(page, frame, flags, &mut self.frame_allocator)
                .unwrap()
                .ignore()
        }
    }

    pub fn map_writeable_page(&mut self, virtual_addr: u64, physical_addr: u64, executable: bool) {
        let flags = PageTableFlags::PRESENT.union(PageTableFlags::WRITABLE);
        let flags = if executable {
            flags
        } else {
            flags.union(PageTableFlags::NO_EXECUTE)
        };
        self.map_page(virtual_addr, physical_addr, flags);
    }

    pub fn map_physical_mem(&mut self) {
        for i in 0..1024 {
            // 1 TB of physical memory (I wish...)
            const PAGE_1GB_SIZE: u64 = 1024 * 1024 * 1024;
            let phys_addr = PhysAddr::new(i * PAGE_1GB_SIZE);
            let virt_addr = VirtAddr::new(i * PAGE_1GB_SIZE + PHYSICAL_MEMORY_OFFSET);
            let page = Page::<Size1GiB>::from_start_address(virt_addr).unwrap();
            let frame = PhysFrame::<Size1GiB>::from_start_address(phys_addr).unwrap();
            let flags = PageTableFlags::PRESENT
                .union(PageTableFlags::WRITABLE)
                .union(PageTableFlags::NO_EXECUTE);
            unsafe {
                self.pt
                    .map_to(page, frame, flags, &mut self.frame_allocator)
                    .unwrap()
                    .ignore();
            }
        }
        info!("Offset physical memory map... OK!");
    }

    pub fn allocate_pages(&mut self, start_virtual_addr: u64, pages: u64, executable: bool) {
        let addr = self.frame_allocator.allocate(pages).unwrap();
        for i in 0..pages {
            let physical_addr = addr + i * 0x1000;
            let virtual_addr = start_virtual_addr + i * 0x1000;
            self.map_writeable_page(virtual_addr, physical_addr, executable);
        }
    }

    pub fn allocate_page(&mut self, virtual_addr: u64, executable: bool) -> u64 {
        let addr = self.frame_allocator.allocate(1).unwrap();
        self.map_writeable_page(virtual_addr, addr, executable);
        addr
    }

    pub fn allocate_stack(&mut self) {
        self.allocate_pages(KERNEL_STACK_START, KERNEL_STACK_PAGES - 1, false);
        info!("Kernel stack allocation... OK!");
    }

    fn identity_map_loader_code(&mut self, memory_map: &MemoryMap) {
        for entry in memory_map.entries() {
            match entry.ty {
                MemoryType::LOADER_CODE => {
                    info!(
                        "Identity mapping loader code: {:08X} ({} pages)",
                        entry.phys_start, entry.page_count
                    );
                    self.loader_code = Some(MemoryRegion {
                        start: entry.phys_start,
                        pages: entry.page_count,
                    });
                    for page in 0..entry.page_count {
                        let addr = entry.phys_start + page * 0x1000;
                        self.map_writeable_page(addr, addr, true);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn deconstruct(self) -> (OffsetPageTable<'static>, FreeMemoryMap, MemoryRegion) {
        (
            self.pt,
            self.frame_allocator.free_memory_map,
            self.loader_code.unwrap(),
        )
    }
}
