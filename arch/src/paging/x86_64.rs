use core::{marker::PhantomData, mem, sync::atomic::AtomicU64};

use primitives::{
    Page, PageFrame, PageSize, PhysAddr,
    paging::{PageAttributes, PageFrameAllocator, PhysToVirt},
};

use crate::paging::{MapError, sealed::PageSize as Sealed};

use self::size::*;

#[derive(Debug)]
#[repr(transparent)]
struct PageEntry<L, S>(AtomicU64, PhantomData<L>, PhantomData<S>);

#[repr(C, align(4096))]
#[derive(Debug)]
struct PageTable<L, S>([PageEntry<L, S>; 512]);

struct Pml4;

// Check PageTable size and alignment.
const _: () = {
    ["PageTable size"][mem::size_of::<PageTable<(), ()>>() - PageSize4KB::SIZE];
    ["PageTable align"][mem::align_of::<PageTable<(), ()>>() - PageSize4KB::SIZE];
};

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

    fn pml4(&self) -> &PageTable<Pml4, PageSize4KB> {
        let pml4 = self.translator.phys_to_virt(self.pml4.addr()).unwrap();
        // Safety:
        // * The pointer is properly aligned because pml4 is PageFrame<PageSize4KB>.
        // * pml4 is not null
        unsafe { pml4.as_mut() }
    }

    // 戻り値でしたいこと:
    //   - マップされたページにアクセスするためには flush() の呼び出しを必須とする
    //     - ただし他の結果を連結させて 1 度の呼び出しにすることは可能
    //   - 実際にマップされた際の属性を返す
    pub fn map<A, S>(
        &self,
        _frame: PageFrame<S>,
        _page: Page<S>,
        _attributes: PageAttributes,
        _count: usize,
        _allocatro: &A,
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

        // let pml4_index = _frame.addr().as_u64() >> (12 + 4 * 9);
        // self.pml4().0[pml4_index as usize];
        unimplemented!();
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
