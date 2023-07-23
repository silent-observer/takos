use async_trait::async_trait;

use alloc::boxed::Box;

#[async_trait(?Send)]
pub trait UsbController {
    fn initialize(&self);
    async fn initialize_devices(&self);
    async fn run(&self);
}