//! Provide memmap features.

use core::{
    cell::UnsafeCell,
    fmt::Debug,
    hint,
    marker::PhantomPinned,
    mem, ptr,
    sync::atomic::{AtomicBool, Ordering::*},
};

use uefi::table::boot::{MemoryMap, MemoryType};
use util::{asmfunc, paging::PAGE_SIZE};

/// Max order of the buddy system.
pub const MAX_ORDER: usize = 10;

/// [PageMap] for this kernel.
pub static PAGE_MAP: PageMap = PageMap {
    table: UnsafeCell::new([
        None, None, None, None, None, None, None, None, None, None, None,
    ]),
    cache: UnsafeCell::new(ptr::null_mut()),
    lock: AtomicBool::new(false),
};

/// Holds page map info, allocate and free pages with using buddy system.
pub struct PageMap {
    /// Table of [PageBlock] whose orders are 2^0, 2^1, .., 2^{[`MAX_ORDER`]}.
    table: UnsafeCell<[Option<&'static mut PageBlock>; MAX_ORDER + 1]>,
    /// Linked list holding unused [`PageBlock`]s.
    cache: UnsafeCell<*mut PageBlock>,
    /// Lock to operate [PageMap] without race conditions.
    lock: AtomicBool,
}

// Safety: `PageMap` does not provide any method to directly change interior values. We take
//     lock and disables interrupts when change values in methods. This guarantees there is no race
//     condition.
unsafe impl Sync for PageMap {}

impl PageMap {
    /// Initialize [PageMap] with `memmap`. Considers [MemoryType::BOOT_SERVICES_CODE],
    /// [MemoryType::BOOT_SERVICES_DATA] and [MemoryType::CONVENTIONAL] are usable.
    ///
    /// # Safety
    ///
    /// Since [PageMap] does not save whether it is initialized, causes UB if you call this more
    /// than once.
    pub unsafe fn init(&self, memmap: &'static MemoryMap) {
        let _lock = self.lock();

        // Save a start address of a page block.
        let mut block_start = 0;
        // Save how many pages are continuous.
        let mut page_count = 0;
        for desc in memmap.entries() {
            if ![
                MemoryType::BOOT_SERVICES_CODE,
                MemoryType::BOOT_SERVICES_DATA,
                MemoryType::CONVENTIONAL,
            ]
            .contains(&desc.ty)
            {
                continue;
            }

            // If `desc` is the continuation of the block considering, connect them.
            if block_start + (page_count * PAGE_SIZE) as u64 == desc.phys_start {
                page_count += desc.page_count as usize;
                continue;
            }

            self.register_continuous_pages(block_start, page_count, true);

            block_start = desc.phys_start;
            page_count = desc.page_count as _;
        }

        self.register_continuous_pages(block_start, page_count, true);
    }

    /// Allocate `page_count` pages and returns the start address. If failed allocating, returns
    /// null.
    ///
    /// Since accepted `page_count` is one of 2^0, 2^1, ..., 2^{[`MAX_ORDER`]}, passing others
    /// causes failure.
    pub fn allocate(&self, page_count: usize) -> *mut u8 {
        let _lock = self.lock();

        // Check `page_count` condition.
        if !page_count.is_power_of_two() || page_count > 1 << MAX_ORDER {
            return ptr::null_mut();
        }
        let order = page_count.ilog2() as usize;

        // Search a free page whose order is `order` or more.
        let mut search_order = order;
        while search_order <= MAX_ORDER {
            if let Some(block) = self.remove_block(None, search_order, true) {
                let start = block.start;
                // Cache the used `PageBlock`.
                self.push_cache(block, true);
                if search_order > order {
                    // When a found block's order is greater than `order`, there is extra space. So,
                    // push it back into the tables after splitting it into smaller blocks.
                    self.register_continuous_pages(
                        start + (PAGE_SIZE << order) as u64,
                        (1 << search_order) - page_count,
                        true,
                    )
                }
                return start as _;
            }
            search_order += 1;
        }
        // If there is no enough free page whose order is no more than `MAX_ORDER` , it's failure.
        ptr::null_mut()
    }

    /// Free `page_count` pages starting at `start`.
    ///
    /// # Safety
    ///
    /// - `start` must be the address allocated by [`PageMap::allocated()`] with passing `page_count`
    ///     as a argument.
    /// - You can free allocated pages just once.
    /// - You must never access freed pages.
    pub unsafe fn free(&self, start: *mut u8, page_count: usize) {
        let _lock = self.lock();
        let start = start as u64;

        if let Some(block) = self.pop_cache(true) {
            block.start = start;
            block.page_count = page_count;
            unsafe { self.insert_block_with_merge(block, true) };
        } else {
            // Since there is no cached block, we have to allocate a page for blocks. After we do
            // it, there is no buddy because all buddies whose sizes are less than `page_count`
            // are contained by the block splitted to alloation.
            //
            // In short, we don't have to merge surplus pages, and all we have to do is just add
            // the other pages into [`Self::table`] as continouse pages.
            let new_start = start + PAGE_SIZE as u64;
            // Safety:
            //   * `start` is properly aligned and not null because it is returned value of
            //     `Self::allocate()`.
            //   * Memory in pages is guaranteed not to be used by caller.
            unsafe { self.store_as_cache(start, new_start, true) };
            self.register_continuous_pages(new_start, page_count - 1, true);
        }
    }

    /// Returns how many pages are free.
    pub fn free_pages_count(&self) -> usize {
        let _lock = self.lock();
        let table = unsafe { &*self.table.get() };
        let ret = table.iter().fold(0, |acc, block| {
            acc + block
                .as_ref()
                .map(|blk| blk.list_len() * blk.page_count)
                .unwrap_or(0)
        });
        ret
    }

    /// Add a given block that starts at `block_start` and whose size, in page, is `page_count`
    /// into [PageMap::table]. If `page_count` is greater than 2^{[`MAX_ORDER`]}, insert them after
    /// splitting it into smaller blocks.
    fn register_continuous_pages(
        &self,
        mut block_start: u64,
        mut page_count: usize,
        is_locked: bool,
    ) {
        let _lock = if !is_locked { Some(self.lock()) } else { None };

        while page_count > 0 {
            let Some(block) = self.pop_cache(true) else {
                let end = block_start + PAGE_SIZE as u64;
                unsafe { self.store_as_cache(block_start, end, true) };
                block_start = end;
                page_count -= 1;
                continue;
            };

            let order = (page_count.ilog2() as usize)
                .min(MAX_ORDER)
                // Consider align
                .min((block_start.trailing_zeros() - PAGE_SIZE.trailing_zeros()) as usize);
            let count = 1 << order;
            block.start = block_start;
            block.page_count = count;
            // Safety: `count` is a power of two and no more than 2^{`MAX_ORDER`}.
            unsafe { self.insert_block(block, true) };

            block_start += (count * PAGE_SIZE) as u64;
            page_count -= count;
        }
    }

    /// Find the buddy of `block` and merge them recursively, then insert a merged block to the
    /// [`PageMap::table`].
    ///
    /// If you do not need to search the buddy , call [`PageMap::insert_block()`] instead.
    ///
    /// # Safety
    ///
    /// `block.page_count` must be the power of two and no more than 2^{[`MAX_ORDER`]}.
    unsafe fn insert_block_with_merge(&self, block: &'static mut PageBlock, is_locked: bool) {
        let _lock = if !is_locked { Some(self.lock()) } else { None };

        while block.page_count <= MAX_ORDER {
            let order = block.page_count.ilog2() as usize;
            let buddy_start = block.start ^ (PAGE_SIZE << order) as u64;
            let Some(buddy_block) = self.remove_block(Some(buddy_start), order, true) else {
                break;
            };
            // The start address of merged block is the minimum of both.
            block.start = block.start.min(buddy_start);
            block.page_count *= 2;
            self.push_cache(buddy_block, true);
        }
        // Safety: start address and the number of pages is guaranteed to meet the condition by
        //     caller and `Self::remove_block` that returns a proper block.
        unsafe { self.insert_block(block, true) }
    }

    /// Insert `block` to the proper position of [PageMap::table].
    ///
    /// This method does not search the buddy and merge with it. If you want to merge `block` and
    /// the buddy, call [`PageMap::insert_block_with_merge()`] instead.
    ///
    /// # Safety
    ///
    /// `block.page_count` must be the power of two and no more than 2^{[`MAX_ORDER`]}.
    unsafe fn insert_block(&self, block: &'static mut PageBlock, is_locked: bool) {
        let _lock = if !is_locked { Some(self.lock()) } else { None };

        let table = unsafe { &mut *self.table.get() };

        let order = block.page_count.ilog2() as usize;
        let mut prev = None;
        mem::swap(&mut table[order], &mut prev);
        block.next = prev;
        table[order] = Some(block);
    }

    /// Store memory space \[`start`, `end`) as an linked list of [`PageBlock`]s into
    /// [`Self::cache`]. Since [`PageBlock`]'s size is greater than 8 bytes, we can use its space
    /// to store the next [`PageBlock`] pointer.
    ///
    /// # Safety
    ///
    /// `start` must be properly aligned to [`PageBlock`] and must not be `0` (that is `null`).
    /// Memory space \[`start`, `end`) must not be used as other objects.
    unsafe fn store_as_cache(&self, mut start: u64, end: u64, is_locked: bool) {
        let _lock = if !is_locked { Some(self.lock()) } else { None };
        let cache = unsafe { &mut *self.cache.get() };

        let mut head = *cache;
        let block_size = mem::size_of::<PageBlock>() as u64;
        while start + block_size < end {
            let cur = start as *mut *mut PageBlock;
            start += block_size;

            // Safety:
            //     * `cur` is valid because
            //        ** `cur` is not null.
            //        ** Each `cur` is separeted.
            //        ** `cur` is accessed by a thread.
            //        ** `cur` is not a pointer casted to.
            //
            //     * `cur` is properly aligned.
            //       First, the alignment of `PageBlock` is at least 8 bytes because of the
            //       definition, and a raw pointer is 8-byte aligned.
            //       Second, passed `start` value is properly aligned to `PageBlock` by caller and
            //       just adding to `start` the size of PageBlock multiple of the align of.
            //       These mean `cur` is properly aligned to a raw pointer.
            unsafe { cur.write(head) };
            head = cur as *mut PageBlock;
        }
        *cache = head;
    }

    /// Push `block` to front of [`Self::cache`].
    fn push_cache(&self, block: &'static mut PageBlock, is_locked: bool) {
        let _lock = if !is_locked { Some(self.lock()) } else { None };
        let cache = unsafe { &mut *self.cache.get() };

        let new_cache = (block as *mut PageBlock).cast();
        // Safety: Since `block` is properly aligned to [`PageBlock`] that is aligned to 8 or above
        //     bytes, so is `new_cache`, that is sufficient to hold raw pointer.
        unsafe { *new_cache = *cache };
        *cache = new_cache.cast();
    }

    /// Pop [`PageBlock`] from [`Self::cache`]'s linked list if it has.
    fn pop_cache(&self, is_locked: bool) -> Option<&'static mut PageBlock> {
        let _lock = if !is_locked { Some(self.lock()) } else { None };
        let cache = unsafe { &mut *self.cache.get() };
        if cache.is_null() {
            return None;
        }

        let head = *cache as *mut *mut PageBlock;
        // Safety: `Self::cache` should have proper value.
        let next = unsafe { *head };
        *cache = next;
        let ret: *mut PageBlock = head.cast();
        // Safety: Caches stored in `Self::cache` are constructed by `Self::store_as_cache` that
        //     alignes them properly.
        unsafe {
            ret.write(PageBlock::new(0, 0));
            Some(&mut *ret.cast())
        }
    }

    /// Remove a block from [`Self::table`] and return it if succeeded.
    ///
    /// If `start` is `None`, returns one of tables whose orders are `order`. Otherwise, returns
    /// the table starting at `start`.
    fn remove_block(
        &self,
        start: Option<u64>,
        order: usize,
        is_locked: bool,
    ) -> Option<&'static mut PageBlock> {
        if order > MAX_ORDER {
            return None;
        }

        let _lock = if !is_locked { Some(self.lock()) } else { None };
        let table = unsafe { &mut *self.table.get() };

        // Check each block following `table[order]` one by one. To do so, we have to disconnect
        // the next block from each because of Rust borrowing rule.
        // So, we hold the original value of `table[order]` as `head`, the tail of the linked list
        // starting with `head ` as `tail`, and the original value of `tail.next` as `next` after
        // separated from `tail`.
        let mut head = None;
        mem::swap(&mut table[order], &mut head);
        // `head` meets the condition above now.

        // If `table[order]` was `None`, there is no block whose order is `order`.
        let head = head?;
        let mut next = None;
        mem::swap(&mut head.next, &mut next);
        // `next` meets the condition above now.

        if start.is_none() {
            mem::swap(&mut table[order], &mut next);
            return Some(head);
        }

        // Here, `starts` is `Some` because we already handled `None` case.
        let start = start.unwrap();
        // Since `tail` can't hold `head` because of Rust borrowing check, we are handling the case
        // where `head` is the wanted block here.
        if start == head.start {
            mem::swap(&mut table[order], &mut next);
            return Some(head);
        }

        let mut tail = &mut head.next;
        // Check if `next` is the wanted one.
        while let Some(cur) = next {
            next = None;
            mem::swap(&mut cur.next, &mut next);
            // Now
            //
            // 1. `block[order]` ( = `head` ) -> ... -> `tail` -> `cur` -> `next` -> ...
            //
            // becomes
            //
            // 1. `block[order]` == `None`
            // 2. `head` -> ... -> `tail`
            // 3. `next` -> ...
            // and
            // 4. `cur` <- check this!

            if start == cur.start {
                if let Some(tail) = tail {
                    tail.next = next;
                    table[order] = Some(head);
                    return Some(cur);
                }
            }
            *tail = Some(cur);
            tail = &mut tail.as_mut().unwrap().next;
        }
        table[order] = Some(head);

        None
    }

    /// Disables interrupts and get the lock of `self`. If another thread has lock, spins loop to
    /// get.
    fn lock(&self) -> Lock<'_> {
        asmfunc::cli();
        while self
            .lock
            .compare_exchange_weak(false, true, Acquire, Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }
        Lock(&self.lock)
    }
}

impl Debug for PageMap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let _lock = self.lock();
        let table = unsafe { &*self.table.get() };
        let mut list = f.debug_list();
        for entry in table {
            list.entry(entry);
        }
        list.finish()
    }
}

/// Provides the unlock feature as [`Drop::drop`].
#[must_use = "Droping Lock causes immediately release the lock."]
struct Lock<'a>(&'a AtomicBool);

impl<'a> Drop for Lock<'a> {
    fn drop(&mut self) {
        self.0.store(false, Release);
        asmfunc::sti();
    }
}

/// Represents a block as continuous pages.
struct PageBlock {
    /// Points to next [PageBlock].
    next: Option<&'static mut PageBlock>,
    /// Start address of the block.
    start: u64,
    /// The size of block, in page. It intends to be a power of two.
    page_count: usize,
    _pin: PhantomPinned,
}

impl PageBlock {
    /// Constructs new [PageBlock] whose `next` is `None`.
    fn new(start: u64, page_count: usize) -> Self {
        Self {
            next: None,
            start,
            page_count,
            _pin: PhantomPinned,
        }
    }

    /// Retunrs the end address of the block.
    fn end(&self) -> u64 {
        self.start + (self.page_count * PAGE_SIZE) as u64
    }

    /// Returns the length of the lists whose head is `self`.
    fn list_len(&self) -> usize {
        let mut cur = self;
        let mut count = 1;
        while let Some(next) = &cur.next {
            cur = *next;
            count += 1;
        }
        count
    }
}

impl Debug for PageBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if f.alternate() {
            writeln!(f, "{:08x}-{:08x},", self.start, self.end())
        } else {
            write!(f, "{:08x}-{:08x}, ", self.start, self.end())
        }?;
        if let Some(next) = &self.next {
            Debug::fmt(next, f)?
        }

        Ok(())
    }
}
