#![no_main]
#![no_std]

extern crate alloc;

use core::arch::asm;
use core::mem::{size_of, transmute};

use alloc::{vec, string::String};

use elf::abi::PT_LOAD;
use elf::endian::AnyEndian;
use log::info;
use uefi::data_types::PhysicalAddress;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::{fs::Path, table::boot::PAGE_SIZE};
use uefi::prelude::*;
use elf::ElfBytes;
use uefi::table::boot::{AllocateType, MemoryType};
use takobl_api::{FrameBufferData, BootData};

#[entry]
fn main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();
    info!("Hello world testing 3!");

    //print_memory_map(&system_table);
    //test_filesystem(image_handle, &system_table);
    let kernel_entry = load_kernel(image_handle, &system_table);
    let boot_data = allocate_boot_data(&system_table);

    boot_data.frame_buffer = get_gop_data(system_table.boot_services());

    info!("Boot Data ptr: {:?}", boot_data as *mut BootData);
    info!("Boot Data: {:?}", boot_data);

    info!("Jumping!");
    let _ = system_table.exit_boot_services();
    jump_to(kernel_entry, boot_data);
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

fn load_kernel(image_handle: Handle, system_table: &SystemTable<Boot>)-> PhysicalAddress {
    let mut fs = system_table.boot_services().get_image_file_system(image_handle)
        .expect("Couldn't get filesystem");
    let path = Path::new(cstr16!("kernel.elf"));
    let data = fs.read(path).expect("Couldn't read file");
    let elf = ElfBytes::<AnyEndian>::minimal_parse(data.as_slice())
        .expect("Couldn't parse elf");
    let segments = elf.segments().expect("Couldn't get segments");

    let mut page_start_index = None;
    let mut page_end_index = None;
    for segment in segments {
        if segment.p_type != PT_LOAD { continue; }
        let segment_start_index = segment.p_paddr / PAGE_SIZE as u64;
        let mem_size = segment.p_memsz as usize;
        let mem_pages = (mem_size + (PAGE_SIZE - 1)) / PAGE_SIZE;
        let segment_end_index = segment_start_index + mem_pages as u64;
        match page_start_index {
            Some(p) if segment_start_index < p => page_start_index = Some(segment_start_index),
            Some(_) => {}
            None => page_start_index = Some(segment_start_index),
        }
        match page_end_index {
            Some(p) if segment_end_index > p => page_end_index = Some(segment_end_index),
            Some(_) => {}
            None => page_end_index = Some(segment_end_index),
        }
    }

    let start_address = page_start_index.unwrap() * PAGE_SIZE as u64;
    let page_count = (page_end_index.unwrap() - page_start_index.unwrap()) as usize;
    system_table.boot_services()
        .allocate_pages(
            AllocateType::Address(start_address), 
            MemoryType::LOADER_DATA, 
            page_count)
        .expect("Couldn't allocate memory");

    for segment in segments {
        if segment.p_type != PT_LOAD { continue; }
        let file_size = segment.p_filesz as usize;
        let mem_size = segment.p_memsz as usize;
        let physical_address = segment.p_paddr;
        info!("Address: {}, size: {}", physical_address, mem_size);
        let data = elf.segment_data(&segment).expect("Couldn't get segment data");
        unsafe {
            let dest = physical_address as *mut u8;
            let src = data.as_ptr();
            system_table.boot_services().memmove(dest, src, file_size);
            if mem_size > file_size {
                system_table.boot_services().set_mem(dest.offset(file_size as isize), mem_size - file_size, 0u8);
            }
        }
    }
    elf.ehdr.e_entry
}

fn jump_to(addr: PhysicalAddress, boot_data: &'static mut BootData) -> ! {
    unsafe {
        asm!(
            "push 0; jmp {}",
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

fn allocate_boot_data(system_table: &SystemTable<Boot>) -> &'static mut BootData {
    let size = size_of::<BootData>();
    let boot_data = system_table.boot_services()
        .allocate_pool(MemoryType::LOADER_DATA, size)
        .expect("Couldn't allocate memory");
    unsafe {
        let boot_data: *mut BootData = transmute(boot_data);
        core::ptr::write(boot_data, BootData::new());
        boot_data.as_mut().unwrap()
    }
}

#[allow(dead_code)]
fn print_memory_map(system_table: &SystemTable<Boot>) {
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

    for (i, entry) in memory_map.entries().enumerate() {
        info!("{}: {:?}", i, entry);
        system_table.boot_services().stall(1_000_000);
    }
}