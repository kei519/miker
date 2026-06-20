#![allow(missing_docs)]

use uefi::table::{Runtime, SystemTable, boot::MemoryMap};

use crate::screen::FrameBufferInfo;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KernelArgType {
    FrameBuffer,
    MemMap,
    RuntimeTable,
    End = u32::MAX,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct KernelArgHeader {
    ty: KernelArgType,
    size: u32,
}

#[derive(Debug)]
pub enum KernelArg {
    FrameBuffer(&'static FrameBufferInfo),
    MemMap(&'static mut MemoryMap),
    RuntimeTable(SystemTable<Runtime>),
}
