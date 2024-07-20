#![no_std]
#![no_main]

use core::fmt::Write as _;

use kernel::memmap::PAGE_MAP;
use uefi::table::{boot::MemoryMap, Runtime, SystemTable};
use util::{
    asmfunc,
    buffer::StrBuf,
    graphics::GrayscalePrint as _,
    screen::{FrameBufferInfo, GrayscaleScreen},
    sync::OnceStatic,
};

static FB_INFO: OnceStatic<FrameBufferInfo> = OnceStatic::new();

#[repr(align(4096))]
#[allow(dead_code)]
struct Stack([u8; 1 << 20]);

#[no_mangle]
static mut KERNEL_STACK: Stack = Stack([0; 1 << 20]);

core::arch::global_asm! { r#"
.global _start
_start:
    lea rsp, [KERNEL_STACK + rip]
    add rsp, 1 * 1024 * 1024 - 8
    call main
"#
}

#[no_mangle]
fn main(fb_info: &FrameBufferInfo, memmap: &'static MemoryMap, _runtime: SystemTable<Runtime>) {
    let mut screen = GrayscaleScreen::new(fb_info.clone());
    FB_INFO.init(fb_info.clone());
    // Safety: There is one processor running and this is the first time to initialize.
    unsafe { PAGE_MAP.init(memmap) };

    let num_free_pages = PAGE_MAP.free_pages_count();

    let mut buf = [0; 4 * 4096];
    let mut buf = StrBuf::new(&mut buf);
    let _ = write!(buf, "{:#x?}", PAGE_MAP);
    let num_row = FB_INFO.as_ref().vertical_resolution / 16;
    let col_len = 30;
    for (i, line) in buf.to_str().lines().enumerate() {
        let col = i / num_row;
        let row = i % num_row;
        screen.print(line, (col * col_len * 8, row * 16));
    }
    screen.print(buf.to_str(), (0, 0));

    // Many allocations.
    let mut starts2 = [[core::ptr::null_mut(); 11]; 20];
    for starts in &mut starts2 {
        for (order, start) in starts.iter_mut().enumerate() {
            *start = PAGE_MAP.allocate(1 << order);
            assert!(!start.is_null());
        }
    }
    let mut starts = [core::ptr::null_mut(); 11];
    for (order, start) in starts.iter_mut().enumerate() {
        *start = PAGE_MAP.allocate(1 << order);
    }
    for (order, start) in starts.into_iter().enumerate() {
        unsafe { PAGE_MAP.free(start, 1 << order) };
    }

    for starts in starts2 {
        for (order, start) in starts.into_iter().enumerate() {
            unsafe { PAGE_MAP.free(start, 1 << order) };
        }
    }

    let mut buf = [0; 128];
    let mut buf = StrBuf::new(&mut buf);
    let _ = write!(
        buf,
        "Old Free: {} pages\nCur Free: {} pages",
        num_free_pages,
        PAGE_MAP.free_pages_count(),
    );
    screen.print(buf.to_str(), (0, 0));

    loop {
        asmfunc::hlt();
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
        asmfunc::hlt();
    }
}
