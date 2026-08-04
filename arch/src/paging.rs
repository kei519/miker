//! Provides the architecture dependnet abilities associated with paging.

cfg_select! {
    target_arch = "x86_64" => {
        mod x86_64;
        use x86_64 as imp;
    }
    _ => {
        compile_error!("Unsupported architecture");
    }
}

use primitives::{
    Page, PageFrame, PageSize, PhysAddr,
    paging::{PageAttributes, PageFrameAllocator, PhysToVirt},
};

pub use imp::size::*;

pub struct PageMapper<Acc>(imp::PageMapper<Acc>);

impl<T: PhysToVirt> PageMapper<T> {
    pub fn new(base: PhysAddr, translator: T) -> Self {
        PageMapper(imp::PageMapper::new(base, translator))
    }

    pub fn map<A, S>(
        &self,
        frame: PageFrame<S>,
        page: Page<S>,
        attributes: PageAttributes,
        count: usize,
        allocator: &A,
    ) -> Result<Page<S>, MapError>
    where
        A: PageFrameAllocator,
        S: PageSize + sealed::PageSize,
    {
        self.0.map(frame, page, attributes, count, allocator)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MapError {
    AlreadyMapped,
}

mod sealed {
    pub trait PageSize {}
}
