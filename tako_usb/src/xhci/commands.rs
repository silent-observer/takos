use core::{future::Future, pin::Pin, task::{Context, Poll}};

use alloc::collections::BTreeMap;
use futures::channel::oneshot::{Receiver, Sender, self};
use log::info;
use spin::Mutex;
use tako_async::timer::Timer;
use x86_64::structures::paging::Translate;

use super::{Xhci, trb::{Trb, EventRing}};

pub struct CommandFuture(Receiver<Trb>);

impl Future for CommandFuture {
    type Output = Trb;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().0).poll(cx).map(|trb| trb.unwrap())
    }
}

impl<T: Translate> Xhci<T> {
    pub async fn handle_events(event_ring: &Mutex<EventRing>, pending_command_senders: &Mutex<BTreeMap<u64, Sender<Trb>>>) {
        loop {
            let mut event_ring = event_ring.lock();
            while event_ring.has_event() {
                let trb = event_ring.current_event();
                info!("Got event {:X?}!", trb);
                if trb.trb_type() == 33 {
                    let addr = trb.parameter;
                    if let Some(sender) = pending_command_senders.lock().remove(&addr) {
                        sender.send(*trb).unwrap();
                    }
                }
                event_ring.advance();
            }
            Timer::new(1).await;
        }
    }

    pub fn new_command(&self, trb: Trb) -> CommandFuture {
        let (sender, receiver) = oneshot::channel();
        let mut command_ring = self.command_ring.lock();
        let addr = command_ring.get_current_addr(&self.translator);
        self.pending_command_senders.lock().insert(addr, sender);
        command_ring.enqueue_trb(trb);
        self.registers.doorbell.ring_host();
        CommandFuture(receiver)
    }
}