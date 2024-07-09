#![no_std]
#![no_main]

#[export_name = "efi_main"]
fn main(_: usize, _: usize) -> ! {
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
