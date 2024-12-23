//! Library for drivers.

use core::fmt;

use custom_debug::Debug;

use crate::{bitfield::BitField as _, paging::ADDRESS_CONVERTER, pci::ConfigSpace};

/// Represents a configuration space for a AHCI device.
pub struct AhciConfig<'a>(&'a mut ConfigSpace);

impl AhciConfig<'_> {
    fn abar(&self) -> u64 {
        let addr = self.0.bars[5] as u64;
        assert!(addr.get_bits(0..1) == 0);
        assert!(addr.get_bits(1..3) == 0);
        assert!(addr.get_bits(3..4) == 0);
        ADDRESS_CONVERTER.get_addr(addr).unwrap()
    }

    /// Provides exclusive access to the HBA memory registers.
    pub fn registers(&mut self) -> &mut HbaMemoryRegisters {
        unsafe {
            (self.abar() as *mut u64)
                .cast::<HbaMemoryRegisters>()
                .as_mut()
                .unwrap()
        }
    }
}

impl<'a> AhciConfig<'a> {
    /// Constructs.
    pub fn new(config: &'a mut ConfigSpace) -> Self {
        Self(config)
    }
}

/// HBA Memory Registers.
#[derive(Debug)]
#[repr(C)]
pub struct HbaMemoryRegisters {
    /// Generic Host Control.
    pub generic_host_control: GenericHostControl,
    #[debug(skip)]
    _reserved: [u8; 0x34],
    #[debug(skip)]
    _reserved_for_nvmhci: [u8; 0x40],
    /// Vendor Specific registers.
    pub vendor_specific: VendorSpecificRegisters,
    /// Port Control Register for port 0-31.
    pub ports_registers: [PortRegister; 32],
}

/// Generic Host Control.
#[derive(Debug)]
#[repr(C)]
pub struct GenericHostControl {
    /// Host Capabilities.
    pub cap: u32,
    /// Global Host Control.
    pub ghc: u32,
    /// Interrupt Status.
    pub is: u32,
    /// Ports Implemented.
    pub pi: u32,
    /// Version.
    pub vs: u32,
    /// Command Completion Coalescing Control
    pub ccc_ctl: u32,
    /// Command Completion Coalsecing Ports.
    pub ccc_ports: u32,
    /// Enclosure Management Location.
    pub em_loc: u32,
    /// Enclosure Management Control.
    pub em_ctl: u32,
    /// Host Capailities Extended.
    pub cap2: u32,
    /// BIOS/OS Handoff Control and Status.
    pub bohc: u32,
}

/// Vendor specific registers at offset 0xA0 to 0xFF for a HBA register.
#[derive(Debug)]
#[repr(C)]
pub struct VendorSpecificRegisters {
    /// Contents of the registers.
    pub data: [u8; 0x60],
}

/// Registers necessary to implement per port.
#[repr(C)]
pub struct PortRegister {
    /// Command List Base Address.
    clb: u32,
    /// Command List Base Address Upper 32-Bits.
    clbu: u32,
    /// FIS Base Address.
    fb: u32,
    /// FIS Base Address Upper 32-Bits.
    fbu: u32,
    /// Interrupt Status.
    pub is: u32,
    /// Interrupt Enable.
    pub ie: u32,
    /// Command and Status.
    pub cmd: u32,
    _reserved0: u32,
    /// Task File Data.
    pub tfd: u32,
    /// Signature.
    pub sig: u32,
    /// Serial ATA Status (SCR0: SStatus).
    pub ssts: u32,
    /// Serial ATA Control (SCR2: SControl).
    pub sctl: u32,
    /// Serial ATA Error (SCR1: SError).
    pub serr: u32,
    /// Serial ATA Active. (SCR3: SActive).
    pub sact: u32,
    /// Command Issue.
    pub ci: u32,
    /// Serial ATA Notification (SCR4: SNotification).
    pub sntf: u32,
    /// FIS-based Switching Control.
    pub fbs: u32,
    /// Device Sleep.
    pub devslp: u32,
    _reserved1: [u8; 0x28],
    /// Vendor Specific.
    pub vs: u128,
}

impl PortRegister {
    /// Command List Base Address (CLB).
    ///
    /// Returns the physical address for the command list base for a port.
    pub fn clb(&self) -> u64 {
        (self.clbu as u64) << 32 | self.clb as u64
    }

    /// Sets the physical address for the command list base (CLB) for a port.
    ///
    /// # Remarks
    ///
    /// The lower 10 bits of `addr` will be ignored because the address must be 1K-byte aligned.
    //
    // TODO: We must set `0` to `clbu` for HBAs that do not support 64-bit addressing.
    pub fn set_clb(&mut self, addr: u64) {
        self.clbu = addr.get_bits(32..) as _;
        self.clb = (addr.get_bits(10..32) as u32) << 10;
    }

    /// FIS Base Address (FB).
    ///
    /// Returns the physical address for received FISes for a port.
    pub fn fb(&self) -> u64 {
        (self.fbu as u64) << 32 | self.fb as u64
    }

    /// Set the physical address for received FISes (FB) for a port.
    ///
    /// # Remarks
    ///
    /// The lower 8 bits of `addr` will be ignored because FB must be 256-byte aligned.
    //
    // TODO: We must set `0` to `clbu` for HBAs that do not support 64-bit addressing.
    // TODO: We must ensure `addr` is 4K-byte aligned when FIS-based switching is in use.
    pub fn set_fb(&mut self, addr: u64) {
        self.fbu = addr.get_bits(32..) as _;
        self.fb = (addr.get_bits(8..32) as u32) << 8
    }
}

impl fmt::Debug for PortRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PortRegister")
            .field("clb", &self.clb())
            .field("fb", &self.fb())
            .field("is", &self.is)
            .field("ie", &self.ie)
            .field("cmd", &self.cmd)
            .field("tfd", &self.tfd)
            .field("sig", &self.sig)
            .field("ssts", &self.ssts)
            .field("sctl", &self.sctl)
            .field("serr", &self.serr)
            .field("sact", &self.sact)
            .field("ci", &self.ci)
            .field("sntf", &self.sntf)
            .field("fbs", &self.fbs)
            .field("devslp", &self.devslp)
            .field("vs", &self.vs)
            .finish()
    }
}
