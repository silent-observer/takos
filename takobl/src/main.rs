#![no_main]
#![no_std]
#![feature(ascii_char)]

mod paging;

extern crate alloc;

use core::arch::asm;
use core::mem::{size_of, transmute};

use alloc::vec::Vec;
use alloc::format;
use alloc::{vec, string::String};

use elf::abi::{PT_LOAD, PF_X};
use elf::endian::AnyEndian;
use log::info;
use uefi::data_types::PhysicalAddress;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::fs::{self, Path};
use uefi::prelude::*;
use elf::ElfBytes;
use uefi::table::boot::MemoryType;
use takobl_api::{FrameBufferData, BootData, PHYSICAL_MEMORY_OFFSET};
use x86_64::structures::paging::{PageTable, OffsetPageTable};

use crate::paging::{KERNEL_STACK_END, PageTableBuilder};

#[entry]
fn main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();
    info!("Hello world testing 3!");
    //print_memory_map(&system_table);
    //test_filesystem(image_handle, &system_table);
    let mut page_table_builder = PageTableBuilder::new(system_table.boot_services());
    page_table_builder.map_physical_mem();
    page_table_builder.allocate_stack();
    let boot_data = allocate_boot_data(system_table.boot_services());
    let kernel_entry = load_kernel(image_handle, system_table.boot_services(), &mut page_table_builder);

    // info!("Boot Data ptr: {:?}", boot_data as *mut BootData);
    // info!("Boot Data: {:?}", boot_data);
    // print_memory_map(image_handle, &system_table);
    let (mut page_table, free_memory_map, loader_code) = page_table_builder.deconstruct();
    boot_data.free_memory_map = free_memory_map;
    boot_data.frame_buffer = get_gop_data(system_table.boot_services());
    boot_data.loader_code = loader_code;
    info!("Loading page table {:08X}!", page_table.level_4_table() as *mut PageTable as u64);
    info!("Page table {:?}", page_table.level_4_table()[0]);
    //let rip = x86_64::registers::read_rip();
    //info!("Rip: {:016X}", rip);
    //system_table.boot_services().stall(10_000_000);
    let _ = system_table.exit_boot_services();
    //info!("Success!", "{}");
    jump_to(kernel_entry, convert_boot_data(boot_data), &mut page_table);
}

#[allow(dead_code)]
fn test_filesystem(image_handle: Handle, system_table: &SystemTable<Boot>) {
    let mut fs = system_table.boot_services().get_image_file_system(image_handle)
        .expect("Couldn't get filesystem");
    let path = Path::new(cstr16!("test.txt"));
    let data = fs.read(path).expect("Couldn't read file");
    let s = String::from_utf8(data).unwrap();
    info!("File contents: {}", s);
}

fn load_kernel(image_handle: Handle, bs: &BootServices, page_table_builder: &mut PageTableBuilder)-> PhysicalAddress {
    let mut fs: fs::FileSystem<'_> = bs.get_image_file_system(image_handle)
        .expect("Couldn't get filesystem");
    let path = Path::new(cstr16!("kernel.elf"));
    let data = fs.read(path).expect("Couldn't read file");
    let elf = ElfBytes::<AnyEndian>::minimal_parse(data.as_slice())
        .expect("Couldn't parse elf");
    let segments = elf.segments().expect("Couldn't get segments");

    let mut allocated_pages: Vec<(u64, u64)> = Vec::new();

    for segment in segments {
        if segment.p_type != PT_LOAD { continue; }
        let file_size = segment.p_filesz as usize;
        let mem_size = segment.p_memsz as usize;
        let start_virt_address = segment.p_vaddr;
        let end_virt_address = segment.p_vaddr + segment.p_memsz;
        info!("Address: {:016X}-{:016X}, size: 0x{:X}", start_virt_address, end_virt_address, mem_size);
        let data = elf.segment_data(&segment).expect("Couldn't get segment data");

        let is_executable = segment.p_flags & PF_X != 0;
        
        let mut page_address = segment.p_vaddr & !0xFFF;
        let mut memory_offset = segment.p_vaddr & 0xFFF;
        let mut data_offset: usize = 0;
        while data_offset < mem_size {
            info!("data_offset: {:X}", data_offset);
            info!("page_address: {:X}", page_address);
            info!("memory_offset: {:X}", memory_offset);
            let data_left = if data_offset < file_size {file_size - data_offset} else {0x1000};
            let size = (0x1000 - memory_offset as usize).min(data_left);
            let physical_address = allocated_pages
                .iter()
                .find(|(virt, _)| *virt == page_address)
                .map(|(_, phys)| *phys)
                .unwrap_or_else(|| {
                    let new_page = page_table_builder.allocate_page(page_address, is_executable);
                    allocated_pages.push((page_address, new_page));
                    new_page
                });
            unsafe {
                let dest = (physical_address as *mut u8).add(memory_offset as usize);
                if data_offset < file_size {
                    let src = data.as_ptr().add(data_offset);
                    bs.memmove(dest, src, size);
                } else {
                    bs.set_mem(dest, size, 0u8);
                }
            }

            memory_offset += size as u64;
            data_offset += size;
            if memory_offset >= 0x1000 {
                page_address += 0x1000;
                memory_offset -= 0x1000;
            }
        }
        info!("Success!");
    }
    info!("Kernel loading into memory... OK!");
    elf.ehdr.e_entry
}

fn jump_to(addr: PhysicalAddress, boot_data: &'static mut BootData, page_table: &mut OffsetPageTable<'static>) -> ! {
    let cr3 = page_table.level_4_table() as *mut PageTable as u64;
    let stack_ptr = KERNEL_STACK_END;
    unsafe {
        asm!(
            "mov cr3, {}; mov rsp, {}; push 0; jmp {}",
            in(reg) cr3,
            in(reg) stack_ptr,
            in(reg) addr,
            in("rdi") boot_data as *const _ as usize,
        );
    }
    unreachable!();
}

fn get_gop_data(bt: &BootServices) -> FrameBufferData {
    info!("Getting handle");
    let handle = bt.get_handle_for_protocol::<GraphicsOutput>().expect("Couldn't get handle");
    info!("Getting GOP");
    let mut gop = bt
        .open_protocol_exclusive::<GraphicsOutput>(handle)
        .expect("Couldn't open protocol");
    
    //info!("Printing modes!");
    //let mode = gop.query_mode(0).unwrap();
    //gop.set_mode(&mode).unwrap();
    
    let current = gop.current_mode_info();
    info!("Current mode: {:?}", current);
    let (width, height) = current.resolution();
    let result = FrameBufferData {
        buffer_addr: gop.frame_buffer().as_mut_ptr(),
        width, height,
        stride: current.stride(),
    };
    info!("Frame buffer: {:?}", result);
    result
}

fn allocate_boot_data(bs: &BootServices) -> &'static mut BootData {
    let size = size_of::<BootData>();
    let boot_data = bs
        .allocate_pool(MemoryType::LOADER_DATA, size)
        .expect("Couldn't allocate memory");
    unsafe {
        let boot_data: *mut BootData = transmute(boot_data);
        core::ptr::write(boot_data, BootData::new());
        boot_data.as_mut().unwrap()
    }
}

fn convert_boot_data(boot_data: &'static mut BootData) -> &'static mut BootData {
    unsafe {
        let addr = boot_data as *mut BootData as *mut u8;
        let addr = addr.add(PHYSICAL_MEMORY_OFFSET as usize) as *mut BootData;
        addr.as_mut().unwrap()
    }
}

#[allow(dead_code)]
fn print_memory_map(image_handle: Handle, system_table: &SystemTable<Boot>) {
    let memory_map_size = system_table.boot_services().memory_map_size();
    info!("Memory map size: {}", memory_map_size.map_size);
    let buffer_size = memory_map_size.map_size + 4 * memory_map_size.entry_size;
    let mut buffer = vec![0u8; buffer_size];
    let memory_map = {
        let mut x = system_table.boot_services().memory_map(&mut buffer)
            .expect("Couldn't get memory map");
        x.sort();
        x
    };

    let mut str = String::new();

    for (i, entry) in memory_map.entries().enumerate() {
        // info!("{:3}: {:08X}..+{:X} (virt {:X}), type = {:?}",
        //     i,
        //     entry.phys_start,
        //     entry.page_count * 0x1000,
        //     entry.virt_start,
        //     entry.ty);
        str.push_str(&format!("{:3}: {:08X}..+{:X} (virt {:X}), type = {:?}\n",
            i,
            entry.phys_start,
            entry.page_count * 0x1000,
            entry.virt_start,
            entry.ty));
        //system_table.boot_services().stall(500_000);
    }
    info!("{}", str);
    //system_table.boot_services().stall(10_000_000);
    
    info!("Getting file system");
    let mut fs = system_table.boot_services().get_image_file_system(image_handle)
        .expect("Couldn't get filesystem");
    //fs.remove_file(cstr16!("test.txt")).expect("Couldn't delete file");
    let path = Path::new(cstr16!("test.txt"));
    fs.write(path, str.as_bytes()).expect("Couldn't write file");
}