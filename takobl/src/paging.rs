use alloc::vec;
use log::info;
use takobl_api::{PHYSICAL_MEMORY_OFFSET, KernelStackData, FreeMemoryMap, FreeMemoryRegion};
use uefi::{table::boot::{MemoryMap, PAGE_SIZE, MemoryType, AllocateType}, prelude::BootServices};
use x86_64::{PhysAddr, structures::paging::{PageTableFlags, PageTable}};

struct PageTableBuilder {
    count1: u16,
    count2: u16,
    count3: u16,

    prev_address: Option<u64>,
    allocated_tables: u64,
    memory_size: u64,
}

const KERNEL_STACK_SIZE: usize = 5;
const KERNEL_STACK_END: u64 = 0x1_0000_0000;
const KERNEL_STACK_START: u64 = KERNEL_STACK_END - (KERNEL_STACK_SIZE * PAGE_SIZE) as u64;
const KERNEL_STACK_GUARD_PAGE: u64 = KERNEL_STACK_START - PAGE_SIZE as u64;

impl PageTableBuilder {
    fn new() -> Self {
        PageTableBuilder {
            count1: 0,
            count2: 0,
            count3: 0,

            prev_address: None,
            allocated_tables: 1,
            memory_size: 0,
        }
    }

    // Only works if pages are sorted!
    fn count_page(&mut self, virtual_address: u64, huge_page: bool) {
        if let Some(prev_address) = self.prev_address {
            assert!(prev_address <= virtual_address, "Pages must be counted in a sorted order!");
            let diff = virtual_address ^ prev_address;
            if diff & 0xFF80_0000_0000 != 0 {
                if !huge_page {
                    self.count1 += 1;
                    self.count2 += 1;
                    //info!("+2 for {:016X}", virtual_address);
                } else {
                    //info!("+1 for {:016X}", virtual_address);
                }
                self.count3 += 1;
            } else if diff & 0x007F_C000_0000 != 0 {
                if !huge_page {
                    self.count1 += 1;
                    self.count2 += 1;
                    //info!("+2 for {:016X}", virtual_address);
                }
            } else if diff & 0x0000_3FE0_0000 != 0 {
                self.count1 += 1;
                //info!("+1 for {:016X}", virtual_address);
            }
        } else {
            self.count1 += 1;
            self.count2 += 1;
            self.count3 += 1;
            //info!("+3 for {:016X}", virtual_address);
        }
        self.prev_address = Some(virtual_address);
    }
    
    fn count_span(&mut self, start_address: u64, page_count: u64) {
        for i in 0..page_count {
            self.count_page(start_address + i * PAGE_SIZE as u64, false);
        }
    }

    // Map must be sorted!
    fn count(&mut self, map: &MemoryMap) {
        // Identity map all the pages in the memory map
        for entry in map.entries() {
            match entry.ty {
                MemoryType::LOADER_CODE |
                MemoryType::LOADER_DATA |
                MemoryType::RUNTIME_SERVICES_CODE |
                MemoryType::RUNTIME_SERVICES_DATA |
                // MemoryType::BOOT_SERVICES_CODE |
                // MemoryType::BOOT_SERVICES_DATA |
                MemoryType::ACPI_NON_VOLATILE |
                MemoryType::PAL_CODE => {
                    self.count_span(entry.phys_start, entry.page_count);
                    self.memory_size = entry.phys_start + entry.page_count * PAGE_SIZE as u64;
                }
                MemoryType::RESERVED => {}
                _ => {
                    self.memory_size = entry.phys_start + entry.page_count * PAGE_SIZE as u64;
                }
            }
        }

        // Map kernel stack
        self.count_span(KERNEL_STACK_GUARD_PAGE, KERNEL_STACK_SIZE as u64 + 1);

        // Map all physical memory at PHYSICAL_MEMORY_OFFSET
        let total_pages = self.memory_size / PAGE_SIZE as u64;
        let huge_pages = (total_pages + 512 * 512 - 1) / (512 * 512);
        const HUGE_PAGE_SIZE: u64 = 4096 * 512 * 512;
        for i in 0..huge_pages {
            self.count_page(PHYSICAL_MEMORY_OFFSET + i * HUGE_PAGE_SIZE, true);
        }
    }

    fn get_or_create_table(&mut self, base_addr: u64, table_addr: u64, index: usize, new_flags: PageTableFlags) -> u64 {

        let table = unsafe {(table_addr as *mut PageTable).as_mut().unwrap()};
        if table[index].is_unused() {
            let new_table_addr = base_addr + self.allocated_tables * PAGE_SIZE as u64;
            if !PhysAddr::new(new_table_addr).is_aligned(4096u64) {
                //info!("{:08X}", new_table_addr);
            }
            assert!(PhysAddr::new(new_table_addr).is_aligned(4096u64));
            table[index].set_addr(PhysAddr::new(new_table_addr), new_flags);
            //info!("Base {:08X}", base_addr);
            //info!("Allocated table {} for {:08X}", self.allocated_tables, table_addr);
            self.allocated_tables += 1;
        }
        
        table[index].addr().as_u64()
    }

    fn set_page(&mut self, physical_address: u64, virtual_address: u64, flags: PageTableFlags, tables_addr: u64) {
        let p4_index = (virtual_address >> 39) & 0x1FF;
        let p3_index = (virtual_address >> 30) & 0x1FF;
        let p2_index = (virtual_address >> 21) & 0x1FF;
        let p1_index = (virtual_address >> 12) & 0x1FF;

        let p4_flags = flags.difference(PageTableFlags::HUGE_PAGE);
        let p3_table = self.get_or_create_table(tables_addr, tables_addr, p4_index as usize, p4_flags);
        let (final_table, index) = if flags.contains(PageTableFlags::HUGE_PAGE) {
            (p3_table, p3_index)
        } else {
            let p2_table = self.get_or_create_table(tables_addr, p3_table, p3_index as usize, flags);
            let p1_table = self.get_or_create_table(tables_addr, p2_table, p2_index as usize, flags);
            (p1_table, p1_index)
        };
        let table = unsafe {(final_table as *mut PageTable).as_mut().unwrap()};
        assert!(PhysAddr::new(physical_address).is_aligned(4096u64));
        //info!("Set addr: {:08X}", physical_address);
        table[index as usize].set_addr(PhysAddr::new(physical_address), flags);
    }

    fn set_span(&mut self, physical_address: u64, virtual_address: u64, page_count: u64, flags: PageTableFlags, tables_addr: u64) {
        for i in 0..page_count {
            self.set_page(physical_address + i * PAGE_SIZE as u64, virtual_address + i * PAGE_SIZE as u64, flags, tables_addr);
        }
    }

    fn allocate(&mut self, map: &MemoryMap, bs: &BootServices, stack_start: u64) -> PhysAddr {
        let total_table_count = self.count1 + self.count2 + self.count3 + 1;
        info!("Total page tables: {}", total_table_count);
        // Identity map all the pages in the memory map
        let addr = bs.allocate_pages(AllocateType::AnyPages, 
                MemoryType::LOADER_DATA, 
                total_table_count as usize)
            .expect("Failed to allocate pages");
        unsafe {
            core::ptr::write_bytes(addr as *mut u8, 0, total_table_count as usize * PAGE_SIZE);
        }

        const IDENTITY_MAP_FLAGS: PageTableFlags = PageTableFlags::PRESENT.union(PageTableFlags::WRITABLE);
        for entry in map.entries() {
            match entry.ty {
                MemoryType::LOADER_CODE |
                MemoryType::LOADER_DATA |
                MemoryType::RUNTIME_SERVICES_CODE |
                MemoryType::RUNTIME_SERVICES_DATA |
                // MemoryType::BOOT_SERVICES_CODE |
                // MemoryType::BOOT_SERVICES_DATA |
                MemoryType::ACPI_NON_VOLATILE |
                MemoryType::PAL_CODE =>
                    self.set_span(entry.phys_start,
                        entry.phys_start,
                        entry.page_count,
                        IDENTITY_MAP_FLAGS, 
                        addr),
                _ => {}
            }
            
        }

        const STACK_FLAGS: PageTableFlags = PageTableFlags::PRESENT
            .union(PageTableFlags::WRITABLE)
            .union(PageTableFlags::NO_EXECUTE);

        // Map kernel stack
        self.set_span(stack_start,
            KERNEL_STACK_START,
            KERNEL_STACK_SIZE as u64,
            STACK_FLAGS,
            addr);

        // Map all physical memory at PHYSICAL_MEMORY_OFFSET
        let total_pages = self.memory_size / PAGE_SIZE as u64;
        let huge_pages = (total_pages + 512 * 512 - 1) / (512 * 512);
        const HUGE_PAGE_FLAGS: PageTableFlags = PageTableFlags::PRESENT
            .union(PageTableFlags::WRITABLE)
            .union(PageTableFlags::NO_EXECUTE)
            .union(PageTableFlags::HUGE_PAGE);
        const HUGE_PAGE_SIZE: u64 = 4096 * 512 * 512;
        for i in 0..huge_pages {
            let physical_address = i * HUGE_PAGE_SIZE;
            let virtual_address = PHYSICAL_MEMORY_OFFSET + physical_address;
            //info!("{:08X} -> {:08X}", virtual_address, physical_address);
            self.set_page(physical_address, virtual_address, HUGE_PAGE_FLAGS, addr);
        }

        for i in [0, 1, 2, 5, 6] {
            let ptr = (addr + i * PAGE_SIZE as u64) as *mut PageTable;
            let table = unsafe {ptr.as_ref().unwrap()};
            info!("Table {} at {:?}:", i, ptr);
            for j in 0..512 {
                if !table[j].is_unused() {
                    info!("    {}: {:08X}, {:?}", j, table[j].addr().as_u64(), table[j].flags());
                }
            }
        }
        info!("Total pages: {}", total_pages);
        info!("Total huge pages: {}", huge_pages);

        PhysAddr::new(addr)
    }
}

fn create_free_memory_map(map: &MemoryMap<'_>) -> FreeMemoryMap {
    let mut result = FreeMemoryMap::new();
    let mut prev_region: Option<FreeMemoryRegion> = None;
    for entry in map.entries() {
        match entry.ty {
            MemoryType::BOOT_SERVICES_CODE |
            MemoryType::BOOT_SERVICES_DATA |
            MemoryType::CONVENTIONAL => {
                if let Some(ref mut prev) = prev_region {
                    let prev_end = prev.end();
                    if prev_end == entry.phys_start {
                        prev.pages += entry.page_count;
                    } else{
                        result.add(*prev);
                        *prev = FreeMemoryRegion {
                            start: entry.phys_start,
                            pages: entry.page_count
                        }
                    }
                } else {
                    prev_region = Some(FreeMemoryRegion {
                        start: entry.phys_start,
                        pages: entry.page_count
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

pub fn create_page_table(bs: &BootServices) -> (PhysAddr, KernelStackData, FreeMemoryMap) {
    let memory_map_size = bs.memory_map_size();
    info!("Memory map size: {}", memory_map_size.map_size);
    let buffer_size = memory_map_size.map_size + 4 * memory_map_size.entry_size;
    let mut buffer = vec![0u8; buffer_size];
    let memory_map = {
        let mut x = bs.memory_map(&mut buffer)
            .expect("Couldn't get memory map");
        x.sort();
        x
    };

    let mut free_memory_map = create_free_memory_map(&memory_map);

    let stack_start_physical = bs.allocate_pages(AllocateType::AnyPages, 
        MemoryType::LOADER_DATA,
        KERNEL_STACK_SIZE)
    .expect("Couldn't allocate stack");
    free_memory_map.remove(&FreeMemoryRegion { start: stack_start_physical, pages: KERNEL_STACK_SIZE as u64});
    
    let mut builder = PageTableBuilder::new();
    builder.count(&memory_map);
    let page_table = builder.allocate(&memory_map, bs, stack_start_physical);
    let stack_data = KernelStackData {
        stack_start: KERNEL_STACK_START,
        stack_end: KERNEL_STACK_END,
        guard_page: KERNEL_STACK_GUARD_PAGE,
    };
    (page_table, stack_data, free_memory_map)
}