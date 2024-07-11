#![no_std]
#![no_main]

use core::fmt::Write as _;

use uefi::table::boot::MemoryMap;
use util::{
    buffer::StrBuf,
    graphics::{GrayscalePixelWrite as _, GrayscalePrint as _},
    screen::{FrameBufferInfo, GrayscaleScreen},
    sync::OnceStatic,
};

static FB_INFO: OnceStatic<FrameBufferInfo> = OnceStatic::new();

#[no_mangle]
fn _start(fb_info: &FrameBufferInfo, memmap: &MemoryMap) {
    let mut screen = GrayscaleScreen::new(fb_info.clone());
    FB_INFO.init(fb_info.clone());

    // Display memmap
    let mut buf = [0; 256];
    // 1 memory descritpor length in display.
    let item_len = 60;

    let col_num = screen.range().0 / 8;
    let row_num = screen.range().1 / 16;
    let item_per_row = col_num / item_len;
    for (count, desc) in memmap.entries().enumerate().take(row_num * item_per_row) {
        let mut buf = StrBuf::new(&mut buf);
        let _ = write!(
            buf,
            "{:016x}-{:016x}: {:?}",
            desc.phys_start,
            desc.phys_start + desc.page_count * 4096,
            desc.ty,
        );
        // Calculate start position.
        let col = count / row_num;
        let row = count % row_num;
        screen.print(buf.to_str(), (col * item_len * 8, row * 16));
    }

    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}

#[panic_handler]
fn _panic_handler(info: &core::panic::PanicInfo) -> ! {
    if FB_INFO.is_initialized() {
        let mut screen = GrayscaleScreen::new(FB_INFO.as_ref().clone());
        let mut buf = [0; 4 * 1024];
        let mut buf = StrBuf::new(&mut buf);
        let _ = write!(buf, "{:#}", info);
        screen.print(buf.to_str(), (0, 0))
    }

    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}
