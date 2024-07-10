#![no_std]
#![no_main]

use core::slice;

use util::FrameBufferInfo;

#[no_mangle]
fn _start(fb_info: &FrameBufferInfo) {
    // Draw whole screen with green.
    let fb = unsafe {
        slice::from_raw_parts_mut(
            fb_info.frame_buffer as *mut u32,
            fb_info.pixels_per_scanline * fb_info.vertical_resolution,
        )
    };
    for pixel in fb {
        *pixel = 0x008800;
    }
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
