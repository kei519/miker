#![no_std]
#![no_main]

#[unsafe(no_mangle)]
fn _start() {
    loop {
        hlt()
    }
}

fn hlt() {
    use core::arch::asm;

    cfg_select! {
        target_arch = "x86_64" => {
            // SAFETY: This is a supervisor program.
            unsafe { asm!("hlt") };
        }
        target_arch = "aarch64" => {
            // SAFETY: This is a supervisor program.
            unsafe { asm!("wfi") }
        }
    }
}

#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        hlt()
    }
}
