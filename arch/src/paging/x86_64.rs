use core::{
    mem,
    sync::atomic::{AtomicU64, Ordering::SeqCst},
};

use arbitrary_int::prelude::*;
use bitbybit::bitfield;
use bitops::bits_u64;
use primitives::{
    Page, PageFrame, PageSize, PhysAddr,
    paging::{PageAttributes, PageFrameAllocator, PhysToVirt},
};

use crate::paging::{MapError, sealed::PageSize as Sealed};

use self::size::*;

#[derive(Debug)]
#[repr(transparent)]
struct PageEntry(AtomicU64);

// Check PageEntry size and alignment.
const _: () = {
    ["PageEntry size"][mem::size_of::<PageEntry>() - 8];
    ["PageEntry align"][mem::align_of::<PageEntry>() - 8];
};

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
struct PageTable([PageEntry; 512]);

// Check PageTable size and alignment.
const _: () = {
    ["PageTable size"][mem::size_of::<PageTable>() - PageSize4KB::SIZE];
    ["PageTable align"][mem::align_of::<PageTable>() - PageSize4KB::SIZE];
};

impl PageTable {
    fn from_frame<'a, S>(frame: PageFrame<S>, translator: &dyn PhysToVirt) -> Option<&'a Self>
    where
        S: PageSize + Sealed,
    {
        translator.phys_to_virt(frame.addr()).map(|virt| {
            debug_assert!(virt.is_aligned(PageSize4KB::SIZE));
            // Safety: aligned
            unsafe { virt.as_ref() }
        })
    }
}

#[derive(Debug)]
pub struct PageMapper<T> {
    pml4: PageFrame<PageSize4KB>,
    translator: T,
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
        S: PageSize + Sealed,
    {
        const {
            assert!(
                S::SIZE == PageSize4KB::SIZE
                    || S::SIZE == PageSize2MB::SIZE
                    || S::SIZE == PageSize1GB::SIZE
            )
        };

        let frame_addr = frame.addr().as_u64();

        let Some(mut table) = PageTable::from_frame(self.pml4, &self.translator) else {
            return Err(MapError::CannotAccess);
        };
        let mut shift = 12 + 9 * 4;

        let entry = loop {
            debug_assert!(12 <= shift);

            shift -= 9;
            let entry = &table.0[bits_u64::get_bits(frame_addr, shift, shift + 9) as usize];
            if (1 << shift) == S::SIZE {
                break entry;
            }

            let entry = entry.0.load(SeqCst);
            table = if let Some(table) = PageTable::from_frame(
                PageFrame::<S>::new(PhysAddr::new(entry >> 12 << 12)),
                &self.translator,
            ) {
                table
            } else {
                return Err(MapError::CannotAccess);
            };
        };

        if bits_u64::get_bit(entry.0.load(SeqCst), 0) {
            return Err(MapError::AlreadyMapped);
        }

        entry.0.store(page.start().as_usize() as u64 | 1, SeqCst);

        Ok(page)
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
