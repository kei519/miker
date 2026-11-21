#![no_std]
#![no_main]

use uefi::{prelude::*, println};

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    println!("Hello, world!");

    loop {
        hlt();
    }
}

fn hlt() {
    use core::arch::asm;

    cfg_select! {
        target_arch = "x86_64" => {
            //SAFETY: safe
            unsafe { asm!("hlt") };
        }
        target_arch = "aarch64" => {
            //SAFETY: safe
            unsafe { asm!("wfi") }
        }
    }
}
