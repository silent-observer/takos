use crate::xhci::trb::Trb;
pub trait UsbController {
    fn initialize(&mut self);
    fn poll_event(&self) -> &Trb;
}