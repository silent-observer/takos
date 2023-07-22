use core::{sync::atomic::{AtomicU64, Ordering}, pin::Pin, task::{Context, Poll}};

use alloc::{vec::Vec, collections::BTreeMap};
use futures_util::{task::AtomicWaker, Stream, StreamExt, Future};
use spin::Mutex;
use thingbuf::StaticThingBuf;

type TimerId = u64;

static TIMER_ID: AtomicU64 = AtomicU64::new(0);
static TIMER_COUNT: AtomicU64 = AtomicU64::new(0);
static TIMER_REGISTER_QUEUE: StaticThingBuf<(TimerId, u64), 16> = StaticThingBuf::new();
static TIMER_WAKERS: Mutex<BTreeMap<TimerId, AtomicWaker>> = Mutex::new(BTreeMap::new());
static WAKER: AtomicWaker = AtomicWaker::new();

fn new_timer_id(delay: u64) -> TimerId {
    let id = TIMER_ID.fetch_add(1, Ordering::Relaxed);
    let current_time = TIMER_COUNT.load(Ordering::Relaxed);
    TIMER_REGISTER_QUEUE.push((id, current_time + delay)).unwrap();
    id
}

pub fn tick() {
    TIMER_COUNT.fetch_add(1, Ordering::Relaxed);
    WAKER.wake();
}

struct TickerStream(u64);

impl Stream for TickerStream {
    type Item = u64;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let current = &mut self.get_mut().0;
        if TIMER_COUNT.load(Ordering::Relaxed) > *current {
            let time = TIMER_COUNT.load(Ordering::Relaxed);
            *current = time;
            Poll::Ready(Some(time))
        } else {
            WAKER.register(cx.waker());
            Poll::Pending
        }
    }
}

pub struct Timer(TimerId);

impl Future for Timer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let wakers = TIMER_WAKERS.lock();
        if let Some(waker) = wakers.get(&self.0) {
            waker.register(cx.waker());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

impl Timer {
    pub fn new(delay: u64) -> Self {
        let timer_id = new_timer_id(delay);
        TIMER_WAKERS.lock().insert(timer_id, AtomicWaker::new());
        Self(timer_id)
    }
}

pub async fn timer_executor() {
    let mut timers: Vec<(TimerId, u64)> = Vec::new();
    let mut ticker = TickerStream(0);
    loop {
        let current_time = ticker.next().await.unwrap();
        while let Some((timer_id, time)) = TIMER_REGISTER_QUEUE.pop() {
            timers.push((timer_id, time));
        }
        timers.retain(|&(timer_id, time_at)| {
            if time_at <= current_time {
                if let Some(waker) = TIMER_WAKERS.lock().remove(&timer_id) {
                    waker.wake();
                }
                false
            } else {
                true
            }
        });
    }
}