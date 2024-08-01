//! Provides tools for screen operations.

use alloc::format;
use util::{
    graphics::GrayscalePrint as _,
    screen::{FrameBufferInfo, GrayscaleScreen},
    sync::OnceStatic,
};

/// Frame buffer information initialized at the begining of kernel.
pub static FB_INFO: OnceStatic<FrameBufferInfo> = OnceStatic::new();

pub fn drawing_task() {
    let mut i = 0;
    let mut screen = GrayscaleScreen::new(FB_INFO.as_ref().clone());
    loop {
        screen.print(&format!("count: {:010}", i), (0, 0));
        i += 1;
        unsafe { core::arch::asm!("hlt") };
    }
}
