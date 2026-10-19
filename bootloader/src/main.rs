#![no_main]

use uefi::prelude::*;

#[entry]
fn main() -> Status {
    println!("Hello, world!");
    Status::SUCCESS
}
