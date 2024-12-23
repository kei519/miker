//! Library for drivers.

use core::fmt;

use custom_debug::Debug;
use modular_bitfield::{bitfield, prelude::*};

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
    pub cap: HbaCap,
    /// Global Host Control.
    pub ghc: GlobalHbaControl,
    /// Interrupt Status.
    pub is: u32,
    /// Ports Implemented.
    pub pi: u32,
    /// Version.
    pub vs: u32,
    /// Command Completion Coalescing Control
    pub ccc_ctl: CccCtl,
    /// Command Completion Coalsecing Ports.
    pub ccc_ports: u32,
    /// Enclosure Management Location.
    pub em_loc: EmLoc,
    /// Enclosure Management Control.
    pub em_ctl: u32,
    /// Host Capailities Extended.
    pub cap2: u32,
    /// BIOS/OS Handoff Control and Status.
    pub bohc: u32,
}

/// Indicates basic capabilities of the HBA to driver software.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct HbaCap {
    /// Number of Ports (NP).
    ///
    /// 0's based value indicating the maximum number of ports supported by the HBA silicon. A
    /// muximum of 32 ports can be supported.
    #[skip(setters)]
    pub np: B5,

    /// Supports External SATA (SXS).
    ///
    /// When set, indicates that the HBA has one or more Serial ATA ports that has a signal only
    /// connector that is externally accessible (e.g. eSATA connector).
    #[skip(setters)]
    pub sxs: bool,

    /// Enclosure Management Supported (EMS).
    ///
    /// When set, indicates that the HBA supports enclosure management (defined in section 12).
    #[skip(setters)]
    pub ems: bool,

    /// Command Completion Coalescing Supported (CCCS).
    ///
    /// When set, indicates that the HBA supports command completion coalescing (defined in
    /// section 11).
    #[skip(setters)]
    pub cccs: bool,

    /// Number of Command Slots (NCS).
    ///
    /// 0's based value indicating the number of command slots per port supported by this HBA. A
    /// minimum of 1 and maximum of 32 slots per port can be supported.
    #[skip(setters)]
    pub ncs: B5,

    /// Partial State Capable (PSC).
    ///
    /// Indicates whether the HBA can support transition to the Partial state.
    #[skip(setters)]
    pub psc: bool,

    /// Slumber State Capable (SSC).
    ///
    /// Indicates whether the HBA can support transitions to the Slumber state.
    #[skip(setters)]
    pub ssc: bool,

    /// PIO Multiple DRQ Block (PMD).
    ///
    /// If set, the HBA supports multiple DRQ block data transfers for the PIO command protocol.
    #[skip(setters)]
    pub pmd: bool,

    /// FIS-based Switching Supported (FBSS).
    ///
    /// When set, indicates that the HBA supports Port Multiplier FIS-based switching.
    #[skip(setters)]
    pub fbss: bool,

    /// Supports Port Multiplier (SPM).
    ///
    /// Indicates whether the HBA can support a Port Multiplier.
    #[skip(setters)]
    pub smp: bool,

    /// Supports AHCI mode only (SAM).
    ///
    /// The SATA controller may optionally support AHCI access mechanism only. If set, indicates
    /// that the SATA controller does not implement a legacy, task-file based register interface.
    #[skip(setters)]
    pub sam: bool,

    #[skip]
    __: B1,

    /// Interface Speed Support (ISS).
    ///
    /// Inicate the maximum speed the HBA can support on its ports.
    ///
    /// | Bits | Definition |
    /// | --- | --- |
    /// | 0000 | Reserved |
    /// | 0001 | Gen 1 (1.5 Gbps) |
    /// | 0010 | Gen 2 (3 Gbps) |
    /// | 0011 | Gen 3 (6 Gbps) |
    /// | 0100 - 1111 | Reserved |
    #[skip(setters)]
    pub iss: B4,

    /// Supports Command List Override (SCLO).
    ///
    /// When set, the HBA supports the PxCMD.CLO bit and its associated function.
    #[skip(setters)]
    pub scld: bool,

    /// Supports Activity LED (SAL).
    ///
    /// When set, the HBA supports a single activity indication output pin.
    pub sal: bool,

    /// Supports Aggressive Link Power Management (SALP).
    ///
    /// When set, the HBA can support auto-generating link requests to the Partial or Slumber
    /// states when there are no commands to process.
    #[skip(setters)]
    pub salp: bool,

    /// Supports Staggered Sping-up (SSS).
    ///
    /// When set, the HBA supports staggered spin-up on its ports, fro use in balancing power
    /// spikes.
    #[skip(setters)]
    pub sss: bool,

    /// Supports Mechanical Presence Switch (SMPS).
    ///
    /// When set, the HBA supports mechanical presence switches on its ports for use in hot plug
    /// operations.
    #[skip(setters)]
    pub smps: bool,

    /// Supports SNotification Register.
    ///
    /// When set, the HBA supports the PxSNTF (SNotification) register and its assocaiated
    /// functionality.
    #[skip(setters)]
    pub ssntf: bool,

    /// Supports Native Command Queuing (SNCQ).
    ///
    /// Indictes whether the HBA supports Serial ATA native command queuing.
    #[skip(setters)]
    pub sncq: bool,

    /// Supports 64-bit Addressing (S64A).
    ///
    /// Indicates whether the HBA can access 64-bit data structures.
    #[skip(setters)]
    pub s64a: bool,
}

/// Controls various global actions of the HBA.
#[bitfield]
#[derive(Debug, Default)]
pub struct GlobalHbaControl {
    pub reset: bool,
    pub ie: bool,
    #[skip(setters)]
    pub msrm: bool,
    #[skip]
    __: B28,
    pub ae: bool,
}

/// Used to configure the command completion coalescing feature for the entire HBA.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct CccCtl {
    /// Enable (EN).
    ///
    /// When cleared, the command completion coalescing feature is disabled and no CCC interrupts
    /// are generated.
    pub en: bool,

    #[skip]
    __: B2,

    /// Interrupt (INT).
    ///
    /// Specifies the interrupt used by the CCC feature. This interrupt must be marked as unused in
    /// the Ports Implemented (PI) register by the corresponding bit being set to `0`.
    #[skip(setters)]
    pub int: B5,

    /// Command Completions (CC)>
    ///
    /// Specifies the number of command completions that are necessary to cause a CCC interrupt.
    pub cc: u8,

    /// Timeout Value (TV).
    ///
    /// The timeout value is specified in 1 millisecond intervals.
    pub tv: u16,
}

/// The enclosure management location register identifies the location and size of the enclosure
/// management message buffer.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct EmLoc {
    /// Buffer Size (SZ).
    ///
    /// Specifies the size of the transmit message buffer area in Dwords.
    #[skip(setters)]
    pub sz: u16,

    /// Offset (OFST).
    ///
    /// The offset of the message buffer in Dwords from the beginning of the ABAR.
    #[skip(setters)]
    pub ofst: u16,
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
