#![no_std]
#![no_main]

#[unsafe(no_mangle)]
fn _start() {
    loop {
        arch::hlt();
    }
}

#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        arch::hlt();
    }
}
