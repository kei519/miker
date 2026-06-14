#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

pub mod acpi;
pub mod driver;
pub mod interrupt;
pub mod logger;
pub mod memmap;
pub mod paging;
pub mod screen;
pub mod sync;
pub mod syscall;
pub mod task;
pub mod timer;

pub extern crate alloc;
