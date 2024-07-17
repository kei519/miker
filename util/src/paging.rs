//! Provides paging supports.

use core::{
    ops::{Index, IndexMut},
    slice,
};

use crate::bitfield::BitField as _;

/// Represents the size of a page.
pub const PAGE_SIZE: usize = 4096;

/// Represents PML4, PDP, PD, PT and a page pointed by a PT.
#[repr(C, align(4096))]
#[derive(Debug, Clone)]
pub struct PageTable([PageEntry; 512]);

impl PageTable {
    /// Constructs a new empty [PageTable].
    pub const fn new() -> Self {
        Self([PageEntry::null(); 512])
    }

    /// Returns an iterator over every [PageEntry].
    pub fn iter(&self) -> slice::Iter<'_, PageEntry> {
        self.0.iter()
    }

    /// Returns an iterator over every [PageEntry] which are modifiable.
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, PageEntry> {
        self.0.iter_mut()
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<usize> for PageTable {
    type Output = PageEntry;

    fn index(&self, index: usize) -> &Self::Output {
        Index::index(&self.0, index)
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        IndexMut::index_mut(&mut self.0, index)
    }
}

impl AsRef<[u8]> for PageTable {
    fn as_ref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self as *const _ as _, PAGE_SIZE) }
    }
}

impl AsMut<[u8]> for PageTable {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self as *mut _ as _, PAGE_SIZE) }
    }
}

/// Provides ease ways to access each page entries from a virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualAddress {
    /// Exact virtual address.
    pub addr: u64,
}

impl VirtualAddress {
    /// Constructs [VirtualAddress] from an exact address.
    pub const fn new(addr: u64) -> Self {
        Self { addr }
    }

    /// Returns each index of the `level`-th page table which the [VirtualAddress] reffers. `level`
    /// has to be in \[0, 4\]. Otherwise, returns `0`.
    ///
    /// | Level | Page table |
    /// | :---: | :--- |
    /// | 0 | Offset. |
    /// | 1 | PT. |
    /// | 2 | PD. |
    /// | 3 | PDP. |
    /// | 4 | PML4. |
    pub fn get_level_index(&self, level: u32) -> usize {
        (match level {
            0 => self.addr.get_bits(..12),
            1..=4 => {
                let start = 12 + (level - 1) * 9;
                let end = 12 + level * 9;
                self.addr.get_bits(start..end)
            }
            _ => 0,
        }) as _
    }

    /// Returns the PML4 index of the [VirtualAddress].
    pub fn pml4_index(&self) -> usize {
        self.get_level_index(4)
    }

    /// Returns the PDP index of the [VirtualAddress].
    pub fn pdp_index(&self) -> usize {
        self.get_level_index(3)
    }

    /// Returns the PD index of the [VirtualAddress].
    pub fn pd_index(&self) -> usize {
        self.get_level_index(2)
    }

    /// Returns the PT index of the [VirtualAddress].
    pub fn pt_index(&self) -> usize {
        self.get_level_index(1)
    }

    /// Returns the offset of the [VirtualAddress].
    pub fn offset(&self) -> u64 {
        self.get_level_index(0) as _
    }
}

impl From<u64> for VirtualAddress {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<VirtualAddress> for u64 {
    fn from(value: VirtualAddress) -> Self {
        value.addr
    }
}

/// Represents an entry of page maps.
///
/// | Bit | Description |
/// | :---: | :--- |
/// | 0 (P) | Present (must be 1) |
/// | 1 (R/W) | If 0, writes may not be allowd. |
/// | 2 (U/S) | If 0, user-mode accesses are not allowed. |
/// | 3 (PWT) | Page-level write-through. |
/// | 4 (PCD) | Page-level cache disable. |
/// | 5 (A) | Indicates whether this entry has been used for translation. |
/// | 6 | Ignored. |
/// | 7 (PS) | Page size |
/// | 8 (G) | If CR4.PGE = 1, determines whether the translations is global. |
/// | 11:9 | Ignored. |
/// | 51:12 | Physical address of the next structure. |
/// | 62:52 | Ignored. |
/// | 63 (XD) | If IA32_EFER.NXE = 1, execute-disable; otherwise, must be 0. |
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct PageEntry(u64);

impl PageEntry {
    /// Constructs an empty [PageEntry].
    pub const fn null() -> Self {
        Self(0)
    }

    /// Costructs a new [PageEntry] pointing `next` as a next page table or page.
    ///
    /// # Safety
    ///
    /// `next` must be a pointer to a start address of a valid page table (or page) and aligned to
    /// 4 KiB.
    pub const unsafe fn new(next: u64, writable: bool, user: bool) -> Self {
        let writable = if writable { 1 } else { 0 };
        let user = if user { 1 } else { 0 };
        let entry = (next & !0xfff) // address of next structure
            | 1 // present
            | (writable << 1)
            | (user << 2);
        Self(entry)
    }

    fn set_present(&mut self, value: bool) -> &mut Self {
        self.0.set_bit(0, value);
        self
    }

    /// Returns whether the [PageEntry] is present.
    pub fn present(&self) -> bool {
        self.0.get_bit(0)
    }

    /// Returns whether the [PageEntry] is writable.
    pub fn writable(&self) -> bool {
        self.0.get_bit(1)
    }

    /// If `value` is `true`, sets the [PageEntry] writable. Otherwise, unsets.
    pub fn set_writable(&mut self, value: bool) -> &mut Self {
        self.0.set_bit(1, value);
        self
    }

    /// Returns whether the [PageEntry] allows user-mode access.
    pub fn user(&self) -> bool {
        self.0.get_bit(2)
    }

    /// If `value` is `true`, allows user-mode accessing to the [PageEntry]. Otherwise, disallows.
    pub fn set_user(&mut self, value: bool) -> &mut Self {
        self.0.set_bit(2, value);
        self
    }

    /// Returns whether software has accessed the page table of page referenced by this
    /// [PageEntry].
    pub fn accessed(&self) -> bool {
        self.0.get_bit(5)
    }

    /// If this [PageEntry] is present and valid next address is set, returns to the next
    /// [PageTable]. Otherwise, returns `None`.
    ///
    /// Even if the next structure is page, returns as [PageTable]. You can access it as byte
    /// string with [`PageTable::as_ref`].
    pub fn next(&self) -> Option<&PageTable> {
        let next = unsafe { self.next_addr() };
        if next == 0 || !self.present() {
            None
        } else {
            Some(unsafe { &*(next as *const _) })
        }
    }

    /// If this [PageEntry] is present and valid next address is set, returns to the next
    /// [PageTable] as mutable. Otherwise, returns `None`.
    ///
    /// Even if the next structure is page, returns as [PageTable]. You can access it as byte
    /// string with [`PageTable::as_ref`] or [`PageTable::as_mut`].
    pub fn next_mut(&self) -> Option<&mut PageTable> {
        let next = unsafe { self.next_addr() };
        if next == 0 || !self.present() {
            None
        } else {
            Some(unsafe { &mut *(next as *mut _) })
        }
    }

    /// Sets `next` as the next page table or page referenced by the [PageEntry].
    pub fn set_next(&mut self, next: &PageTable) -> &mut Self {
        let next = next as *const _ as _;
        unsafe { self.set_present(true).set_next_from_addr(next) }
    }

    /// Returns the address of the next page table or page referenced by this [PageEntry].
    ///
    /// # Safety
    ///
    /// If this [PageEntry] is not present, the returned address is invalid.
    pub unsafe fn next_addr(&self) -> u64 {
        self.0.get_bits(..52) & !0xfff
    }

    /// Sets the next page table or page of the [PageEntry] from the start address `next`.
    ///
    /// # Safety
    ///
    /// `next` have to be a start address of a valid page table or page 4 KiB aligned.
    pub unsafe fn set_next_from_addr(&mut self, next: u64) -> &mut Self {
        self.0.set_bits(12..52, next.get_bits(12..));
        self
    }
}

impl Default for PageEntry {
    fn default() -> Self {
        Self::null()
    }
}
