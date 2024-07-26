//! Utility library widely used in MIKer.

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod font_data;

pub mod acpi;
pub mod apic;
pub mod asmfunc;
pub mod bitfield;
pub mod buffer;
pub mod descriptor;
pub mod elf;
pub mod graphics;
pub mod interrupt;
pub mod paging;
pub mod pci;
pub mod screen;
pub mod sync;

#[cfg(feature = "alloc")]
pub mod error;

pub use macros::*;
