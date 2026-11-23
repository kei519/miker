#![no_std]
#![no_main]

static mut STACK: [u8; 1 << 20] = [0u8; _];

#[unsafe(no_mangle)]
fn _start() {
    let stack = &raw mut STACK;
    core::hint::black_box(&stack);
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
