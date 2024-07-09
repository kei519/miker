#![no_std]
#![no_main]

use uefi::{
    helpers, println,
    table::{Boot, SystemTable},
    Handle, Status,
};

#[uefi::entry]
fn main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    if let Err(e) = helpers::init(&mut st) {
        return e.status();
    };
    println!("Hello UEFI!");
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}

#[panic_handler]
fn _panic_handler(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}
