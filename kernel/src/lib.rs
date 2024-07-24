//! Provides functions and structures for kernel.

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

pub mod memmap;
pub mod screen;
