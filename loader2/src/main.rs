#![no_std]
#![no_main]

use core::fmt::Write as _;

use uefi::table::{boot::MemoryMap, cfg::ACPI2_GUID, Runtime, SystemTable};
use util::{
    acpi::Rsdp,
    buffer::StrBuf,
    graphics::{GrayscalePixelWrite as _, GrayscalePrint as _},
    screen::{FrameBufferInfo, GrayscaleScreen},
    sync::OnceStatic,
};

static FB_INFO: OnceStatic<FrameBufferInfo> = OnceStatic::new();

#[no_mangle]
fn _start(fb_info: &FrameBufferInfo, _memmap: &'static MemoryMap, runtime: SystemTable<Runtime>) {
    let mut screen = GrayscaleScreen::new(fb_info.clone());
    FB_INFO.init(fb_info.clone());

    // Display memmap
    let mut buf = [0; 4096];
    // 1 memory descritpor length in display.
    let item_len = 60;

    let row_num = screen.range().1 / 16;

    let rsdp_ptr = runtime
        .config_table()
        .iter()
        .find_map(|config| {
            if config.guid == ACPI2_GUID {
                Some(config.address)
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("There is no ACPI2 entry."));
    // Safety: UEFI pass the proper one.
    let rsdp = unsafe { Rsdp::from_ptr(rsdp_ptr.cast()).unwrap() };

    // let entry = rsdp.xsdt().unwrap().entry(0).unwrap();
    let mut buf = StrBuf::new(&mut buf);
    let _ = write!(buf, "{:#?}", rsdp);
    for (i, line) in buf.to_str().lines().enumerate() {
        let row = i % row_num;
        let col = i / row_num;
        screen.print(line, (col * item_len * 8, row * 16));
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
