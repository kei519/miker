#![no_std]
#![no_main]

use core::{
    cmp,
    mem::{self, MaybeUninit},
    slice,
};

use primitives::elf::Elf64;
use uefi::{
    CStr16,
    boot::{AllocateType, MemoryType, SearchType},
    data_types::PhysicalAddress,
    mem::memory_map::MemoryMap as _,
    prelude::*,
    println,
    proto::{
        loaded_image::LoadedImage,
        media::{
            file::{File, FileAttribute, FileInfo, FileMode},
            fs::SimpleFileSystem,
        },
        rng::Rng,
    },
};

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let image_handle = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).unwrap();
    let mut fs_handle =
        boot::open_protocol_exclusive::<SimpleFileSystem>(image_handle.device().unwrap()).unwrap();
    let mut root = fs_handle.open_volume().unwrap();

    // Acquire the handle of kernel image.
    let kernel_path = env!("KERNEL_INSTALL_PATH");
    let mode = FileMode::Read;
    let attr = FileAttribute::empty();
    let mut file_handle = root.open(cstr16!("."), mode, attr).unwrap();
    for item in kernel_path.split('/') {
        let Some(mut dir) = file_handle.into_directory() else {
            panic!("One of node to {} is a file", kernel_path);
        };
        let mut name_buf = [0u16; 64];
        let child_handle = dir
            .open(
                CStr16::from_str_with_buf(item, &mut name_buf).unwrap(),
                FileMode::Read,
                FileAttribute::empty(),
            )
            .unwrap();
        file_handle = child_handle;
    }
    let Some(mut kernel) = file_handle.into_regular_file() else {
        panic!("{} is not a file", kernel_path);
    };

    // Read kernel image to heap.
    let kernel_file_size = {
        let mut kernel_fle_info_buf = [0u8; 512];
        let kernel_file_info = kernel
            .get_info::<FileInfo>(&mut kernel_fle_info_buf)
            .unwrap();
        kernel_file_info.file_size() as usize
    };
    let kernel_size_in_page = kernel_file_size.div_ceil(boot::PAGE_SIZE);
    let kernel_read_buf_ptr = boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::BOOT_SERVICES_DATA,
        kernel_size_in_page,
    )
    .unwrap();
    // SAFETY: allocated
    let kernel_read_buf = unsafe {
        slice::from_raw_parts_mut(
            kernel_read_buf_ptr.as_ptr(),
            kernel_size_in_page * boot::PAGE_SIZE,
        )
    };
    let read_size = kernel.read(kernel_read_buf).unwrap();
    let elf = Elf64::new(&kernel_read_buf[..read_size]).unwrap();
    println!("{elf:#?}");
    for seg in elf.segments() {
        let seg = seg.unwrap();
        println!(
            "{:016x}..{:016x} (align: {:x}): {:x?} {:?} (file_size: {})",
            seg.vaddr,
            seg.vaddr + seg.mem_size,
            seg.align,
            seg.ty,
            seg.flags,
            seg.data.len()
        );
    }
    let (start, end) = elf
        .segments()
        .fold((u64::MAX, u64::MIN), |(start, end), seg| {
            let seg = seg.unwrap();
            if seg.mem_size == 0 {
                (start, end)
            } else {
                (
                    cmp::min(start, seg.vaddr),
                    cmp::max(end, seg.vaddr + seg.mem_size),
                )
            }
        });
    println!("{start:x}..{end:x}");
    let (start, end) = (
        start / (boot::PAGE_SIZE as u64) * boot::PAGE_SIZE as u64,
        end.next_multiple_of(boot::PAGE_SIZE as u64),
    );
    println!("{start:x}..{end:x}");

    let rng_handle = {
        let mut buf = [MaybeUninit::uninit(); 16];
        match boot::locate_handle(SearchType::from_proto::<Rng>(), &mut buf) {
            Ok(handles) => handles.first().copied(),
            Err(e) if e.status() == Status::NOT_FOUND => None,
            Err(e) => panic!("{e}"),
        }
    };

    let allocate_type = if let Some(rng_handle) = rng_handle {
        let mut rng = boot::open_protocol_exclusive::<Rng>(rng_handle).unwrap();
        let mut aslr_base_ne = [0u8; mem::size_of::<u64>()];
        rng.get_rng(None, &mut aslr_base_ne).unwrap();
        let aslr_base = PhysicalAddress::from_ne_bytes(aslr_base_ne);
        let page_base = (aslr_base >> 12 << 12) % (64 << 20);
        AllocateType::Address(page_base)
    } else {
        AllocateType::AnyPages
    };
    println!("aslr_base: {:x?}", allocate_type);

    let mem_map = boot::memory_map(MemoryType::BOOT_SERVICES_DATA).unwrap();
    println!("{mem_map:x?}");

    for desc in mem_map.entries() {
        println!("{desc:x?}");
    }

    let page = boot::allocate_pages(
        allocate_type,
        MemoryType::BOOT_SERVICES_DATA,
        (end - start) as usize / boot::PAGE_SIZE,
    );
    println!("{:x?}", page);

    loop {
        arch::hlt();
    }
}
