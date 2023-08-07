use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use log::info;
use spin::{Mutex, MutexGuard};
use x86_64::structures::paging::FrameAllocator;

use crate::paging::map_writable_page;

use super::frame_allocator::FRAME_ALLOCATOR;

const BLOCK_SIZES: &[u64] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];
const BLOCK_COUNTS: &[u64] = &[512, 256, 128, 64, 32, 16, 8, 4, 2];

const HEAP_START: u64 = 0xFFFF_D000_0000_0000;
const HEAP_SIZE: u64 = 128 * 1024 * 1024;
const HEAP_END: u64 = HEAP_START + HEAP_SIZE;

struct FreeListNode {
    next: Option<&'static mut FreeListNode>,
}

struct FreeList {
    first: Option<&'static mut FreeListNode>,
}
pub struct BlockAllocator {
    next_page_addr: u64,
    free_lists: [FreeList; BLOCK_SIZES.len()],
    page_free_list: FreeList,
}

impl BlockAllocator {
    const fn new() -> Self {
        const EMPTY: FreeList = FreeList { first: None };
        Self {
            next_page_addr: HEAP_START,
            free_lists: [EMPTY; BLOCK_SIZES.len()],
            page_free_list: EMPTY,
        }
    }

    fn get_new_page(&mut self) -> Option<u64> {
        if self.page_free_list.first.is_none() {
            if self.next_page_addr >= HEAP_END {
                return None;
            }

            let addr = self.next_page_addr;
            let frame = FRAME_ALLOCATOR.lock().allocate_frame()?;
            map_writable_page(addr, frame);
            self.next_page_addr += 0x1000;
            Some(addr)
        } else {
            let node = self.page_free_list.first.take()?;
            self.page_free_list.first = node.next.take();
            Some(node as *mut FreeListNode as u64)
        }
    }

    fn new_blocked_page(&mut self, size_index: usize) -> Option<()> {
        let page = self.get_new_page()?;

        let block_size = BLOCK_SIZES[size_index];
        let block_count = BLOCK_COUNTS[size_index];

        let mut prev_addr = self.free_lists[size_index].first.take();
        for i in (0..block_count).rev() {
            let block_addr = page + i * block_size;
            unsafe {
                let new_node = block_addr as *mut FreeListNode;
                core::ptr::write(new_node, FreeListNode { next: prev_addr });
                prev_addr = Some(new_node.as_mut().unwrap());
            };
        }
        self.free_lists[size_index].first = prev_addr;

        Some(())
    }

    fn allocate_block(&mut self, size_index: usize) -> Option<u64> {
        if self.free_lists[size_index].first.is_none() {
            self.new_blocked_page(size_index)?;
        }

        let node = self.free_lists[size_index].first.take()?;
        self.free_lists[size_index].first = node.next.take();

        Some(node as *mut FreeListNode as u64)
    }

    fn deallocate_block(&mut self, size_index: usize, block_addr: u64) {
        let node = unsafe { (block_addr as *mut FreeListNode).as_mut().unwrap() };
        node.next = self.free_lists[size_index].first.take();
        self.free_lists[size_index].first = Some(node);
    }

    fn deallocate_page(&mut self, page_addr: u64) {
        let node = unsafe { (page_addr as *mut FreeListNode).as_mut().unwrap() };
        node.next = self.page_free_list.first.take();
        self.page_free_list.first = Some(node);
    }

    fn allocate_big(&mut self, pages: u64) -> Option<u64> {
        if self.next_page_addr + (pages - 1) * 0x1000 >= HEAP_END {
            return None;
        }

        let addr = self.next_page_addr;
        for _ in 0..pages {
            let frame = FRAME_ALLOCATOR.lock().allocate_frame()?;
            map_writable_page(self.next_page_addr, frame);
            self.next_page_addr += 0x1000;
        }
        Some(addr)
    }

    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        // info!("Allocating {:?}", layout);
        assert!(layout.align() <= 2048);
        let addr = if layout.size() > 2048 {
            let pages = (layout.size() + 0xFFF) / 0x1000;
            self.allocate_big(pages as u64)
        } else {
            let size = layout.size().max(layout.align()) as u64;
            let size_index = BLOCK_SIZES.iter().position(|&s| s >= size).unwrap();
            self.allocate_block(size_index)
        };

        if let Some(addr) = addr {
            addr as *mut u8
        } else {
            null_mut()
        }
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        // info!("Deallocating {:?}", layout);
        assert!(layout.align() <= 2048);

        if layout.size() > 2048 {
            let pages = ((layout.size() + 0xFFF) / 0x1000) as u64;
            for i in (0..pages).rev() {
                let addr = ptr as u64 + i * 0x1000;
                self.deallocate_page(addr);
            }
        } else {
            let size = layout.size().max(layout.align()) as u64;
            let size_index = BLOCK_SIZES.iter().position(|&s| s >= size).unwrap();
            self.deallocate_block(size_index, ptr as u64);
        }
    }
}

pub struct Locked<T>(Mutex<T>);

impl<T> Locked<T> {
    const fn new(t: T) -> Self {
        Self(Mutex::new(t))
    }

    fn lock(&self) -> MutexGuard<T> {
        self.0.lock()
    }
}

unsafe impl GlobalAlloc for Locked<BlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.lock().dealloc(ptr, layout)
    }
}

#[global_allocator]
pub static ALLOCATOR: Locked<BlockAllocator> = Locked::new(BlockAllocator::new());
