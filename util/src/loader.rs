#![allow(missing_docs)]

#[cfg(feature = "alloc")]
use alloc::boxed::Box;
use uefi::table::boot::MemoryMap;

use crate::screen::FrameBufferInfo;

#[repr(C)]
pub enum BootArgEntry {
    FrameBufferInfo(FrameBufferInfo),
    MemoryMap(MemoryMap),
}

#[repr(C)]
pub struct BootArgs {
    count: usize,
    args: [BootArgEntry],
}

#[cfg(feature = "alloc")]
impl BootArgs {
    pub fn new<const N: usize>(args: [BootArgEntry; N]) -> Box<Self> {
        #[repr(C)]
        struct Generic<T: ?Sized> {
            count: usize,
            args: T,
        }

        let b: Box<Generic<[BootArgEntry]>> = Box::new(Generic { count: N, args });
        let ptr = Box::into_raw(b) as *mut BootArgs;
        unsafe { Box::from_raw(ptr) }
    }
}
