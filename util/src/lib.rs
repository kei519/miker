//! Utility library widely used in MIKer.

#![cfg_attr(not(test), no_std)]
#![deny(missing_docs)]

mod font_data;

pub mod acpi;
pub mod bitfield;
pub mod buffer;
pub mod elf;
pub mod graphics;
pub mod paging;
pub mod pci;
pub mod screen;
pub mod sync;
