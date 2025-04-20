//! Provides some useful items to control paging.

use core::mem;

use util::paging::{ADDRESS_CONVERTER, AddressConverter, PageTable, VirtualAddress};
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

/// Base virtual address of kernel.
pub const KERNEL_VIRT_BASE: VirtualAddress = VirtualAddress::new(0xffff_ffff_8000_0000);

/// Base physical address where kernel is deployed. [`KERNEL_VIRT_BASE`] is mapped to this.
pub static KERNEL_PHYS_BASE: OnceStatic<u64> = OnceStatic::new();

/// PML4 used by kernel.
pub static KERNEL_PML4: OnceStatic<InterruptFreeMutex<&'static mut PageTable>> = OnceStatic::new();

/// PDPT used for straight page map.
static STRAIGHT_PDPT: InterruptFreeMutex<PageTable> = InterruptFreeMutex::new(PageTable::new());

/// 512 PDs used for straight page map.
// Safety: PageTable all whose fields are zero is valid.
static STRAIGHT_PDS: InterruptFreeMutex<[PageTable; 512]> =
    InterruptFreeMutex::new(unsafe { mem::zeroed() });

/// Initialize straight mapping of physical address `0` to virtual address
/// [`STRAIGHT_PAGE_MAP_BASE`] with size [`STRAIGHT_PAGE_SIZE`] for kernel.
pub fn init_straight_mapping() {
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

    // Initialize 512 x 512 x 2 MiB straight mapping.
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
            // Since these pages are for kernel, set as writable and supervisor pages.
            // Safety: this `addr` is 4-KB aligned.
            *pd_entry = unsafe { PageEntry::new(addr, true, false) };
            pd_entry.set_page_size(true);
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
    } else if (KERNEL_VIRT_BASE.addr..=u64::MAX).contains(&addr) {
        // FIXME: This returns Some() even thought there is no mapping. We should get the end of
        //        kernel address to avoid it.
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
