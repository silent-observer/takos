use futures::Future;

use crate::xhci::trb::Trb;

use async_trait::async_trait;

use alloc::boxed::Box;

#[async_trait(?Send)]
pub trait UsbController {
    fn initialize(&mut self);
    async fn run(&mut self);
}