//! Provides tools for screen operations.

use alloc::format;
use util::{
    graphics::GrayscalePrint as _,
    screen::{FrameBufferInfo, GrayscaleScreen},
    sync::OnceStatic,
};

use crate::MUTEX_TEST;

/// Frame buffer information initialized at the begining of kernel.
pub static FB_INFO: OnceStatic<FrameBufferInfo> = OnceStatic::new();

pub fn drawing_task() {
    let mut i = 0;
    let mut screen = GrayscaleScreen::new(FB_INFO.as_ref().clone());
    loop {
        match i {
            200 => {
                assert!(MUTEX_TEST.try_lock().is_none());
                screen.print(
                    &format!("count: {:010} -> MUTEX_TEST={:?}", i, MUTEX_TEST),
                    (0, 0),
                );
            }
            201.. => {
                let lock = MUTEX_TEST.lock();
                screen.print(&format!("locked={}", lock), (0, 16));
                drop(lock);
                screen.print(
                    &format!("count: {:010} -> MUTEX_TEST={:?}", i, MUTEX_TEST),
                    (0, 0),
                );
            }
            _ => {
                screen.print(&format!("count: {:010}", i), (0, 0));
            }
        }
        i += 1;
        unsafe { core::arch::asm!("hlt") };
    }
}
