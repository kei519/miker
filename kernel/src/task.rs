//! Handle scheduling.

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use core::arch::global_asm;
use core::{cell::UnsafeCell, mem};
use util::paging::PAGE_SIZE;

use util::{
    asmfunc,
    collections::HashMap,
    sync::{InterruptFreeMutex, InterruptFreeMutexGuard},
};

use crate::memmap::PAGE_MAP;

const DEFAULT_STACK_SIZE_IN_PAGES: usize = 16;

pub static TASK_MANAGER: TaskManager = TaskManager::new();

/// Managing task schedule.
// TODO: Support multi processor.
#[derive(Debug)]
pub struct TaskManager {
    /// All tasks not exited.
    tasks: UnsafeCell<HashMap<u32, UnsafeCell<Task>>>,
    /// Runnnig queue.
    queue: UnsafeCell<VecDeque<u32>>,
    /// Currently running tasks's id.
    running_id: UnsafeCell<u32>,
    /// Id that was used on last registering task.
    head_id: UnsafeCell<u32>,
    lock: InterruptFreeMutex<()>,
}

unsafe impl Sync for TaskManager {}

impl TaskManager {
    /// Constructs a new empty [`TaskManager`].
    pub const fn new() -> Self {
        Self {
            tasks: UnsafeCell::new(HashMap::new()),
            queue: UnsafeCell::new(VecDeque::new()),
            running_id: UnsafeCell::new(0),
            head_id: UnsafeCell::new(0),
            lock: InterruptFreeMutex::new(()),
        }
    }

    /// Initialize [`TaskManager`]. Register current task as task whose id is `0`.
    pub fn init(&self) {
        let _lock = self.lock.lock();
        // Safety: lock is acquired.
        let tasks = unsafe { &mut *self.tasks.get() };
        let queue = unsafe { &mut *self.queue.get() };
        tasks.insert(0, UnsafeCell::new(Task::new(0, 0)));
        queue.push_back(0);
    }

    /// Register new task, whose entry point is `f` and it will run on `cs` code segment and `ss`
    /// stack segment with `priority`.
    pub fn register_new_task(&self, f: fn(), priority: u32, cs: u16, ss: u16) {
        let mut lock = self.lock.lock();
        // Safety: lock is acquired.
        let tasks = unsafe { &mut *self.tasks.get() };
        let queue = unsafe { &mut *self.queue.get() };

        let new_id = self.determine_id(Some(&mut lock));
        let new_task = Task::with_function(new_id, priority, f, cs, ss);

        tasks.insert(new_task.id, UnsafeCell::new(new_task));
        queue.push_back(new_id);
    }

    /// Saves current context `prev_ctx` and switches task.
    ///
    /// # Safety
    ///
    /// Call it from timer interrupt handler without enabling interrupts.
    pub unsafe fn switch(&self, prev_ctx: &Context) {
        // This may cause deadlock when another interrupt occurs, but it won't because caller
        // guarantees safety requirement.
        let mut lock = self.lock.lock();

        let next_id = self.rotate(Some(&mut lock));

        // Safety: lock is acquired.
        let tasks = unsafe { &mut *self.tasks.get() };
        let current_id = unsafe { &mut *self.running_id.get() };
        if *current_id == next_id {
            return;
        }

        let current_task = unsafe { &mut *tasks.get(current_id).unwrap().get() };
        *current_task.ctx = prev_ctx.clone();

        let next_task = unsafe { &*tasks.get(&next_id).unwrap().get() };
        *current_id = next_task.id;
        // We should release lock here because we can never release it after context switch. (Any
        // task never return here on the same context because `prev_ctx` is the context before
        // the interrupt occured.
        // We have to consider rece conditions, but before `IRET` instruction, IF flag is not set.
        // Since another interrupt cannot occur, race conditions do not.
        drop(lock);

        restore_context(next_task.ctx.as_ref());
    }

    fn rotate(&self, lock: Option<&mut InterruptFreeMutexGuard<'_, ()>>) -> u32 {
        let _lock = if lock.is_some() {
            None
        } else {
            Some(self.lock.lock())
        };

        let queue = unsafe { &mut *self.queue.get() };
        let current_id = queue.pop_front().unwrap();
        queue.push_back(current_id);
        let next_id = queue.front().copied().unwrap();
        next_id
    }

    fn determine_id(&self, lock: Option<&mut InterruptFreeMutexGuard<'_, ()>>) -> u32 {
        let _lock = match lock {
            Some(_) => None,
            None => Some(self.lock.lock()),
        };

        let head_id = unsafe { &mut *self.head_id.get() };
        match head_id.checked_add(1) {
            Some(next) => next,
            None => todo!("Not supported that the number of registerd tasks exceeds 0xFFFF_FFFF."),
        }
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Task {
    id: u32,
    // Should be saved in ProcessManager?
    _priority: u32,
    ctx: Box<Context>,
    _stack: Stack,
}

impl Task {
    pub fn new(id: u32, priority: u32) -> Self {
        Self {
            id,
            _priority: priority,
            ctx: Box::new(Context::new()),
            _stack: Stack::new(0),
        }
    }

    pub fn with_function(id: u32, priority: u32, f: fn(), cs: u16, ss: u16) -> Self {
        let mut ctx = Context::new();
        let stack = Stack::new(DEFAULT_STACK_SIZE_IN_PAGES);
        ctx.cr3 = asmfunc::get_cr3();
        ctx.rip = f as _;
        ctx.rsp = stack.as_end_ptr() as u64 - 8;
        ctx.cs = cs as _;
        ctx.ss = ss as _;
        ctx.rflags = 0x202;
        Self {
            id,
            _priority: priority,
            ctx: Box::new(ctx),
            _stack: stack,
        }
    }
}

/// Process context that have to save when switching contexts.
#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct Context {
    /// CR3
    pub cr3: u64,
    /// RIP
    pub rip: u64,
    /// RFLAGS
    pub rflags: u64,
    /// RESERVED1
    pub reserved1: u64,
    /// CS
    pub cs: u64,
    /// SS
    pub ss: u64,
    /// FS
    pub fs: u64,
    /// GS
    pub gs: u64,
    /// RAX
    pub rax: u64,
    /// RBX
    pub rbx: u64,
    /// RCX
    pub rcx: u64,
    /// RDX
    pub rdx: u64,
    /// RDI
    pub rdi: u64,
    /// RSI
    pub rsi: u64,
    /// RSP
    pub rsp: u64,
    /// RBP
    pub rbp: u64,
    /// R8
    pub r8: u64,
    /// R9
    pub r9: u64,
    /// R10
    pub r10: u64,
    /// R11
    pub r11: u64,
    /// R12
    pub r12: u64,
    /// R13
    pub r13: u64,
    /// R14
    pub r14: u64,
    /// R15
    pub r15: u64,
    /// FX
    pub fxsave_area: [u8; 512],
}

impl Context {
    /// Construct a new [`Context`] all of whose fields are zero.
    pub const fn new() -> Self {
        // Safety: It is valid `Context` value, whose all fields are zero.
        unsafe { mem::zeroed() }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct Stack {
    start: *mut u8,
    page_count: usize,
}

impl Stack {
    fn new(page_count: usize) -> Self {
        let start = PAGE_MAP.allocate(page_count);
        Self { start, page_count }
    }

    fn as_ptr(&self) -> *const u8 {
        self.start
    }

    fn as_end_ptr(&self) -> *const u8 {
        unsafe { self.as_ptr().byte_add(self.page_count * PAGE_SIZE) }
    }
}

/// Switch context from `current` to `next`.
#[allow(dead_code)]
fn switch_context(next: &Context, current: &mut Context) {
    unsafe { _switch_context(next, current) };
}

/// Restores saved context `next`.
fn restore_context(next: &Context) {
    unsafe { _restore_context(next) };
}

extern "sysv64" {
    fn _switch_context(next: &Context, current: &mut Context);
    fn _restore_context(next: &Context);
}

global_asm! { r#"
.global _switch_context
_switch_context:
    pushfq
    pop qword ptr [rsi + 0x10] # RFLAGS

    mov [rsi + 0x40], rax
    mov [rsi + 0x48], rbx
    mov [rsi + 0x50], rcx
    mov [rsi + 0x58], rdx
    mov [rsi + 0x60], rdi
    mov [rsi + 0x68], rsi

    mov rax, [rsp]
    mov [rsi + 0x08], rax # RIP

    mov rax, cr3
    mov [rsi + 0x00], rax # CR3

    mov ax, cs
    mov [rsi + 0x20], rax # CS
    mov ax, ss
    mov [rsi + 0x28], rax # SS
    mov ax, fs
    mov [rsi + 0x30], rax # FS
    mov ax, gs
    mov [rsi + 0x38], rax # GS

    lea rax, [rsp + 8]
    mov [rsi + 0x70], rax # RSP
    mov [rsi + 0x78], rbp
    mov [rsi + 0x80], r8
    mov [rsi + 0x88], r9
    mov [rsi + 0x90], r10
    mov [rsi + 0x98], r11
    mov [rsi + 0xa0], r12
    mov [rsi + 0xa8], r13
    mov [rsi + 0xb0], r14
    mov [rsi + 0xb8], r15
    fxsave [rsi + 0xc0]

    # Fall through to _restore_context

.global _restore_context
_restore_context:

    # Constructs the next context frame.
    push qword ptr [rdi + 0x28] # SS
    push qword ptr [rdi + 0x70] # RSP
    push qword ptr [rdi + 0x10] # RFLAGS
    push qword ptr [rdi + 0x20] # CS
    push qword ptr [rdi + 0x08] # RIP

    fxrstor [rdi + 0xc0]
    mov r15, [rdi + 0xb8]
    mov r14, [rdi + 0xb0]
    mov r13, [rdi + 0xa8]
    mov r12, [rdi + 0xa0]
    mov r11, [rdi + 0x98]
    mov r10, [rdi + 0x90]
    mov r9, [rdi + 0x88]
    mov r8, [rdi + 0x80]
    mov rbp, [rdi + 0x78]

    mov rax, [rdi + 0x38] # GS
    mov rbx, [rdi + 0x30] # FS
    mov gs, ax
    mov fs, bx

    mov rax, [rdi + 0x00] # CR3
    mov cr3, rax

    mov rsi, [rdi + 0x68]
    mov rdx, [rdi + 0x58]
    mov rcx, [rdi + 0x50]
    mov rbx, [rdi + 0x48]
    mov rax, [rdi + 0x40]

    mov rdi, [rdi + 0x60]

    iretq
"#
}
