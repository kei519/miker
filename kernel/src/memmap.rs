//! Provide memmap features.

use core::{
    cell::UnsafeCell,
    fmt::Debug,
    hint,
    marker::PhantomPinned,
    mem,
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
    lock: AtomicBool::new(false),
};

/// Holds page map info, allocate and free pages with using buddy system.
pub struct PageMap {
    /// Table of [PageBlock] whose orders are 2^0, 2^1, .., 2^{[`MAX_ORDER`]}.
    table: UnsafeCell<[Option<&'static mut PageBlock>; MAX_ORDER + 1]>,
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

        // Save the head address to allocate [PageTable].
        let mut brk = 0;
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

            self.register_continuous_pages(&mut brk, &mut block_start, &mut page_count, true);

            block_start = desc.phys_start;
            page_count = desc.page_count as _;
        }

        self.register_continuous_pages(&mut brk, &mut block_start, &mut page_count, true);
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
    /// into [PageMap::table]. If `page_count` is bigger than 2^{[`MAX_ORDER`]}, insert them after
    /// splitting it into smaller blocks. Set `brk` to zero at first time.
    ///
    /// When `brk` gets not enough to alloate [PageTable], allocate a page from the block fot it.
    fn register_continuous_pages(
        &self,
        brk: &mut u64,
        block_start: &mut u64,
        page_count: &mut usize,
        is_locked: bool,
    ) {
        let _lock = if !is_locked { Some(self.lock()) } else { None };

        while *page_count > 0 {
            // `brk == 0` means that there is no space to allocate `PageTable`.
            if *brk == 0 {
                *brk = *block_start;
                *block_start += PAGE_SIZE as u64;
                *page_count -= 1;
                continue;
            }

            // There is enough space here.
            let block = *brk as *mut PageBlock;
            *brk += mem::size_of::<PageBlock>() as u64;

            let order = (page_count.ilog2() as usize).min(MAX_ORDER);
            let count = 1 << order;
            // Safety:
            //     * `block` is valid because
            //        ** `block` is not null.
            //        ** Each `block` is separeted.
            //        ** `block` is accessed a thread.
            //        ** `block` is not a pointer casted to.
            //
            //     * `block` is propery aligned because "first" non-zero `brk` is 4 KB aligned and
            //        just adding to `brk` the size of PageBlock multiple of the align of. (Here,
            //        "first" means that just after `brk` is 0 and assigned non-zero value.)
            let block = unsafe {
                block.write(PageBlock::new(*block_start, count));
                &mut *block
            };
            // Safety: `count` is a power of two and eqaul to or smaller than 2^{`MAX_ORDER`}.
            unsafe { self.insert_block(block, true) };

            *block_start += (count * PAGE_SIZE) as u64;
            *page_count -= count;

            // If there is no enough space to allocate `PageBlock`, set brk to zero to avoid
            // allocating other objects spaces and indicate allocate a new page.
            if *brk as usize % PAGE_SIZE > PAGE_SIZE - mem::size_of::<PageBlock>() {
                *brk = 0;
            }
        }
    }

    /// Insert `block` to the proper position of [PageMap::table].
    ///
    /// # Safety
    ///
    /// `block.page_count` must be the power of two and equal to or smaller than 2^{[`MAX_ORDER`]}.
    unsafe fn insert_block(&self, block: &'static mut PageBlock, is_locked: bool) {
        let _lock = if !is_locked { Some(self.lock()) } else { None };

        let table = unsafe { &mut *self.table.get() };

        let order = block.page_count.ilog2() as usize;
        let mut prev = None;
        mem::swap(&mut table[order], &mut prev);
        block.next = prev;
        table[order] = Some(block);
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