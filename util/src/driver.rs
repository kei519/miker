//! Library for drivers.

use bitfield::bitfield;

use crate::{bitfield::BitField as _, paging::ADDRESS_CONVERTER, pci::ConfigSpace};

/// a
pub struct Ahci<'a>(&'a mut ConfigSpace);

impl Ahci<'_> {
    /// AHCI Base Address.
    pub fn abar(&self) -> u64 {
        let addr = self.0.bars[5] as u64;
        assert!(addr.get_bits(0..1) == 0);
        assert!(addr.get_bits(1..3) == 0);
        assert!(addr.get_bits(3..4) == 0);
        ADDRESS_CONVERTER.get_addr(addr).unwrap()
    }
}

impl<'a> Ahci<'a> {
    /// Constructs.
    pub fn new(config: &'a mut ConfigSpace) -> Self {
        Self(config)
    }
}

/// HBA Memory Registers.
#[derive(Debug)]
#[repr(C)]
pub struct HbaMemoryRegister {
    /// Generic Host Control.
    pub generic_host_control: GenericHostControl,
    _reserved: [u8; 0x34],
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
    pub ghc: GlobaHbaControl,
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
    pub em_loc: u32,
    /// Enclosure Management Control.
    pub em_ctl: u32,
    /// Host Capailities Extended.
    pub cap2: u32,
    /// BIOS/OS Handoff Control and Status.
    pub bohc: u32,
}

bitfield! {
    /// Indicates basic capabilities of the HBA to driver software.
    pub struct HbaCap(u32);
    impl Debug;

    // Default return type.
    u8;

    /// Number of Ports (NP).
    ///
    /// 0's based value indicating the maximum number of ports supported by the HBA silicon. A
    /// muximum of 32 ports can be supported.
    pub np, _: 4, 0;

    /// Supports External SATA (SXS).
    ///
    /// When set, indicates that the HBA has one or more Serial ATA ports that has a signal only
    /// connector that is externally accessible (e.g. eSATA connector).
    pub sxs, _: 5;

    /// Enclosure Management Supported (EMS).
    ///
    /// When set, indicates that the HBA supports enclosure management (defined in section 12).
    pub ems, _: 6;

    /// Command Completion Coalescing Supported (CCCS).
    ///
    /// When set, indicates that the HBA supports command completion coalescing (defined in
    /// section 11).
    pub cccs, _: 7;

    /// Number of Command Slots (NCS).
    ///
    /// 0's based value indicating the number of command slots per port supported by this HBA. A
    /// minimum of 1 and maximum of 32 slots per port can be supported.
    pub ncs, _: 12, 8;

    /// Partial State Capable (PSC).
    ///
    /// Indicates whether the HBA can support transition to the Partial state.
    pub psc, _: 13;

    /// Slumber State Capable (SSC).
    ///
    /// Indicates whether the HBA can support transitions to the Slumber state.
    pub ssc, _: 14;

    /// PIO Multiple DRQ Block (PMD).
    ///
    /// If set, the HBA supports multiple DRQ block data transfers for the PIO command protocol.
    pub pmd, _: 15;

    /// FIS-based Switching Supported (FBSS).
    ///
    /// When set, indicates that the HBA supports Port Multiplier FIS-based switching.
    pub fbss, _: 16;

    /// Supports Port Multiplier (SPM).
    ///
    /// Indicates whether the HBA can support a Port Multiplier.
    pub smp, _: 17;

    /// Supports AHCI mode only (SAM).
    ///
    /// The SATA controller may optionally support AHCI access mechanism only. If set, indicates
    /// that the SATA controller does not implement a legacy, task-file based register interface.
    pub sam, _: 18;

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
    pub iss, _: 23, 20;

    /// Supports Command List Override (SCLO).
    ///
    /// When set, the HBA supports the PxCMD.CLO bit and its associated function.
    pub scld, _: 24;

    /// Supports Activity LED (SAL).
    ///
    /// When set, the HBA supports a single activity indication output pin.
    pub sal, _: 25;

    /// Supports Aggressive Link Power Management (SALP).
    ///
    /// When set, the HBA can support auto-generating link requests to the Partial or Slumber
    /// states when there are no commands to process.
    pub salp, _: 26;

    /// Supports Staggered Sping-up (SSS).
    ///
    /// When set, the HBA supports staggered spin-up on its ports, fro use in balancing power
    /// spikes.
    pub sss, _ : 27;

    /// Supports Mechanical Presence Switch (SMPS).
    ///
    /// When set, the HBA supports mechanical presence switches on its ports for use in hot plug
    /// operations.
    pub smps, _: 28;

    /// Supports SNotification Register.
    ///
    /// When set, the HBA supports the PxSNTF (SNotification) register and its assocaiated
    /// functionality.
    pub ssntf, _: 29;

    /// Supports Native Command Queuing (SNCQ).
    ///
    /// Indictes whether the HBA supports Serial ATA native command queuing.
    pub sncq, _: 30;

    /// Supports 64-bit Addressing (S64A).
    ///
    /// Indicates whether the HBA can access 64-bit data structures.
    pub s64a, _: 31;
}

bitfield! {
    /// Controls various global actions of the HBA.
    pub struct GlobaHbaControl(u32);
    impl Debug;

    /// HBA Reset (HR).
    ///
    /// When set by SW (?), thisbit causes an internal reset of the HBA.
    pub _, reset: 0;

    /// Interrupt Enable (IE).
    ///
    /// This global bit enables interrupts from the HBA.
    pub ie, set_ie: 1;

    /// MSI Revert to Single Message (MRSM).
    ///
    /// When set by hardware, indicates that the HBA requested more than one MSI vector but has
    /// reverted to using the first vector only.
    pub msrm, _: 2;

    /// AHCI Enable (AE).
    ///
    /// When set, indicates that communication to the HBA shall be via AHCI mechanisms.
    pub ae, set_ae: 31;
}

bitfield! {
    /// Used to configure the command completion coalescing feature for the entire HBA.
    pub struct CccCtl(u32);
    impl Debug;

    // Default return type.
    u8;

    /// Enable (EN).
    ///
    /// When cleared, the command completion coalescing feature is disabled and no CCC interrupts
    /// are generated.
    pub en, set_en: 0;

    /// Interrupt (INT).
    ///
    /// Specifies the interrupt used by the CCC feature. This interrupt must be marked as unused in
    /// the Ports Implemented (PI) register by the corresponding bit being set to `0`.
    pub int, _: 7, 3;

    /// Command Completions (CC)>
    ///
    /// Specifies the number of command completions that are necessary to cause a CCC interrupt.
    pub cc, set_cc: 15, 8;

    /// Timeout Value (TV).
    ///
    /// The timeout value is specified in 1 millisecond intervals.
    pub u16, tv, set_tv: 16, 31;
}

/// Vendor specific registers at offset 0xA0 to 0xFF for a HBA register.
#[derive(Debug)]
#[repr(C)]
pub struct VendorSpecificRegisters {
    /// Contents of the registers.
    pub data: [u8; 0x60],
}

/// Registers necessary to implement per port.
#[derive(Debug)]
#[repr(C)]
pub struct PortRegister {
    /// Command List Base Address.
    pub clb: u32,
    /// Command List Base Address Upper 32-Bits.
    pub clbu: u32,
    /// FIS Base Address.
    pub fb: u32,
    /// FIS Base Address Upper 32-Bits.
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
