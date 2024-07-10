#![no_std]
#![no_main]

use core::{
    any,
    fmt::{Debug, Display},
    mem::{transmute, MaybeUninit},
    ptr, slice,
};

use uefi::{
    cstr16, helpers, println,
    proto::{
        console::gop::{self, GraphicsOutput},
        loaded_image::LoadedImage,
        media::{
            file::{File, FileAttribute, FileInfo, FileMode, FileType},
            fs::SimpleFileSystem,
        },
        Protocol,
    },
    table::{
        boot::{
            AllocateType, MemoryMap, MemoryType, OpenProtocolAttributes, OpenProtocolParams,
            ScopedProtocol, SearchType,
        },
        Boot, SystemTable,
    },
    CStr16, Error, Handle, Identify, Status,
};
use util::{
    elf::{Elf64Ehdr, Elf64Phdr, ElfProgType},
    screen::{FrameBufferInfo, PixelFormat},
};

/// 2nd loader path in the boot device.
const SECOND_LOADER_PATH: &CStr16 = cstr16!("\\loader2");

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
        unsafe { core::arch::asm!("hlt") };
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

    // Get the 2nd loader file handle.
    let loader2_handle = root_dir
        .open(SECOND_LOADER_PATH, FileMode::Read, FileAttribute::empty())
        .map_err(|e| error!(e))?;
    let FileType::Regular(mut loader2) = loader2_handle.into_type().map_err(|e| error!(e))? else {
        println!("{} is a directory", SECOND_LOADER_PATH);
        return Err(error!(Error::new(Status::NOT_FOUND, ())));
    };

    // Allocate temporary buffer to load whole 2nd loader file
    // to deploy it into the propery address.
    let mut buf = [0; 1024];
    let file_info: &FileInfo = loader2
        .get_info(&mut buf)
        .map_err(|_| error!(Error::new(Status::BUFFER_TOO_SMALL, ())))?;
    let num_tmp_pages = (file_info.file_size() as usize + 4095) / 4096;
    let tmp_addr = st
        .boot_services()
        .allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            num_tmp_pages,
        )
        .map_err(|e| error!(e))?;
    let buf = slice::from_raw_parts_mut(tmp_addr as *mut _, num_tmp_pages * 4096);
    loader2.read(buf).map_err(|e| error!(e))?;

    // Get address info from ELF and programe headers.
    let elf_header = &*(buf.as_ptr() as *const Elf64Ehdr);
    let elf_phdrs = slice::from_raw_parts(
        (tmp_addr + elf_header.phoff) as *const Elf64Phdr,
        elf_header.phnum as _,
    );
    // Calculate the start and end addresses between which the 2nd loader will be loaded.
    let mut start = u64::MAX;
    let mut end = 0;
    for phdr in elf_phdrs {
        if phdr.ty == ElfProgType::Load {
            start = start.min(phdr.vaddr);
            end = end.max(phdr.vaddr + phdr.memsz);
        }
    }

    // Allocate memory for deploying the 2nd loader at its proper address.
    let num_pages = (end - start + 4095) / 4096;
    st.boot_services()
        .allocate_pages(
            AllocateType::Address(start),
            MemoryType::LOADER_CODE,
            num_pages as _,
        )
        .map_err(|e| error!(e))?;

    // Copy temporary data to right place.
    for phdr in elf_phdrs {
        if phdr.ty == ElfProgType::Load {
            ptr::copy_nonoverlapping(
                (tmp_addr + phdr.offset) as *const u8,
                phdr.vaddr as *mut u8,
                phdr.filesz as _,
            );
        }
    }

    println!("succeeded loading 2nd loader to {:08x}-{:08x}", start, end);

    let memmap = st
        .boot_services()
        .memory_map(MemoryType::LOADER_DATA)
        .map_err(|e| error!(e))?;

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

    // Exit UEFI boot service to pass the control to 2nd loader.
    let _ = st.exit_boot_services(MemoryType::LOADER_DATA);

    type EntryFn = extern "sysv64" fn(&FrameBufferInfo, &MemoryMap) -> !;
    let loader2_entry: EntryFn = transmute(elf_header.entry);
    loader2_entry(&fb_info, &memmap);
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
        unsafe { core::arch::asm!("hlt") };
    }
}
