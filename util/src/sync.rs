//! Useful syncronization primitives.

use core::{
    any,
    cell::UnsafeCell,
    hint,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    sync::atomic::{self, AtomicBool, Ordering::*},
};

use crate::asmfunc;

/// Immutable static variable that will be initialized after running program. You cannot change
/// inner value after you once initialized.
#[derive(Debug)]
pub struct OnceStatic<T> {
    /// Data which may be uninitialized.
    data: UnsafeCell<MaybeUninit<T>>,
    /// Represents if `data` field is initialized.
    is_initialized: AtomicBool,
    /// Represents if someone has `self`'s lock.
    ///
    /// # Safety
    ///
    /// You must not change any values in this struct when this field is `true`, and you must
    /// definitely set this to `true` before changing values.
    lock: AtomicBool,
}

// Safety: Send `&OnceStatic<T>` means send `&T` because there is a public way only get `&T`
//      not `&mut T` with unsychronized fashion. So `OnceStatic<T>` is `Sync` requires `T` is `Sync`
//      but just only that.
unsafe impl<T: Sync> Sync for OnceStatic<T> {}

impl<T> OnceStatic<T> {
    /// Constructs uninitialized one.
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            is_initialized: AtomicBool::new(false),
            lock: AtomicBool::new(false),
        }
    }

    /// Initializes inner value to `value`. If it is already initialized, return `false`, otherwise
    /// `true`.
    pub fn init(&self, value: T) -> bool {
        // Acquire lock to update the inner value.
        while self
            .lock
            .compare_exchange_weak(false, true, Acquire, Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }

        if self.is_initialized.load(Relaxed) {
            self.lock.store(false, Relaxed);
            return false;
        }

        // Safety: Since `lock` can be taken by one thread, there is no data rece.
        unsafe { (*self.data.get()).write(value) };

        // When getting inner value, we assume `data` is initialized if `is_initialized` is `true`.
        // So this storing should be `Release` only when called `get()`-like method.
        // `lock` should the same when called `init()`.
        // Since these are partially requirement, we put `fence()` here.
        atomic::fence(Release);
        self.is_initialized.store(true, Relaxed);
        self.lock.store(false, Relaxed);
        true
    }

    /// Constructs already initialized with `value` one.
    pub const fn from(value: T) -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::new(value)),
            is_initialized: AtomicBool::new(true),
            lock: AtomicBool::new(false),
        }
    }

    /// Returns if `self` is initialized.
    pub fn is_initialized(&self) -> bool {
        self.is_initialized.load(Relaxed)
    }

    /// Returns the inner value as a reference to it.
    ///
    /// For a safe alternative see [`as_ref`][AsRef::as_ref].
    ///
    /// # Safety
    ///
    /// Calling this method with uninitialized value is undefined behavior.
    pub unsafe fn as_ref_unchecked(&self) -> &T {
        unsafe { (*self.data.get()).assume_init_ref() }
    }
}

impl<T> Drop for OnceStatic<T> {
    fn drop(&mut self) {
        if self.is_initialized() {
            // Safety: `self` is mutable borrowed, that is there is no other references and `data` is
            //         initialized.
            unsafe { (*self.data.get()).assume_init_drop() };
        }
    }
}

impl<T: Copy> OnceStatic<T> {
    /// Returns the copied inner value.
    ///
    /// # Panics
    ///
    /// May panic if it is not initialized.
    pub fn get(&self) -> T {
        if !self.is_initialized.load(Acquire) {
            panic!(
                "OnceStatic ({:?}) is not initialized!",
                any::type_name_of_val(self)
            );
        }
        // Safety: Once `is_initialized` set to `true` after initializing, no one overwriter
        //      `data`. This leads there is no data rece.
        *unsafe { (*self.data.get()).assume_init_ref() }
    }

    /// Returns the copied inner value.
    ///
    /// For a safe alternative see [`get`][Self::get].
    ///
    /// # Safety
    ///
    /// Calling this method with uninitialized value is undefined behavior.
    pub unsafe fn get_uncecked(&self) -> T {
        *unsafe { (*self.data.get()).assume_init_ref() }
    }
}

impl<T> Default for OnceStatic<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AsRef<T> for OnceStatic<T> {
    /// Returns the inner value as a reference to it.
    ///
    /// # Panics
    ///
    /// May panic if it is not initialzed.
    fn as_ref(&self) -> &T {
        if !self.is_initialized.load(Acquire) {
            panic!(
                "OnceStatic ({:?}) is not initialized!",
                any::type_name_of_val(self)
            );
        }
        // Safety: Once `is_initialized` set to `true` after initializing, no one overwriter
        //      `data`. This leads there is no data rece.
        unsafe { (*self.data.get()).assume_init_ref() }
    }
}

/// Provides a mutex lock with disabling interrupts until release the lock.
#[derive(Debug)]
pub struct InterruptFreeMutex<T> {
    /// Data guarded by the lock.
    data: UnsafeCell<T>,
    /// Represents whether someone has the lock.
    locker: AtomicBool,
    /// Saves whether IF (interrupt flag) is set before disabling interrupts.
    prev_if: AtomicBool,
}

// Safety: `InterruptFreeMutex<T>` and its shared referenece only provide an exclusive mutability to
//     `T`. This means that the safety of `Sync`ing `InterruptFreeMutex<T>` requires the safety of
//     `Send`ing `T`.
unsafe impl<T: Send> Sync for InterruptFreeMutex<T> {}

impl<T> InterruptFreeMutex<T> {
    /// Constructs new [`InterruptFreeMutex`] whose initial value is `value`.
    pub const fn new(value: T) -> Self {
        Self {
            data: UnsafeCell::new(value),
            locker: AtomicBool::new(false),
            prev_if: AtomicBool::new(false),
        }
    }

    /// Trys to take a lock, and if succeeds returns the guard. Otherwise, returns `None`.
    pub fn try_lock(&self) -> Option<InterruptFreeMutexGuard<'_, T>> {
        self.prev_if.store(asmfunc::get_if(), Relaxed);
        // NOTE: We disable interrupts even if interrupts are already disabled because conditional
        //       branching is expensive.
        asmfunc::cli();

        if self.locker.swap(true, Relaxed) {
            if self.prev_if.load(Relaxed) {
                asmfunc::sti();
            }
            None
        } else {
            // Since we don't need `Acquire` when failed, put fence here.
            atomic::fence(Acquire);
            Some(InterruptFreeMutexGuard {
                data: unsafe { &mut *self.data.get() },
                locker: &self.locker,
                prev_if: &self.prev_if,
            })
        }
    }

    /// Until succeeding acuiring the lock, spins loop. Then returns the guard.
    ///
    /// If you do not need to lock definitely, use [`InterruptFreeMutex::try_lock()`] instead.
    pub fn lock(&self) -> InterruptFreeMutexGuard<'_, T> {
        loop {
            match self.try_lock() {
                Some(guard) => break guard,
                None => hint::spin_loop(),
            }
        }
    }
}

/// Provides exclusive access to a data guarded by lock.
#[must_use = "Droping lock causes immediately release the lock."]
pub struct InterruptFreeMutexGuard<'this, T> {
    /// Guarded data.
    data: &'this mut T,
    /// Reference to value representing whether it is locked. Used to release the lock.
    locker: &'this AtomicBool,
    /// Reference to value representing whether IF is set.
    prev_if: &'this AtomicBool,
}

impl<'this, T> Deref for InterruptFreeMutexGuard<'this, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'this, T> DerefMut for InterruptFreeMutexGuard<'this, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'this, T> Drop for InterruptFreeMutexGuard<'this, T> {
    fn drop(&mut self) {
        self.locker.store(false, Release);
        if self.prev_if.load(Relaxed) {
            asmfunc::sti();
        }
    }
}
