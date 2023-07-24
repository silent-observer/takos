use core::pin::Pin;

use async_trait::async_trait;

use alloc::boxed::Box;
use spin::Mutex;
use x86_64::{structures::paging::Translate, VirtAddr};

#[async_trait(?Send)]
pub trait UsbController {
    fn initialize(&self);
    async fn initialize_devices(&self);
    async fn run(&self);
}

pub trait MemoryInterface {
    fn to_physical(&self, virt_addr: u64) -> Option<u64>;
    fn allocate<T: Unpin>(&self, init: T) -> (u64, Pin<&mut T>); // This leaks memory!
    fn deallocate<T: Unpin>(&self, ptr: Pin<&mut T>);
}

impl<T: Translate> MemoryInterface for Mutex<T> {
    fn to_physical(&self, virt_addr: u64) -> Option<u64> {
        Some(self.lock().translate_addr(VirtAddr::new(virt_addr))?.as_u64())
    }

    fn allocate<ObjectType: Unpin>(&self, init: ObjectType) -> (u64, Pin<&mut ObjectType>) {
        let data = Box::leak(Box::new(init));
        let addr = self.to_physical(data as *mut _ as u64).unwrap();
        (addr, Pin::new(data))
    }

    fn deallocate<ObjectType: Unpin>(&self, ptr: Pin<&mut ObjectType>) {
        let raw = ptr.get_mut() as *mut _;
        let b = unsafe{Box::from_raw(raw)};
        drop(b)
    }
}