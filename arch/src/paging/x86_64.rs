use core::{
    mem,
    sync::atomic::{AtomicU64, Ordering::SeqCst},
};

use arbitrary_int::prelude::*;
use bitbybit::bitfield;
use bitops::{BitOps, bits_u64};
use primitives::{
    Page, PageFrame, PageSize as _, PhysAddr,
    paging::{PageAttributes, PageFrameAllocator, PhysToVirt},
};

use crate::paging::{MapError, sealed::PageSize};

use self::size::*;

#[derive(Debug)]
#[repr(transparent)]
struct RawPageEntry(AtomicU64);

// Check PageEntry size and alignment.
const _: () = {
    ["PageEntry size"][mem::size_of::<RawPageEntry>() - 8];
    ["PageEntry align"][mem::align_of::<RawPageEntry>() - 8];
};

struct PageEntry<'a> {
    raw: &'a RawPageEntry,
    level: PageLevel,
}

impl<'a> PageEntry<'a> {
    fn set_frame<S: PageSize>(&self, frame: PageFrame<S>) -> bool {
        if (self.level == PageLevel::Pt && S::SIZE == PageSize4KB::SIZE)
            || (self.level == PageLevel::Pd && S::SIZE == PageSize2MB::SIZE)
            || (self.level == PageLevel::Pdpt && S::SIZE == PageSize1GB::SIZE)
        {
            let shift = S::SIZE.ilog2();
            let mut raw = self.raw.0.load(SeqCst);
            raw.set_bit(0, true)
                .set_bits(shift.., frame.addr().as_u64() >> shift);
            true
        } else {
            false
        }
    }
}

#[bitfield(u64, debug, default = 0, forbid_overlaps)]
struct Cr3 {
    // #[bits([0..=2, 5..=11])]
    // _ignored: u10,
    #[bit(3, rw)]
    pwt: bool,
    #[bit(4, rw)]
    pcd: bool,
    #[bits(12..=52, rw)]
    phys_addr: u41,
    // #[bits(53..=63)]
    // _reserved: u11,
}

#[bitfield(u49, debug, default = 0, forbit_overlaps)]
struct PeCommon {
    #[bit(0, rw)]
    present: bool,
    #[bit(1, rw)]
    write: bool,
    #[bit(2, rw)]
    supervisor: bool,
    #[bit(3, rw)]
    pwt: bool,
    #[bit(4, rw)]
    pcd: bool,
    #[bit(5, r)]
    accessed: bool,
    #[bit(6, rw)]
    size: bool,
    #[bit(7, rw)]
    ordinary: bool,
    #[bits(8..=47, rw)]
    addr: u40,
    #[bit(48, rw)]
    xd: bool,
}

#[bitfield(u64, debug, default = 0, forbid_overlaps)]
struct PageTableReferenceEntry {
    #[bits([0..=5, 7, 11, 12..=51, 63], rw)]
    common: PeCommon,
}

#[bitfield(u64, debug, default = 0, forbid_overlaps)]
struct PageTableMapEntry {
    #[bits([0..=5, 7, 11, 12..=51, 63], rw)]
    common: PeCommon,
    #[bit(6, r)]
    dirty: bool,
    #[bit(8, rw)]
    global: bool,
}

#[repr(C, align(4096))]
#[derive(Debug)]
struct RawPageTable([RawPageEntry; 512]);

// Check PageTable size and alignment.
const _: () = {
    ["PageTable size"][mem::size_of::<RawPageTable>() - PageSize4KB::SIZE];
    ["PageTable align"][mem::align_of::<RawPageTable>() - PageSize4KB::SIZE];
};

struct PageTable<'a> {
    raw: &'a RawPageTable,
    level: PageLevel,
}

impl<'a> PageTable<'a> {
    fn from_frame<S>(frame: PageFrame<S>, translator: &dyn PhysToVirt) -> Option<Self>
    where
        S: PageSize,
    {
        translator.phys_to_virt(frame.addr()).map(|virt| {
            debug_assert!(virt.is_aligned(PageSize4KB::SIZE));
            Self {
                // Safety: aligned
                raw: unsafe { virt.as_ref() },
                level: PageLevel::Pml4,
            }
        })
    }

    fn next<S: PageSize>(
        &self,
        index: PageTableIndex,
        translator: &dyn PhysToVirt,
    ) -> Result<Self, NextError> {
        if (S::SIZE == PageSize4KB::SIZE && self.level == PageLevel::Pt)
            || (S::SIZE == PageSize2MB::SIZE && self.level == PageLevel::Pd)
            || (S::SIZE == PageSize1GB::SIZE && self.level == PageLevel::Pdpt)
        {
            return Err(NextError::NoNextTable);
        }
        let next_level = self.level.next().unwrap();

        let entry = self.raw.0[index.0].0.load(SeqCst);
        if !bits_u64::get_bit(entry, 0) {
            Err(NextError::NoNextTable)
        } else {
            translator
                .phys_to_virt(PhysAddr::new(entry & !(PageSize4KB::SIZE as u64 - 1)))
                .map(|virt| {
                    debug_assert!(virt.is_aligned(PageSize4KB::SIZE));
                    Self {
                        // Safety: aligned
                        raw: unsafe { virt.as_ref() },
                        level: next_level,
                    }
                })
                .ok_or(NextError::CannotAccess)
        }
    }

    fn entry<S: PageSize>(&self, index: PageTableIndex) -> Option<PageEntry<'_>> {
        if (S::SIZE == PageSize4KB::SIZE && self.level == PageLevel::Pt)
            || (S::SIZE == PageSize2MB::SIZE && self.level == PageLevel::Pd)
            || (S::SIZE == PageSize1GB::SIZE && self.level == PageLevel::Pdpt)
        {
            Some(PageEntry {
                raw: &self.raw.0[index.0],
                level: self.level,
            })
        } else {
            None
        }
    }
}

enum NextError {
    CannotAccess,
    NoNextTable,
}

struct PageTableIndex(usize);

#[derive(Debug)]
pub struct PageMapper<T> {
    pml4: PageFrame<PageSize4KB>,
    translator: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PageLevel {
    Pt,
    Pd,
    Pdpt,
    Pml4,
    // Pml5,
}

impl PageLevel {
    fn next(self) -> Option<Self> {
        match self {
            PageLevel::Pt => None,
            PageLevel::Pd => Some(PageLevel::Pt),
            PageLevel::Pdpt => Some(PageLevel::Pd),
            PageLevel::Pml4 => Some(PageLevel::Pdpt),
        }
    }
}

impl<T: PhysToVirt + Send + Sync> PageMapper<T> {
    pub const fn new(base: PhysAddr, translator: T) -> Self {
        let pml4 = PageFrame::new(base);
        Self { translator, pml4 }
    }

    // 戻り値でしたいこと:
    //   - マップされたページにアクセスするためには flush() の呼び出しを必須とする
    //     - ただし他の結果を連結させて 1 度の呼び出しにすることは可能
    //   - 実際にマップされた際の属性を返す
    pub fn map<A, S>(
        &self,
        frame: PageFrame<S>,
        page: Page<S>,
        _attributes: PageAttributes,
        _allocator: &A,
    ) -> Result<Page<S>, MapError>
    where
        A: PageFrameAllocator,
        S: PageSize,
    {
        let frame_addr = frame.addr().as_u64();

        let final_level = match S::SIZE {
            size if size == const { PageSize4KB::SIZE } => PageLevel::Pt,
            size if size == const { PageSize2MB::SIZE } => PageLevel::Pd,
            size if size == const { PageSize1GB::SIZE } => PageLevel::Pdpt,
            _ => unreachable!(),
        };

        let Some(mut table) = PageTable::from_frame(self.pml4, &self.translator) else {
            return Err(MapError::CannotAccess);
        };
        // let mut shift = 12 + 9 * 4;

        // let entry = loop {
        //     debug_assert!(12 <= shift);

        //     shift -= 9;
        //     let entry = &table.0[bits_u64::get_bits(frame_addr, shift, shift + 9) as usize];
        //     if (1 << shift) == S::SIZE {
        //         break entry;
        //     }

        //     let entry = entry.0.load(SeqCst);
        //     table = if let Some(table) = PageTable::from_frame(
        //         PageFrame::<S>::new(PhysAddr::new(entry >> 12 << 12)),
        //         &self.translator,
        //     ) {
        //         table
        //     } else {
        //         return Err(MapError::CannotAccess);
        //     };
        // };

        // if bits_u64::get_bit(entry.0.load(SeqCst), 0) {
        //     return Err(MapError::AlreadyMapped);
        // }

        // entry.0.store(page.start().as_usize() as u64 | 1, SeqCst);

        // Ok(page)

        unimplemented!()
    }
}

pub mod size {
    use primitives::PageSize;

    use crate::paging::sealed::PageSize as Sealed;

    /// Represents a 4 KiB page size.
    #[derive(Debug, Clone, Copy)]
    pub struct PageSize4KB;

    impl PageSize for PageSize4KB {
        const SIZE: usize = 1 << 12;
    }

    impl Sealed for PageSize4KB {}

    /// Represents a 2 MiB page size.
    #[derive(Debug, Clone, Copy)]
    pub struct PageSize2MB;

    impl PageSize for PageSize2MB {
        const SIZE: usize = 1 << 21;
    }

    impl Sealed for PageSize2MB {}

    /// Represents a 1 GiB page size.
    #[derive(Debug, Clone, Copy)]
    pub struct PageSize1GB;

    impl PageSize for PageSize1GB {
        const SIZE: usize = 1 << 30;
    }

    impl Sealed for PageSize1GB {}
}
