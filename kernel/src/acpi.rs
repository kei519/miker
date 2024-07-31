use alloc::format;
use uefi::table::{cfg::ACPI2_GUID, Runtime, SystemTable};
use util::error;
use util::{
    acpi::{DescriptionTable, Fadt, Rsdp},
    error::Result,
    sync::OnceStatic,
};

use crate::paging;

pub static FADT: OnceStatic<&'static Fadt> = OnceStatic::new();

/// ACPI MMIO base virtual address.
pub static MMIO_BASE: OnceStatic<u64> = OnceStatic::new();

/// Set [`FADT`] and [`MMIO_BASE`].
pub fn init(runtime: SystemTable<Runtime>) -> Result<()> {
    let rsdp = 'search: {
        for config in runtime.config_table() {
            if config.guid == ACPI2_GUID {
                // Addresses in ACPI2 are referenced as physical. See
                // https://uefi.org/specs/UEFI/2.10/04_EFI_System_Table.html#industry-standard-configuration-tables
                let addr = paging::pyhs_to_virt(config.address as _).unwrap();
                // Safety: `runtime` and `phys_to_virt()` are proper.
                break 'search match unsafe { Rsdp::from_ptr(addr.addr as _) } {
                    Ok(rsdp) => rsdp,
                    Err(e) => error!(format!("{:?}", e)),
                };
            }
        }
        panic!("not found RSDP!");
    };

    let mut fadt = None;
    let mut mcfg = None;
    for entry in rsdp.xsdt().unwrap().entries() {
        match entry {
            DescriptionTable::Fadt(entry) => fadt = Some(entry),
            DescriptionTable::Mcfg(entry) => mcfg = Some(entry),
            _ => {}
        }
    }
    if fadt.is_none() {
        error!("not found FADT");
    }
    FADT.init(fadt.unwrap());

    if mcfg.is_none() {
        error!("not found MCFG");
    }
    let mcfg = mcfg.unwrap();
    if mcfg.entries_count() == 0 {
        error!("There is no MCFG configs");
    }
    MMIO_BASE.init(
        paging::pyhs_to_virt(mcfg.configs()[0].base_addr)
            .unwrap()
            .addr,
    );

    Ok(())
}
