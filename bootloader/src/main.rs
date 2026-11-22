#![no_std]
#![no_main]

use uefi::{prelude::*, println};

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    println!("Hello, world!");
    println!("kernel: {}", env!("KERNEL_INSTALL_PATH"));

    loop {
        arch::hlt();
    }
}
