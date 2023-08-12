use core::arch::asm;
use core::pin::Pin;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;
use x86_64::registers::control::Cr3;

type TaskId = usize;
type Page = [u64; 512];

struct TaskData {
    kernel_stack_top: u64,
    cr3_value: u64,
    #[allow(unused)]
    stack: Pin<Box<[Page]>>,
}

pub struct Scheduler {
    tasks: Vec<TaskData>,
    current_task: TaskId,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current_task: 0,
        }
    }

    pub fn initialize(&mut self) {
        self.tasks = vec![TaskData {
            kernel_stack_top: 0,
            cr3_value: Cr3::read().0.start_address().as_u64(),
            stack: Box::pin([]),
        }];
    }

    #[allow(improper_ctypes_definitions)]
    extern "sysv64" fn task_call() -> ! {
        let f: usize;
        unsafe {
            asm!("mov {}, [rbp-8]", "add rsp, 8", out(reg) f);
            SCHEDULER.force_unlock();
            asm!("sti", "jmp {}", in(reg) f, options(noreturn));
        }
    }

    pub fn new_task(&mut self, stack_pages: usize, f: fn() -> !) -> TaskId {
        let id = self.tasks.len();
        let mut stack = Pin::new(vec![[0u64; 512]; stack_pages].into_boxed_slice());
        let kernel_stack_top =
            &stack.as_ref()[0] as *const u64 as u64 + 0x1000 * stack_pages as u64;
        let instruction_pointer = f as u64;
        stack[stack_pages - 1][511] = instruction_pointer;
        stack[stack_pages - 1][510] = Self::task_call as u64;
        stack[stack_pages - 1][509] = kernel_stack_top;
        let kernel_stack_top = kernel_stack_top - 24;
        self.tasks.push(TaskData {
            kernel_stack_top,
            cr3_value: Cr3::read().0.start_address().as_u64(),
            stack,
        });
        id
    }

    #[naked]
    extern "sysv64" fn switch_to_task_internal(new_cr3: u64, new_rsp: u64, old_rsp: &mut u64) {
        // rdi = new_cr3
        // rsi = new_rsp
        // rdx = old_rsp
        unsafe {
            asm!(
                "push rbp",
                "mov [rdx], rsp",
                "mov rsp, rsi",
                "mov rax, cr3",
                "cmp rdi, rax",
                "je 2f",
                "mov cr3, rdi",
                "2:",
                "pop rbp",
                "ret",
                options(noreturn)
            );
        }
    }

    pub fn switch_to_task(&mut self, task_id: TaskId) {
        x86_64::instructions::interrupts::without_interrupts(|| {
            let new_cr3 = self.tasks[task_id].cr3_value;
            let new_rsp = self.tasks[task_id].kernel_stack_top;
            let old_rsp = &mut self.tasks[self.current_task].kernel_stack_top;
            self.current_task = task_id;
            Self::switch_to_task_internal(new_cr3, new_rsp, old_rsp);
        });
    }
}

pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
