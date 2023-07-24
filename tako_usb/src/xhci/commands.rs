use core::{future::Future, pin::Pin, task::{Context, Poll}};

use alloc::boxed::Box;
use futures::channel::oneshot::{Receiver, self};
use log::info;

use crate::controller::MemoryInterface;

use super::{trb::{Trb, TrbType, AddressDeviceCommandTrb}, contexts::InputContext};
use super::Xhci;

pub struct PendingEventFuture(Receiver<Trb>);

impl Future for PendingEventFuture {
    type Output = Trb;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().0).poll(cx).map(|trb| trb.unwrap())
    }
}

impl<Mem: MemoryInterface + 'static> Xhci<Mem> {
    pub fn handle_event_notification(&self, trb: &Trb) {
        let addr = trb.parameter;
        let trb_type = trb.trb_type();
        if let Some(sender) = self.pending_event_senders.lock().remove(&(trb_type, addr)) {
            sender.send(*trb).unwrap();
        } else {
            info!("No one listened to the event {:X?}", trb);
        }
    }

    pub fn new_pending_event(&self, trb_type: TrbType, parameter: u64) -> PendingEventFuture {
        let (sender, receiver) = oneshot::channel();
        self.pending_event_senders.lock().insert((trb_type, parameter), sender);
        PendingEventFuture(receiver)
    }

    pub fn send_command(&self, trb: Trb) -> PendingEventFuture {
        let mut command_ring = self.command_ring.lock();
        let addr = command_ring.get_current_addr(self.mem);
        let future = self.new_pending_event(TrbType::CommandCompletionEvent, addr);

        command_ring.enqueue_trb(trb);
        self.registers.doorbell.ring_host();

        future
    }

    pub async fn send_address_device_command(&self, slot_id: u8, input_context: Box<InputContext>) -> Trb {
        let (addr, data) = self.mem.allocate(*input_context);
        let trb = AddressDeviceCommandTrb::new(slot_id, addr);
        let trb = trb.into();
        info!("TRB: {:X?}", trb);
        let response = self.send_command(trb).await;
        self.mem.deallocate(data);
        response
    }

    pub fn reset_port(&self, port: u8) -> PendingEventFuture {
        let parameter = (port as u64) << 24;
        let future = self.new_pending_event(TrbType::PortStatusChangeEvent, parameter);

        let portsc = self.registers.operational.portsc(port as usize).read();
        let write_portsc = portsc & 0xC3E0 | 0x10;
        self.registers.operational.portsc(port as usize).write(write_portsc);

        future
    }
}