#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]

use core::{
    any, cmp,
    fmt::{Debug, Display},
    mem::{MaybeUninit, transmute},
    ptr, slice,
};

use uefi::{
    CStr16, Error, Handle, Identify, Status, cstr16, helpers, println,
    proto::{
        Protocol,
        console::gop::{self, GraphicsOutput},
        loaded_image::LoadedImage,
        media::{
            file::{File, FileAttribute, FileInfo, FileMode, FileType},
            fs::SimpleFileSystem,
        },
    },
    table::{
        Boot, Runtime, SystemTable,
        boot::{
            AllocateType, MemoryMap, MemoryType, OpenProtocolAttributes, OpenProtocolParams,
            ScopedProtocol, SearchType,
        },
    },
};
use util::{
    asmfunc,
    elf::{Elf64Ehdr, Elf64Phdr, ElfProgType},
    paging::{PAGE_SIZE, PageEntry, PageTable, VirtualAddress},
    screen::{FrameBufferInfo, PixelFormat},
};

/// kernel path in the boot device.
const KERNEL_PATH: &CStr16 = cstr16!("\\kernel");

/// Converts [Error] to [MyError].
macro_rules! error {
    ($err:expr) => {{
        $crate::MyError {
            err: $err,
            src: ::core::file!(),
            line: ::core::line!(),
        }
    }};
}

#[uefi::entry]
unsafe fn main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    if let Err(e) = helpers::init(&mut st) {
        // Printing messages requires helpers::init() done.
        // When it fails, there is no choice other than just exiting.
        return e.status();
    };
    if let Err(e) = actual_main(image, st) {
        println!("error occurs\n{}", e);
    }

    // Prevents scrolling screen.
    loop {
        asmfunc::hlt();
    }
}

unsafe fn actual_main(image: Handle, st: SystemTable<Boot>) -> Result<(), MyError> {
    // Get the root directory handle.
    let Some(loaded_handle) = get_protocol::<LoadedImage>(&st, image, image)?.device() else {
        return Err(error!(Error::new(Status::NO_MEDIA, ())));
    };
    let mut root_dir = get_protocol::<SimpleFileSystem>(&st, loaded_handle, image)?
        .open_volume()
        .map_err(|e| error!(e))?;

    // Get the kernel file handle.
    let kernel_handle = root_dir
        .open(KERNEL_PATH, FileMode::Read, FileAttribute::empty())
        .map_err(|e| error!(e))?;
    let FileType::Regular(mut kernel) = kernel_handle.into_type().map_err(|e| error!(e))? else {
        println!("{} is a directory", KERNEL_PATH);
        return Err(error!(Error::new(Status::NOT_FOUND, ())));
    };

    // Allocate temporary buffer to load whole kernel file
    // to deploy it into the propery address.
    let mut buf = [0; 1024];
    let file_info: &FileInfo = kernel
        .get_info(&mut buf)
        .map_err(|_| error!(Error::new(Status::BUFFER_TOO_SMALL, ())))?;
    let num_tmp_pages = (file_info.file_size() as usize).div_ceil(PAGE_SIZE);
    let tmp_addr = st
        .boot_services()
        .allocate_pages(
            AllocateType::AnyPages,
            // Allocating BOOT_SERVICES_DATA for temp data eliminates the need to free pages.
            MemoryType::BOOT_SERVICES_DATA,
            num_tmp_pages,
        )
        .map_err(|e| error!(e))?;
    let buf = slice::from_raw_parts_mut(tmp_addr as *mut _, num_tmp_pages * PAGE_SIZE);
    kernel.read(buf).map_err(|e| error!(e))?;

    // Get address info from ELF and programe headers.
    let elf_header = &*(buf.as_ptr() as *const Elf64Ehdr);
    let elf_phdrs = slice::from_raw_parts(
        (tmp_addr + elf_header.phoff) as *const Elf64Phdr,
        elf_header.phnum as _,
    );
    // Calculate the start and end addresses between which the kernel will be loaded.
    let mut start = u64::MAX;
    let mut end = 0;
    for phdr in elf_phdrs {
        if phdr.ty == ElfProgType::Load {
            start = start.min(phdr.vaddr);
            end = end.max(phdr.vaddr + phdr.memsz);
        }
    }

    // Allocate memory to set page tables.
    let current_pml4 = asmfunc::get_cr3();
    let current_pml4 = &*(current_pml4 as *const PageTable);
    let new_pml4 = st
        .boot_services()
        .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1)
        .map_err(|e| error!(e))?;
    let new_pml4 = &mut *(new_pml4 as *mut PageTable);
    for (current, new) in current_pml4.iter().zip(new_pml4.iter_mut()) {
        *new = *current;
    }

    // Allocate memory for deploying the kernel at its proper address.
    let num_pages = (end - start).div_ceil(PAGE_SIZE as _);
    let kernel_virt_head = start & !0xfff;
    let kernel_phys_head = st
        .boot_services()
        .allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_CODE,
            num_pages as _,
        )
        .map_err(|e| error!(e))?;

    // Copy temporary data to right place and set proper page tables.
    for phdr in elf_phdrs {
        if phdr.ty == ElfProgType::Load {
            // Copy data.
            let phaddr = kernel_phys_head + phdr.vaddr - kernel_virt_head;
            ptr::copy_nonoverlapping(
                (tmp_addr + phdr.offset) as *const u8,
                phaddr as *mut u8,
                phdr.filesz as _,
            );
            // Fill left bytes with 0.
            ptr::write_bytes(
                (phaddr + phdr.filesz) as *mut u8,
                0,
                (phdr.memsz - phdr.filesz) as _,
            );
            // Set page tables.
            // Required page size should be calculated from a start of a page.
            let offset = phdr.vaddr & 0xfff;
            let mut num_pages = (offset + phdr.memsz).div_ceil(PAGE_SIZE as _);
            let mut paddr = phaddr & !0xfff;
            let mut vaddr = phdr.vaddr & !0xfff;
            while num_pages > 0 {
                let registerd_pages = set_page_tables(
                    &st,
                    new_pml4,
                    vaddr.into(),
                    paddr,
                    num_pages as _,
                    phdr.flags.writable(),
                )?;
                num_pages -= registerd_pages;
                paddr += registerd_pages * PAGE_SIZE as u64;
                vaddr += registerd_pages * PAGE_SIZE as u64;
            }
        }
    }
    println!(
        "succeeded loading kernel to {:08x}-{:08x}",
        kernel_phys_head,
        kernel_phys_head + end - start
    );

    // Get frame buffer info.
    // We need to get handle for taking GraphicsOutput.
    let mut graphics_handles = [MaybeUninit::uninit(); 64];
    let handle_len = st
        .boot_services()
        .locate_handle(
            SearchType::ByProtocol(&GraphicsOutput::GUID),
            Some(&mut graphics_handles[..]),
        )
        .map_err(|e| error!(e))?;
    // If there is no handles for graphics, we can't get the info.
    if handle_len < 1 {
        return Err(error!(Error::new(Status::NOT_FOUND, ())));
    }

    // Now ready to ge GraphicsOutput.
    let mut graphics =
        get_protocol::<GraphicsOutput>(&st, graphics_handles[0].assume_init(), image)?;
    let mode_info = graphics.current_mode_info();
    let format = match mode_info.pixel_format() {
        gop::PixelFormat::Rgb => PixelFormat::Rgb,
        gop::PixelFormat::Bgr => PixelFormat::Bgr,
        gop::PixelFormat::Bitmask => PixelFormat::Bitmask,
        gop::PixelFormat::BltOnly => PixelFormat::Bitonly,
    };
    let fb_info = FrameBufferInfo {
        format,
        horizontal_resolution: mode_info.resolution().0,
        vertical_resolution: mode_info.resolution().1,
        pixels_per_scanline: mode_info.stride(),
        frame_buffer: graphics.frame_buffer().as_mut_ptr() as _,
    };
    drop(graphics);

    // Exit UEFI boot service to pass the control to kernel
    let (runtime_services, mut memmap) = st.exit_boot_services(MemoryType::LOADER_DATA);

    // Set new PML4.
    asmfunc::set_cr3(new_pml4 as *const _ as _);

    type EntryFn = extern "sysv64" fn(&FrameBufferInfo, &mut MemoryMap, SystemTable<Runtime>) -> !;
    let kernel_entry: EntryFn = transmute(elf_header.entry);
    kernel_entry(&fb_info, &mut memmap, runtime_services);
}

/// Get protocol `P` from boot servieces.
///
/// # Arguments
///
/// * `st` - System table to get boot services.
/// * `handle` - The handle for the protocol to open.
/// * `agent` - The handles of the calling agent.
///             For application, including loader, this is the image handle.
unsafe fn get_protocol<P: Protocol>(
    st: &SystemTable<Boot>,
    handle: Handle,
    agent: Handle,
) -> Result<ScopedProtocol<P>, MyError> {
    let params = OpenProtocolParams {
        handle,
        agent,
        controller: None,
    };
    st.boot_services()
        .open_protocol(params, OpenProtocolAttributes::GetProtocol)
        .map_err(|e| {
            println!("error with protocol {}", any::type_name::<P>());
            error!(e)
        })
}

/// Trys to set `num_pages` page from `vaddr` to `pml4` with physical address `phaddr`.
/// When `vaddr + num_pages * PAGE_SIZE` is across a PageTable boundaries, cannot set pages over
/// the boundary. To avoid failing setting, returns the number of pages properly set.
unsafe fn set_page_tables(
    st: &SystemTable<Boot>,
    pml4: &mut PageTable,
    vaddr: VirtualAddress,
    phaddr: u64,
    num_pages: usize,
    writable: bool,
) -> Result<u64, MyError> {
    let mut level_table = pml4;
    for level in (2..=4).rev() {
        if level_table[vaddr.get_level_index(level)].next().is_none() {
            let ptr = st
                .boot_services()
                .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1)
                .map_err(|e| error!(e))?;
            ptr::write_bytes(ptr as *mut u8, 0, PAGE_SIZE);
            // Since this is not PT, we set the page is writable.
            level_table[vaddr.get_level_index(level)] = PageEntry::new(ptr, true, false);
        }
        // Next page table is definitely set above, so this unwrapping always succeeds.
        level_table = level_table[vaddr.get_level_index(level)]
            .next_mut()
            .unwrap();
    }

    let num_pages_in_frame = cmp::min(num_pages, 512 - vaddr.pt_index());
    for i in 0..num_pages_in_frame {
        level_table[vaddr.pt_index() + i] =
            PageEntry::new(phaddr + (i * PAGE_SIZE) as u64, writable, false);
    }

    Ok(num_pages_in_frame as _)
}

/// Wraps [Error] to show where an error occurs.
struct MyError {
    /// Original error.
    err: Error,
    /// Source file where the error occurs.
    src: &'static str,
    /// Line number where the error occurs.
    line: u32,
}

impl Display for MyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: at {}:{}", self.err, self.src, self.line)
    }
}

impl Debug for MyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}

#[panic_handler]
fn _panic_handler(_info: &core::panic::PanicInfo) -> ! {
    loop {
        asmfunc::hlt();
    }
}
