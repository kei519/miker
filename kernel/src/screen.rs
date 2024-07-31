//! Provides tools for screen operations.

use alloc::format;
use util::{
    graphics::GrayscalePrint as _,
    screen::{FrameBufferInfo, GrayscaleScreen},
    sync::OnceStatic,
};

use crate::task::TASK_MANAGER;

/// Frame buffer information initialized at the begining of kernel.
pub static FB_INFO: OnceStatic<FrameBufferInfo> = OnceStatic::new();

pub fn drawing_task() {
    let mut i = 0;
    let mut screen = GrayscaleScreen::new(FB_INFO.as_ref().clone());
    loop {
        screen.print(&format!("count: {:010}", i), (0, 0));
        if i == 100 {
            TASK_MANAGER.sleep();
            assert_eq!(TASK_MANAGER.task_id(), 1);
        }
        i += 1;
        unsafe { core::arch::asm!("hlt") };
    }
}
