#![no_std]
#![no_main]

use core::fmt::Write as _;

use alloc::{boxed::Box, vec::Vec};
use alloc::{format, vec};
use kernel::paging;
use kernel::{interrupt, memmap::PAGE_MAP, screen::FB_INFO};
use uefi::table::{boot::MemoryMap, Runtime, SystemTable};
use util::{
    asmfunc,
    buffer::StrBuf,
    descriptor::{self, SegmentDescriptor, SegmentType, SystemDescriptor, GDT},
    error::Result,
    graphics::GrayscalePrint as _,
    screen::{FrameBufferInfo, GrayscaleScreen},
    sync::OnceStatic,
};

extern crate alloc;

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

/// Global TSS.
static TSS: OnceStatic<descriptor::TSS> = OnceStatic::new();

#[no_mangle]
fn main(fb_info: &FrameBufferInfo, memmap: &'static mut MemoryMap, runtime: SystemTable<Runtime>) {
    // Safety: There is one processor running and this is the first time to initialize.
    //   There is only `fb_info` that uses first half parts of virtual address. So, all we have to
    //   do is just mapping it properly.
    let runtime = unsafe { PAGE_MAP.init(memmap, runtime) };
    let fb_info = FrameBufferInfo {
        frame_buffer: paging::pyhs_to_virt(fb_info.frame_buffer as _)
            .unwrap()
            .addr as _,
        ..fb_info.clone()
    };
    FB_INFO.init(fb_info);

    match main2(runtime) {
        Ok(_) => unreachable!(),
        Err(e) => {
            let mut screen = GrayscaleScreen::new(FB_INFO.as_ref().clone());
            screen.print(&format!("{}", e), (0, 0));
            loop {
                asmfunc::hlt();
            }
        }
    }
}

// NOTE: Never return `Ok()`.
fn main2(_runtime: SystemTable<Runtime>) -> Result<()> {
    let mut screen = GrayscaleScreen::new(FB_INFO.as_ref().clone());

    // Set a new GDT for kernel.
    let mut gdt = GDT::new(5);
    gdt.set(1, SegmentDescriptor::new(SegmentType::code(true, false), 0))?;
    gdt.set(2, SegmentDescriptor::new(SegmentType::data(true, false), 0))?;

    // Empty TSS.
    TSS.init(descriptor::TSS::new(&[], &[]));
    gdt.set(3, SystemDescriptor::new_tss(TSS.as_ref(), 0))?;

    gdt.register();
    asmfunc::set_cs_ss(1 << 3, 2 << 3);
    asmfunc::set_ds_all(0);
    asmfunc::load_tr(3 << 3);

    interrupt::init()?;

    let mut buf = [0; 4 * 4096];
    let mut buf = StrBuf::new(&mut buf);

    let num_free_pages = PAGE_MAP.free_pages_count();
    let _ = writeln!(buf, "Free: {} pages", num_free_pages);
    let _ = writeln!(buf);

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

    let _ = writeln!(buf, "Large allocation");
    {
        let mut v = vec![94; 1028];
        let _ = writeln!(buf, "v: {:p}, cap: {}", v.as_ptr(), v.capacity());
        v.extend_from_slice(&[1024; 10]);
        let _ = writeln!(buf, "v: {:p}, cap: {}", v.as_ptr(), v.capacity());
        v.extend_from_slice(&[512; 10]);
        let _ = writeln!(buf, "v: {:p}, cap: {}", v.as_ptr(), v.capacity());
        let _ = writeln!(buf);
        for (i, item) in v.into_iter().enumerate() {
            match i {
                0..=1027 => assert_eq!(item, 94),
                1028..=1037 => assert_eq!(item, 1024),
                1038..=1047 => assert_eq!(item, 512),
                _ => unreachable!(),
            }
        }
    }

    let _ = writeln!(buf, "First small some allocations");
    {
        let mut v = vec![0; 16];
        let _ = writeln!(buf, "v: {:p}, cap: {}", v.as_ptr(), v.capacity());
        v.extend_from_slice(&[1024; 16]);
        let _ = writeln!(buf, "v: {:p}, cap: {}", v.as_ptr(), v.capacity());
        let us: Vec<_> = (0..10).map(|_| v.clone()).collect();
        for (i, u) in us.iter().enumerate() {
            let _ = writeln!(buf, "u{i}: {:p}, cap: {}", u.as_ptr(), u.capacity());
        }
        let _ = writeln!(buf);
    }

    let _ = writeln!(buf, "Second some small and large allocations");
    {
        let mut v = vec![0i8; 16];
        let _ = writeln!(buf, "v: {:p}, cap: {}", v.as_ptr(), v.capacity());
        v.extend_from_slice(&[i8::MIN; 16]);
        let _ = writeln!(buf, "v: {:p}, cap: {}", v.as_ptr(), v.capacity());
        let us: Vec<_> = (0..10)
            .map(|_| {
                let hoge = vec![0; 64];
                core::hint::black_box(&hoge);
                v.clone()
            })
            .collect();
        for (i, u) in us.iter().enumerate() {
            let _ = writeln!(buf, "u{i}: {:p}, cap: {}", u.as_ptr(), u.capacity());
        }
        let _ = writeln!(buf);
    }

    let _ = writeln!(buf, "Third some small allocations");
    {
        let mut v = vec![0i8; 16];
        let _ = writeln!(buf, "v: {:p}, cap: {}", v.as_ptr(), v.capacity());
        v.extend_from_slice(&[i8::MIN; 16]);
        let _ = writeln!(buf, "v: {:p}, cap: {}", v.as_ptr(), v.capacity());
        let us: Vec<_> = (0..10).map(|_| v.clone()).collect();
        for (i, u) in us.iter().enumerate() {
            let _ = writeln!(buf, "u{i}: {:p}, cap: {}", u.as_ptr(), u.capacity());
        }
        let _ = writeln!(buf);
    }

    let _ = writeln!(buf, "Some box allocations");
    {
        let b = Box::new(0u8);
        let _ = writeln!(buf, "b: {:p}", Box::into_raw(b));
        let us: Vec<_> = (0..10).map(|_| Box::new(12u16)).collect();
        for (i, b) in us.into_iter().enumerate() {
            let _ = writeln!(buf, "b{i}: {:p}", Box::into_raw(b));
        }
        let _ = writeln!(buf);
    }

    let _ = writeln!(buf, "Free: {} pages", PAGE_MAP.free_pages_count());
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
