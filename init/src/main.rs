#![no_std]
#![no_main]

#[unsafe(no_mangle)]
fn _init() {
    loop {}
}

#[panic_handler]
fn _panic_handler(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
