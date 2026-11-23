#![no_std]
#![no_main]

use core::slice;

use primitives::elf::Elf64;
use uefi::{
    CStr16,
    boot::{AllocateType, MemoryType},
    prelude::*,
    println,
    proto::{
        loaded_image::LoadedImage,
        media::{
            file::{File, FileAttribute, FileInfo, FileMode},
            fs::SimpleFileSystem,
        },
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

    loop {
        arch::hlt();
    }
}
