#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

mod interrupt;
mod memmap;
mod paging;
mod screen;
mod task;

use core::fmt::Write as _;

use alloc::format;
use task::TASK_MANAGER;
use uefi::table::{boot::MemoryMap, Runtime, SystemTable};
use util::apic;
use util::paging::PAGE_SIZE;
use util::{
    asmfunc,
    buffer::StrBuf,
    descriptor::{self, SegmentDescriptor, SegmentType, SystemDescriptor, GDT},
    error::Result,
    graphics::GrayscalePrint as _,
    screen::{FrameBufferInfo, GrayscaleScreen},
    sync::OnceStatic,
};

use {memmap::PAGE_MAP, screen::FB_INFO};

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
    add rsp, 1 * 1024 * 1024
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
    let stack_for_timer_interrupt = PAGE_MAP.allocate(2);
    if stack_for_timer_interrupt.is_null() {
        panic!("Failed to allocate 2 pages");
    }

    // Set a new GDT for kernel.
    let mut gdt = GDT::new(5);
    gdt.set(1, SegmentDescriptor::new(SegmentType::code(true, false), 0))?;
    gdt.set(2, SegmentDescriptor::new(SegmentType::data(true, false), 0))?;

    TSS.init(descriptor::TSS::new(
        &[],
        &[unsafe { stack_for_timer_interrupt.byte_add(2 * PAGE_SIZE) } as _],
    ));
    gdt.set(3, SystemDescriptor::new_tss(TSS.as_ref(), 0))?;

    gdt.register();
    asmfunc::set_cs_ss(1 << 3, 2 << 3);
    asmfunc::set_ds_all(0);
    asmfunc::load_tr(3 << 3);

    interrupt::init()?;

    TASK_MANAGER.init();

    apic::set_lvt_timer(0x40, false, true);
    apic::set_divide_config(1);
    apic::set_init_count(100000);
    asmfunc::sti();

    TASK_MANAGER.register_new_task(count, 1, 1 << 3, 2 << 3);

    loop {
        asmfunc::hlt();
    }
}

fn count() {
    let mut i = 0;
    let mut screen = GrayscaleScreen::new(FB_INFO.as_ref().clone());
    loop {
        screen.print(&format!("count: {:010}", i), (0, 0));
        i += 1;
        unsafe { core::arch::asm!("hlt") };
    }
}

#[panic_handler]
fn _panic_handler(info: &core::panic::PanicInfo) -> ! {
    asmfunc::cli();
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
