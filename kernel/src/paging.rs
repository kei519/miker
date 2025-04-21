//! Provides some useful items to control paging.

use core::mem;

use util::paging::{ADDRESS_CONVERTER, AddressConverter, PAGE_SIZE, PageTable, VirtualAddress};
use util::{
    asmfunc,
    paging::PageEntry,
    sync::{InterruptFreeMutex, OnceStatic},
};

/// Base address to which kernel map whole physical address.
pub const STRAIGHT_PAGE_MAP_BASE: VirtualAddress = VirtualAddress::new(0xffff_8000_0000_0000);

/// Size of straight page map. kernel cannot control memory space whose address is over
/// [`STRAIGHT_PAGE_SIZE`], this is the max physical memory size of kernel.
pub const STRAIGHT_PAGE_SIZE: u64 = 1 << (12 + 9 * 3);

unsafe extern "C" {
    /// Placed at the start of the kernel.
    static _kernel_start: core::ffi::c_void;

    /// Placed at the end of the kernel.
    static _kernel_end: core::ffi::c_void;
}

/// Base virtual address of kernel.
pub static KERNEL_VIRT_BASE: OnceStatic<VirtualAddress> = OnceStatic::new();

/// End of the virtual address of the kernel.
pub static KERNEL_VIRT_END: OnceStatic<VirtualAddress> = OnceStatic::new();

/// Base physical address where kernel is deployed. [`KERNEL_VIRT_BASE`] is mapped to this.
pub static KERNEL_PHYS_BASE: OnceStatic<u64> = OnceStatic::new();

/// End of the physical address of the kernel.
pub static KERNEL_PHYS_END: OnceStatic<u64> = OnceStatic::new();

/// PML4 used by kernel.
pub static KERNEL_PML4: OnceStatic<InterruptFreeMutex<&'static mut PageTable>> = OnceStatic::new();

/// PDPT used for straight page map.
static STRAIGHT_PDPT: InterruptFreeMutex<PageTable> = InterruptFreeMutex::new(PageTable::new());

/// 512 PDs used for straight page map.
// Safety: PageTable all whose fields are zero is valid.
static STRAIGHT_PDS: InterruptFreeMutex<[PageTable; 512]> =
    InterruptFreeMutex::new(unsafe { mem::zeroed() });

/// PT used for straight page map from the end of the kernel to the next 2-MB aligned.
static STRAIGHT_PT_FOR_REMAINDERS: InterruptFreeMutex<PageTable> =
    InterruptFreeMutex::new(PageTable::new());

/// Initialize straight mapping of physical address `0` to virtual address
/// [`STRAIGHT_PAGE_MAP_BASE`] with size [`STRAIGHT_PAGE_SIZE`] for kernel. The physical memory
/// space where the kernel is located are excluded to avoid overwrite the content.
pub fn init_straight_mapping() {
    KERNEL_VIRT_BASE.init(VirtualAddress::new(&raw const _kernel_start as _));
    KERNEL_VIRT_END.init(VirtualAddress::new(&raw const _kernel_end as _));

    // Safety: CR3 is always valid as a pointer of PageTable because if not, processor does not
    //     work.
    KERNEL_PML4.init(InterruptFreeMutex::new(unsafe {
        &mut *(asmfunc::get_cr3() as *mut PageTable)
    }));
    let mut pml4 = KERNEL_PML4.as_ref().lock();
    #[rustfmt::skip]
    // Safety: Page map for kernel defenitely exists because if not, kernel does not properly work.
    let kernel_phys_base = unsafe {
        pml4[KERNEL_VIRT_BASE.pml4_index()] // PML4
            .next().unwrap()[KERNEL_VIRT_BASE.pdp_index()] // PDPT
            .next().unwrap()[KERNEL_VIRT_BASE.pd_index()] // PD
            .next().unwrap()[KERNEL_VIRT_BASE.pt_index()] // PT
            .next_addr()
    };
    KERNEL_PHYS_BASE.init(kernel_phys_base);
    KERNEL_PHYS_END.init(KERNEL_VIRT_END.addr - KERNEL_VIRT_BASE.addr + kernel_phys_base);

    let phys_end_2mb_base = KERNEL_PHYS_END.get() >> 21 << 21;
    let phys_end_page_aligned = KERNEL_PHYS_END.div_ceil(PAGE_SIZE as _) * (PAGE_SIZE as u64);
    let phys_space_is_2mb_aligned = phys_end_2mb_base == phys_end_page_aligned;

    // Initialize 512 x 512 x 2 MiB straight mapping (with skip of the space where the kernel is
    // located).
    for (i, (pdp_entry, pd)) in STRAIGHT_PDPT
        .lock()
        .iter_mut()
        .zip(STRAIGHT_PDS.lock().iter_mut())
        .enumerate()
    {
        for (j, pd_entry) in pd.iter_mut().enumerate() {
            let (i, j) = (i as u64, j as u64);
            #[allow(clippy::identity_op)]
            let addr = (i << (12 + 9 * 2)) + (j << (12 + 9 * 1));

            // Skip the kernel space.
            if (KERNEL_PHYS_BASE.get()..phys_end_2mb_base).contains(&addr) {
                continue;
            }

            if !phys_space_is_2mb_aligned && addr == phys_end_2mb_base {
                // There is remainder space where the kernel is not located and whose size is less
                // than 2 MB. We must map it as 4-KB straight mapping.
                let mut pt = STRAIGHT_PT_FOR_REMAINDERS.lock();
                for (k, pt_entry) in pt
                    .iter_mut()
                    .enumerate()
                    .skip(((phys_end_page_aligned - phys_end_2mb_base) as usize) / PAGE_SIZE)
                {
                    let addr = addr + (k << 12) as u64;
                    // Since these pages are for kernel, set as writable and supervisor pages.
                    // Safety: this `addr` is 4-KB aligned.
                    *pt_entry = unsafe { PageEntry::new(addr, true, false) };
                }
                // Safety: pt is properly aligned because it is a `PageTable`.
                // Also unwrapping succeeds because `pt` is existing kernel data.
                *pd_entry = unsafe {
                    PageEntry::new(
                        virt_to_phys(pt.as_ref().as_ptr() as u64).unwrap(),
                        true,
                        false,
                    )
                };
            } else {
                // Since these pages are for kernel, set as writable and supervisor pages.
                // Safety: this `addr` is 4-KB aligned.
                *pd_entry = unsafe { PageEntry::new(addr, true, false) };
                pd_entry.set_page_size(true);
            }
        }
        // Safety: `pd` is properly aligned because this is `PageTable`.
        // Also unwrapping succeeds because `pd` is existing kernel data.
        *pdp_entry =
            unsafe { PageEntry::new(virt_to_phys(pd as *const _ as u64).unwrap(), true, false) };
    }
    // Safety: `STRAIGHT_PDPT` is properly aligned because this is `PageTable`.
    // Also unwrapping succeeds because `STRAIGHT_PDPT` is existing kernel data.
    pml4[STRAIGHT_PAGE_MAP_BASE.pml4_index()] = unsafe {
        PageEntry::new(
            virt_to_phys(STRAIGHT_PDPT.lock().as_ref().as_ptr() as u64).unwrap(),
            true,
            false,
        )
    };

    ADDRESS_CONVERTER.init(AddressConverter::new(phys_to_virt2));
}

/// Converts physical address `addr` to virtual address if there is such mapping. Otherwise
/// returns `None`. This is only for kernel.
pub const fn pyhs_to_virt(addr: u64) -> Option<VirtualAddress> {
    if addr <= STRAIGHT_PAGE_SIZE {
        Some(VirtualAddress::new(addr + STRAIGHT_PAGE_MAP_BASE.addr))
    } else {
        None
    }
}

/// Converts virtual address `addr` to physical address if there is such mapping. Otherwise
/// returns `None`. This is only for kernel.
pub fn virt_to_phys(addr: impl Into<VirtualAddress>) -> Option<u64> {
    let addr: VirtualAddress = addr.into();
    let addr = addr.addr;
    if (STRAIGHT_PAGE_MAP_BASE.addr..STRAIGHT_PAGE_MAP_BASE.addr + STRAIGHT_PAGE_SIZE)
        .contains(&addr)
    {
        Some(addr - STRAIGHT_PAGE_MAP_BASE.addr)
    } else if (KERNEL_VIRT_BASE.addr..KERNEL_VIRT_END.addr).contains(&addr) {
        Some(addr - KERNEL_VIRT_BASE.addr + KERNEL_PHYS_BASE.get())
    } else {
        None
    }
}

/// Converts physical address `addr` to virtual address. Returns `0` if and only if there is no
/// such mapping.
fn phys_to_virt2(addr: u64) -> u64 {
    pyhs_to_virt(addr).map(|addr| addr.addr).unwrap_or(0)
}
