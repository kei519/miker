//! Useful syncronization primitives.

use core::{
    cell::UnsafeCell,
    hint,
    mem::MaybeUninit,
    sync::atomic::{self, AtomicBool, Ordering::*},
};

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

impl<T: Copy> OnceStatic<T> {
    /// Returns the copied inner value.
    ///
    /// # Panics
    ///
    /// May panic if it is not initialized.
    pub fn get(&self) -> T {
        if !self.is_initialized.load(Acquire) {
            panic!("OnceStatic is not initialized!");
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
            panic!("OnceStatic is not initialized!");
        }
        // Safety: Once `is_initialized` set to `true` after initializing, no one overwriter
        //      `data`. This leads there is no data rece.
        unsafe { (*self.data.get()).assume_init_ref() }
    }
}
