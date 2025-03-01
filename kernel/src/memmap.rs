//! Provide memmap features.

use alloc::vec::Vec;
use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    fmt::Debug,
    marker::PhantomPinned,
    mem, ptr,
};

use uefi::table::{
    Runtime, SystemTable,
    boot::{MemoryMap, MemoryType},
};
use util::{
    paging::{PAGE_SIZE, PageEntry},
    sync::InterruptFreeMutex,
};

use crate::paging::{self, KERNEL_PML4};

/// Represents word size of environment.
pub const WORD_SIZE: usize = mem::size_of::<usize>();

/// Max order of the buddy system.
pub const MAX_ORDER: usize = 10;

/// Number of caches stored in [`Global`] allocator.
const NUM_CHACHES: usize = (PAGE_SIZE.trailing_zeros() - WORD_SIZE.trailing_zeros()) as usize;

/// [PageMap] for this kernel.
pub static PAGE_MAP: PageMap = PageMap {
    table: UnsafeCell::new([
        None, None, None, None, None, None, None, None, None, None, None,
    ]),
    cache: UnsafeCell::new(Cache::new()),
    lock: InterruptFreeMutex::new(()),
    memmap: UnsafeCell::new(None),
};

/// [`Global`] allocator for this kernel.
#[global_allocator]
static GLOBAL: Global = Global {
    start: UnsafeCell::new(0),
    end: UnsafeCell::new(0),
    caches: UnsafeCell::new([Cache::new(); NUM_CHACHES]),
    lock: InterruptFreeMutex::new(()),
};

/// Holds data required for global allocators.
struct Global {
    /// Start address of memory where this can allocate.
    start: UnsafeCell<u64>,
    /// End address of memory where this can allocate.
    end: UnsafeCell<u64>,
    /// Unused memory spaces' caches.
    caches: UnsafeCell<[Cache<usize>; NUM_CHACHES]>,
    /// Lock to operate [`Global`] without race conditions.
    lock: InterruptFreeMutex<()>,
}

// Safety: `Global` does not provide any method to directly change interior values. We take
//     lock and disables interrupts when change values in methods. This guarantees there is no race
//     condition.
unsafe impl Sync for Global {}

unsafe impl GlobalAlloc for Global {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = calc_alloc_size(layout);
        // When required size is page size or more, just make PageMap allocate the proper size.
        if size >= PAGE_SIZE {
            return PAGE_MAP.allocate(size / PAGE_SIZE);
        }

        // Calculate the index of caches from `size`. `size` is assumed a power of two.
        let index = |size: usize| (size / WORD_SIZE).ilog2() as usize;

        let _lock = self.lock.lock();
        let caches = unsafe { &mut *self.caches.get() };
        let start = unsafe { &mut *self.start.get() };
        let end = unsafe { &mut *self.end.get() };

        if let Some(cache) = caches[index(size)].pop_next() {
            return (cache as *mut usize).cast();
        }

        // If there is no cache for `size` bytes, allocate new memory with caching memory until
        // `start` gets properly aligned.
        loop {
            // If there is no space to allocate, let PageMap allocate a page.
            if *start >= *end {
                *start = PAGE_MAP.allocate(1) as _;
                *end = *start + PAGE_SIZE as u64;
            }

            let align_pow = start.trailing_zeros().min(size.trailing_zeros());
            let cache_size = 1 << align_pow;
            let ptr = *start as *mut u8;
            *start += cache_size as u64;
            if cache_size == size {
                return ptr;
            } else {
                // Safety:
                //   * `ptr` is valid.
                //       ** `ptr` is not null because PageMap does not allocate it.
                //       ** Others will not access to `ptr` because we now have lock and update
                //            `start`.
                //   * The `ptr` is a raw pointer whose size is word size.
                //   * `ptr` is properly aligned because we take all `ptr` is aligned to word size
                //       or more.
                unsafe { caches[index(cache_size)].push_ptr(ptr.cast()) }
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = calc_alloc_size(layout);
        if size >= PAGE_SIZE {
            unsafe { PAGE_MAP.free(ptr, size / PAGE_SIZE) };
        } else {
            let _lock = self.lock.lock();
            let caches = unsafe { &mut *self.caches.get() };
            let index = (size / WORD_SIZE).ilog2() as usize;
            unsafe { caches[index].push_ptr(ptr as *mut usize) };
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        // Since allocation size is not the same as `layout.size()`, `layout.size()` and `new_size`
        // can be the same. When so, we do not do anything.
        if calc_alloc_size(layout) == calc_alloc_size(new_layout) {
            ptr
        } else {
            // Same implementation as the default.
            let new_ptr = unsafe { self.alloc(new_layout) };
            if !new_ptr.is_null() {
                unsafe {
                    ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
                    self.dealloc(ptr, layout);
                }
            }
            new_ptr
        }
    }
}

/// Holds page map info, allocate and free pages with using buddy system.
pub struct PageMap {
    /// Table of [PageBlock] whose orders are 2^0, 2^1, .., 2^{[`MAX_ORDER`]}.
    table: UnsafeCell<[Option<&'static mut PageBlock>; MAX_ORDER + 1]>,
    /// Linked list holding unused [`PageBlock`]s.
    cache: UnsafeCell<Cache<PageBlock>>,
    /// Lock to operate [PageMap] without race conditions.
    lock: InterruptFreeMutex<()>,
    /// Memory map given by UEFI.
    memmap: UnsafeCell<Option<&'static mut MemoryMap>>,
}

// Safety: `PageMap` does not provide any method to directly change interior values. We take
//     lock and disables interrupts when change values in methods. This guarantees there is no race
//     condition.
unsafe impl Sync for PageMap {}

impl PageMap {
    /// Initializes [PageMap] with `memmap` and sets straight page mapping for kernel. See
    /// [`init_straight_mapping`](paging::init_straight_mapping) for more information.
    ///
    /// Considers [MemoryType::BOOT_SERVICES_CODE], [MemoryType::BOOT_SERVICES_DATA] and
    /// [MemoryType::CONVENTIONAL] are usable.
    ///
    /// # Safety
    ///
    /// Since [PageMap] does not save whether it is initialized, causes UB if you call this more
    /// than once.
    ///
    /// After this method returns, virtual address below `0xFFFF_8000_0000_0000` is never usable.
    pub unsafe fn init(
        &self,
        memmap: &'static mut MemoryMap,
        runtime: SystemTable<Runtime>,
    ) -> SystemTable<Runtime> {
        paging::init_straight_mapping();

        for index in 0..usize::MAX {
            let Some(desc) = memmap.get_mut(index) else {
                break;
            };
            desc.virt_start = paging::pyhs_to_virt(desc.phys_start)
                .map(|addr| addr.addr)
                .unwrap_or(0);
        }

        let lock = self.lock.lock();

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

            // Don't alloate address around 0 to avoid confusing with null pointer.
            // Virtual address is 0 means that there is no mapping from the physical address, so
            // skip it.
            if desc.virt_start == 0 {
                continue;
            }

            // If `desc` is the continuation of the block considering, connect them.
            if block_start + (page_count * PAGE_SIZE) as u64 == desc.virt_start {
                page_count += desc.page_count as usize;
                continue;
            }

            self.register_continuous_pages(block_start, page_count, true);

            block_start = desc.virt_start;
            page_count = desc.page_count as _;
        }

        self.register_continuous_pages(block_start, page_count, true);
        drop(lock);

        let runtime = {
            // Now we can use `Vec` because memmap setting is done and the lock is dropped.
            let mut memmap: Vec<_> = memmap.entries().copied().collect();
            unsafe {
                // Fail when `runtime` address is over 512 GB, but this situation is not supported.
                let new_runtime_addr =
                    paging::pyhs_to_virt(runtime.get_current_system_table_addr())
                        .unwrap()
                        .addr;
                // Safety: Memory map is properly set.
                runtime
                    .set_virtual_address_map(&mut memmap[..], new_runtime_addr)
                    .unwrap()
            }
        };

        // Drop straight mapping where physical and virtual addresses are the same because now
        // there is no same virtual address as physical one.
        let _lock = self.lock.lock();
        // Safety: This is safe because just mapping with new page map.
        let memmap = unsafe {
            &mut *(paging::pyhs_to_virt(memmap as *mut MemoryMap as _)
                .unwrap()
                .addr as *mut _)
        };
        // Safety: Lock is acquired.
        unsafe { *self.memmap.get() = Some(memmap) };
        // Dropping.
        KERNEL_PML4.as_ref().lock()[0] = PageEntry::null();

        runtime
    }

    /// Allocates `page_count` pages and returns the start address. If failed allocating, returns
    /// null.
    ///
    /// Since accepted `page_count` is one of 2^0, 2^1, ..., 2^{[`MAX_ORDER`]}, passing others
    /// causes failure.
    pub fn allocate(&self, page_count: usize) -> *mut u8 {
        let _lock = self.lock.lock();

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

    /// Frees `page_count` pages starting at `start`.
    ///
    /// # Safety
    ///
    /// - `start` must be the address allocated by [`PageMap::allocated()`] with passing
    ///     `page_count` as a argument.
    /// - You can free allocated pages just once.
    /// - You must never access freed pages.
    pub unsafe fn free(&self, start: *mut u8, page_count: usize) {
        let _lock = self.lock.lock();
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
        let _lock = self.lock.lock();
        let table = unsafe { &*self.table.get() };
        let ret = table.iter().fold(0, |acc, block| {
            acc + block
                .as_ref()
                .map(|blk| blk.list_len() * blk.page_count)
                .unwrap_or(0)
        });
        ret
    }

    /// Adds a given block that starts at `block_start` and whose size, in page, is `page_count`
    /// into [PageMap::table]. If `page_count` is greater than 2^{[`MAX_ORDER`]}, inserts them
    /// after splitting it into smaller blocks.
    fn register_continuous_pages(
        &self,
        mut block_start: u64,
        mut page_count: usize,
        is_locked: bool,
    ) {
        let _lock = if !is_locked {
            Some(self.lock.lock())
        } else {
            None
        };

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

    /// Finds the buddy of `block` and merge them recursively, then inserts a merged block to the
    /// [`PageMap::table`].
    ///
    /// If you do not need to search the buddy , call [`PageMap::insert_block()`] instead.
    ///
    /// # Safety
    ///
    /// `block.page_count` must be the power of two and no more than 2^{[`MAX_ORDER`]}.
    unsafe fn insert_block_with_merge(&self, block: &'static mut PageBlock, is_locked: bool) {
        let _lock = if !is_locked {
            Some(self.lock.lock())
        } else {
            None
        };

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

    /// Inserts `block` to the proper position of [PageMap::table].
    ///
    /// This method does not search the buddy and merge with it. If you want to merge `block` and
    /// the buddy, call [`PageMap::insert_block_with_merge()`] instead.
    ///
    /// # Safety
    ///
    /// `block.page_count` must be the power of two and no more than 2^{[`MAX_ORDER`]}.
    unsafe fn insert_block(&self, block: &'static mut PageBlock, is_locked: bool) {
        let _lock = if !is_locked {
            Some(self.lock.lock())
        } else {
            None
        };

        let table = unsafe { &mut *self.table.get() };

        let order = block.page_count.ilog2() as usize;
        let mut prev = None;
        mem::swap(&mut table[order], &mut prev);
        block.next = prev;
        table[order] = Some(block);
    }

    /// Stores memory space \[`start`, `end`) as an linked list of [`PageBlock`]s into
    /// [`Self::cache`]. Since [`PageBlock`]'s size is greater than 8 bytes, we can use its space
    /// to store the next [`PageBlock`] pointer.
    ///
    /// # Safety
    ///
    /// `start` must be properly aligned to [`PageBlock`] and must not be `0` (that is null).
    /// Memory space \[`start`, `end`) must not be used as other objects.
    unsafe fn store_as_cache(&self, mut start: u64, end: u64, is_locked: bool) {
        let _lock = if !is_locked {
            Some(self.lock.lock())
        } else {
            None
        };
        let cache = unsafe { &mut *self.cache.get() };

        let block_size = mem::size_of::<PageBlock>() as u64;
        while start + block_size < end {
            let cur = { start as *mut PageBlock };
            start += block_size;
            unsafe { cache.push_ptr(cur) };
        }
    }

    /// Pushes `block` to front of [`Self::cache`].
    fn push_cache(&self, block: &'static mut PageBlock, is_locked: bool) {
        let _lock = if !is_locked {
            Some(self.lock.lock())
        } else {
            None
        };
        let cache = unsafe { &mut *self.cache.get() };

        cache.push(block);
    }

    /// Pops [`PageBlock`] from [`Self::cache`]'s linked list if it has.
    fn pop_cache(&self, is_locked: bool) -> Option<&'static mut PageBlock> {
        let _lock = if !is_locked {
            Some(self.lock.lock())
        } else {
            None
        };
        let cache = unsafe { &mut *self.cache.get() };
        cache.pop_next()
    }

    /// Removes a block from [`Self::table`] and returns it if succeeded.
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

        let _lock = if !is_locked {
            Some(self.lock.lock())
        } else {
            None
        };
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
}

impl Debug for PageMap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let _lock = self.lock.lock();
        let table = unsafe { &*self.table.get() };
        let mut list = f.debug_list();
        for entry in table {
            list.entry(entry);
        }
        list.finish()
    }
}

/// Represents a block as continuous pages.
#[derive(Default)]
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

/// Saves unused static reference of `T` as cache that holds the pointer to the next [`Cache<T>`],
/// that is linked list.
///
/// However, the first item, head of linked list should be put on the stack, and do not use as
/// others objects.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Cache<T>(*mut Cache<T>);

impl<T> Cache<T> {
    /// Constructs a new head of a [`Cache<T>`] linked list that is point to null.
    const fn new() -> Self {
        Self(ptr::null_mut())
    }

    /// Pushes `next` to the next of the head of the linked list, `self`. In short, `next` will be
    /// the second item of the list because the head is never assumed changeable.
    ///
    /// # Panics
    ///
    /// Since `T` will used to store the next pointer of cache, this function will panic when the
    /// size of `T` is less than the word size or the align of `T` is less than the word align.
    fn push(&mut self, next: &'static mut T) {
        let word_align = mem::align_of::<usize>();
        // Check the size and the align.
        assert!(
            mem::size_of::<T>() >= WORD_SIZE,
            "size of type must be at least word size, {} bytes",
            WORD_SIZE
        );
        assert!(
            mem::align_of::<T>() >= word_align,
            "align of type must at least align of a word, {} bytes",
            word_align
        );
        // Safety:
        //   * `next` is valid because it is casted from an "static" and "exclusive" reference of
        //     `T`.
        //   * `T` has enough size and is properly aligned because of checking above.
        unsafe { self.push_ptr(next as _) };
    }

    /// Pushes `next` to the next of the head of the linked list, `self`. In short, `next` will be
    /// the second item of the list because the head is never assumed changeable.
    ///
    /// When you push a reference of `T`, use sefety version, [`Self::push()`] instead.
    ///
    /// # Safety
    ///
    /// You should guarantee conditions below:
    ///   * `next` is [valid].
    ///   * The size of `T` is no less than the word size.
    ///   * `next` is properly aligned to hold a word.
    ///
    /// [valid]: https://doc.rust-lang.org/core/ptr/index.html#safety
    unsafe fn push_ptr(&mut self, next: *mut T) {
        let next: *mut Cache<T> = next.cast();
        // Safety:
        //     * `next` is valid because of caller.
        //     * `cur` is properly aligned by caller.
        unsafe { next.write(Cache(self.0)) };
        self.0 = next;
    }

    /// Pops the pointer `self` holds, and links its next with `self`. After that, writes `value`
    /// to it and returns its static exclusive reference.
    fn pop_next_with_value(&mut self, value: T) -> Option<&'static mut T> {
        let ret = self.0;

        if ret.is_null() {
            None
        } else {
            // Safety: `ret` is properly assinged by `push_*` when `ret` is not null.
            let new_next = unsafe { (*ret).0 };
            self.0 = new_next;

            let ret: *mut T = ret.cast();
            // Safety:
            //   * `ret` is valid because it was stored in valid way and `Cache<T>` is not Send or
            //     Sync.
            //   * `ret` is, of cource, properly aligned.
            unsafe { ret.write(value) };
            Some(unsafe { &mut *ret })
        }
    }
}

impl<T: Default> Cache<T> {
    /// Pops the pointer `self` holds, and links its next with `self`. After that, writes default
    /// value to it and returns its static exclusive reference.
    fn pop_next(&mut self) -> Option<&'static mut T> {
        self.pop_next_with_value(Default::default())
    }
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculates allocation memory size by `layout`. Returned value is the power of two and no less
/// than [`WORD_SIZE`]. In short, returns the minimal value of LHS that satisfies
/// `2^n * layout.align() >= layout.size()`.
fn calc_alloc_size(layout: Layout) -> usize {
    if layout.size() <= WORD_SIZE {
        WORD_SIZE
    } else {
        // Ceil of the log2 (size / align).
        let pow = (layout.size() / layout.align() - 1)
            .checked_ilog2()
            .map(|v| v + 1)
            .unwrap_or(0);
        layout.align() << pow
    }
}
