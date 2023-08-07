use core::task::{Context, Poll, Waker};

use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use thingbuf::StaticThingBuf;

use super::{Task, TaskId};

const QUEUE_CAPACITY: usize = 100;

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<StaticThingBuf<TaskId, QUEUE_CAPACITY>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<StaticThingBuf<TaskId, QUEUE_CAPACITY>>) -> Waker {
        Waker::from(Arc::new(Self {
            task_id,
            task_queue,
        }))
    }
    fn wake_task(&self) {
        self.task_queue
            .as_ref()
            .push(self.task_id)
            .expect("Task queue full!");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<StaticThingBuf<TaskId, QUEUE_CAPACITY>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(StaticThingBuf::new()),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task_id, task).is_some() {
            panic!("Duplicate task IDs!");
        }
        self.task_queue.push(task_id).expect("Queue full!");
    }

    fn run_ready_tasks(&mut self) {
        while let Some(task_id) = self.task_queue.pop() {
            let task = match self.tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue,
            };

            let waker = self
                .waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, self.task_queue.clone()));
            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    self.tasks.remove(&task_id);
                    self.waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts;
        interrupts::disable();
        if self.task_queue.is_empty() {
            interrupts::enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }
}
