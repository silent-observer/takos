use core::{future::Future, pin::Pin, task::{Context, Poll}};

use alloc::collections::BTreeMap;
use futures::channel::oneshot::{Receiver, Sender};
use spin::Mutex;
use x86_64::structures::paging::Translate;

use super::{Xhci, trb::{Trb, EventRing}};

struct CommandFuture(Receiver<Trb>);

impl CommandFuture {
}

impl Future for CommandFuture {
    type Output = Trb;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().0).poll(cx).map(|trb| trb.unwrap())
    }
}

impl<T: Translate> Xhci<T> {
    async fn handle_events(event_ring: &mut EventRing, pending_command_senders: &Mutex<BTreeMap<u64, Sender<Trb>>>) {
        loop {
            while event_ring.has_event() {
                let trb = event_ring.current_event();
                if trb.trb_type() == 33 {
                    let addr = trb.parameter;
                    if let Some(sender) = pending_command_senders.lock().remove(&addr) {
                        sender.send(*trb).unwrap();
                    }
                }
                event_ring.advance();
            }
        }
    }
}