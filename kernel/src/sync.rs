//! Provides items for synchronization.

use core::{
    cell::UnsafeCell,
    fmt::{Debug, Display},
    hint,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering::*},
};

use alloc::collections::VecDeque;

use crate::task::{TaskId, TASK_MANAGER};

/// Shared reference providing mutable exclusion.
pub struct Mutex<T> {
    /// Innter data.
    data: UnsafeCell<T>,
    /// Whether lock is acquired.
    lock: AtomicBool,
    /// Waiting task queue.
    queue: UnsafeCell<VecDeque<TaskId>>,
    /// Lock for controlling [`queue`](Mutex<T>.queue).
    queue_lock: AtomicBool,
}

// Safety: `Mutex` provides exclusive mutability from even its shared reference. That is, sending a
//         shared reference of `Mutex<T>` allows you to safely obtain `&mut T`. Therefore,
//         `Mutex<T>` is `Sync` if and only if `&mut T` is `Send`, i.e. `T` is `Send`.
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Constructs new [`Mutex<T>`] with initial value, `value`.
    pub const fn new(value: T) -> Self {
        Self {
            data: UnsafeCell::new(value),
            lock: AtomicBool::new(false),
            queue: UnsafeCell::new(VecDeque::new()),
            queue_lock: AtomicBool::new(false),
        }
    }

    /// Trys to lock and returns [`MutexGuard`] if succeeded.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        // Check whether there are tasks waiting for the lock to be released. If there are, we
        // won't try to lock to yield the lock.
        if self
            .queue_lock
            .compare_exchange(false, true, Acquire, Relaxed)
            .is_err()
        {
            // Don't acquire lock when the queue is also locked.
            return None;
        }
        if unsafe { (*self.queue.get()).len() } != 0 {
            self.queue_lock.store(false, Relaxed);
            return None;
        }
        self.queue_lock.store(false, Relaxed);

        if self
            .lock
            .compare_exchange(false, true, Acquire, Relaxed)
            .is_err()
        {
            None
        } else {
            Some(MutexGuard {
                data: unsafe { &mut *self.data.get() },
                lock: &self.lock,
                queue: &self.queue,
                queue_lock: &self.queue_lock,
            })
        }
    }

    /// Acquires the lock and returns the handle to control the inner data.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        let task_id = TASK_MANAGER.task_id();
        let mut waiting = false;

        // Since `queue_lock` is usually rapidly released, just use a spinlock.
        while self
            .queue_lock
            .compare_exchange_weak(false, true, Acquire, Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }
        let queue = unsafe { &mut *self.queue.get() };
        if queue.is_empty() {
            // If there is no task waiting for the lock to be released, we are going to try to
            // acquire the lock.
            self.queue_lock.store(false, Release);
        } else {
            // If some tasks are waiting for the lock to be released, we add the current task to
            // the waiting queue and put it to sleep.
            queue.push_back(task_id);
            self.queue_lock.store(false, Release);
            waiting = true;
            TASK_MANAGER.sleep();
            // Usually the lock is released when the control returns here.
        }

        // Use `compare_exchange` not `compare_exchange_weak` because suspending the task on a
        // false failure causes it to sleep forever.
        while self
            .lock
            .compare_exchange(false, true, Acquire, Relaxed)
            .is_err()
        {
            // Push self to the watting queue when another task is holding the lock.
            while self
                .queue_lock
                .compare_exchange_weak(false, true, Acquire, Relaxed)
                .is_err()
            {
                hint::spin_loop();
            }
            let queue = unsafe { &mut *self.queue.get() };
            if !queue.contains(&task_id) {
                waiting = true;
                queue.push_back(task_id);
            }
            self.queue_lock.store(false, Release);
            TASK_MANAGER.sleep();
        }

        // Pop self from the queue only when waiting for the lock to be released.
        if waiting {
            while self
                .queue_lock
                .compare_exchange_weak(false, true, Acquire, Relaxed)
                .is_err()
            {
                hint::spin_loop();
            }
            let queue = unsafe { &mut *self.queue.get() };
            if let Some(index) = queue
                .iter()
                .enumerate()
                .find(|(_, &id)| id == task_id)
                .map(|(i, _)| i)
            {
                queue.remove(index);
            }
            self.queue_lock.store(false, Release);
        }

        MutexGuard {
            data: unsafe { &mut *self.data.get() },
            lock: &self.lock,
            queue: &self.queue,
            queue_lock: &self.queue_lock,
        }
    }

    /// Consumes `self` and returns the inner value.
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }

    /// Returns an exclusive reference to the inner value. This does not acquire the lock because
    /// requiring an exclusive referece to `self`, which already ensures exclusivity.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

impl<T: Debug> Debug for Mutex<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut f = f.debug_struct("Mutex");
        if let Some(guard) = self.try_lock() {
            f.field("data", &*guard);
        } else {
            f.field("data", &format_args!("<locked>"));
        }
        f.finish_non_exhaustive()
    }
}

/// Provides exclusive control to the inner value of [`Mutex<T>`]. Releases the lock when dropped.
pub struct MutexGuard<'this, T> {
    data: &'this mut T,
    lock: &'this AtomicBool,
    queue: &'this UnsafeCell<VecDeque<TaskId>>,
    queue_lock: &'this AtomicBool,
}

impl<'this, T> Drop for MutexGuard<'this, T> {
    fn drop(&mut self) {
        self.lock.store(false, Release);
        while self
            .queue_lock
            .compare_exchange_weak(false, true, Acquire, Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }
        if let Some(&next_id) = unsafe { (*self.queue.get()).front() } {
            TASK_MANAGER.wake_up(next_id);
        }
        self.queue_lock.store(false, Release);
    }
}

impl<'this, T> Deref for MutexGuard<'this, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'this, T> DerefMut for MutexGuard<'this, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'this, T: Debug> Debug for MutexGuard<'this, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(&self.data, f)
    }
}

impl<'this, T: Display> Display for MutexGuard<'this, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self.data, f)
    }
}
