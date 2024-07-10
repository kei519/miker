#![no_std]
#![no_main]

use uefi::table::boot::MemoryMap;
use util::{
    graphics::{GlayscalePixelWrite as _, GlayscalePrint as _},
    screen::{FrameBufferInfo, GlayscaleScreen},
};

#[no_mangle]
fn _start(fb_info: &FrameBufferInfo, _: &MemoryMap) {
    let mut screen = GlayscaleScreen::new(fb_info.clone());
    // Draw whole screen with gray.
    let size = screen.range();
    for x in 0..size.0 {
        for y in 0..size.1 {
            screen.write((x, y), 0x00);
        }
    }

    screen.print("Hello 2nd loader!", (0, 0));

    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}

#[panic_handler]
fn _panic_handler(_: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}
