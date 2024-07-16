#![no_std]
#![no_main]

use core::fmt::Write as _;

use uefi::table::{boot::MemoryMap, cfg::ACPI2_GUID, Runtime, SystemTable};
use util::{
    acpi::Rsdp,
    buffer::StrBuf,
    graphics::GrayscalePrint as _,
    screen::{FrameBufferInfo, GrayscaleScreen},
    sync::OnceStatic,
};

static FB_INFO: OnceStatic<FrameBufferInfo> = OnceStatic::new();

#[repr(align(4096))]
#[allow(dead_code)]
struct Stack([u8; 1 * 1024 * 1024]);

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
fn main(fb_info: &FrameBufferInfo, _memmap: &'static MemoryMap, runtime: SystemTable<Runtime>) {
    let mut screen = GrayscaleScreen::new(fb_info.clone());
    FB_INFO.init(fb_info.clone());

    let mut buf = [0; 4096];

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
    let _ = writeln!(buf, "RSDT entries:");
    for entry in rsdp.rsdt().unwrap().entries() {
        let _ = writeln!(buf, "{:?}", entry);
    }
    let _ = writeln!(buf);

    let _ = writeln!(buf, "XSDT entries:");
    for entry in rsdp.xsdt().unwrap().entries() {
        let _ = writeln!(buf, "{:?}", entry);
    }

    screen.print(buf.to_str(), (0, 0));

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
