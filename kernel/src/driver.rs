//! Provides drivers for the kernel.

use util::driver::AhciConfig;
use util::error;
use util::pci::Capability;
use util::{error::Result, pci::ConfigSpaces, sync::OnceStatic};

use crate::acpi::MMIO_PHYS_BASE;
use crate::printkln;

pub static CONFIG_SPACES: OnceStatic<ConfigSpaces> = OnceStatic::new();

/// Initializes drivers.
///
/// # Panics
///
/// It will cause panic if called before [MMIO_PHYS_BASE] is initialized.
pub fn init() -> Result<()> {
    // Safety: MMIO_PHSY_BASE is passed by UEFI, so it must meet the condition.
    CONFIG_SPACES.init(unsafe { ConfigSpaces::from_ptr(*MMIO_PHYS_BASE as _) });

    let Some(ahci_bfd) = CONFIG_SPACES
        .valid_bfds_and_classes()
        .find(|(class, _)| {
            class.base_class == 0x01 && class.sub_class == 0x06 && class.interface == 0x01
        })
        .map(|(_, bfd)| bfd)
    else {
        error!("There is no SATA AHCI devices.")
    };

    printkln!("AHCI's capabilities");
    let mut config = CONFIG_SPACES.get_config_space(ahci_bfd).unwrap();
    printkln!("{:x?}", *config);
    for cap in config.raw_capabilities().map(Capability::from) {
        printkln!("{:x?}", cap);
    }

    let mut ahci = AhciConfig::new(&mut config);
    let hba_reg = ahci.registers();
    printkln!("{:x?}", hba_reg);
    Ok(())
}
