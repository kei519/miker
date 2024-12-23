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
    pub em_ctl: EmCtl,
    /// Host Capailities Extended.
    pub cap2: u32,
    /// BIOS/OS Handoff Control and Status.
    pub bohc: Bohc,
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

/// This register is used to control and obtain status for the enclosure management interface.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct EmCtl {
    /// Message Received (STS.MR).
    ///
    /// The HBA sets this bit when a message is completely received into the message
    /// buffer.
    pub sts_mr: bool,

    #[skip]
    __: B7,

    /// Transmit Message (CTL.TM).
    ///
    /// When set by software, the HBA shall transmit the message contained in the message buffer.
    pub ctl_tm: bool,

    /// Reset (CTL.RST).
    ///
    /// When set by software, the HBA shall reset all enclosure management message logic and the
    /// attached enclosure processor (if applicable) and take all appropriate reset actions to
    /// ensure messages can be transmitted/recevived after the reset.
    pub ctl_rst: bool,

    #[skip]
    __: B6,

    /// LED Message Types (SUPP.LED).
    ///
    /// If set, the HBA supports the LED message type defined in section 12.2.1.
    #[skip(setters)]
    pub supp_led: bool,

    /// SAF-TE Enclosure Management Messages (SUPP.SAFTE).
    ///
    /// If set, the HBA supports the SAF-TE message type.
    #[skip(setters)]
    pub supp_safte: bool,

    /// SAF-TE Enclosure Management Messages (SUPP.SAFTE2).
    ///
    /// If set, the HBA supports the SAF-2 message type.
    #[skip(setters)]
    pub supp_safte2: bool,

    /// SGPIO Enclosure Management Messages (SUPP.SGPIO).
    ///
    /// If set, the HBA supports the SGPIO register interface message type.
    #[skip(setters)]
    pub supp_sgpio: bool,

    #[skip]
    __: B4,

    /// Single Message Buffer (ATTR.SMB).
    ///
    /// If set, the HBA has one message buffer that is shared for messages to transmit and messages
    /// received.
    #[skip(setters)]
    pub attr_smb: bool,

    /// Transmit Only (ATTR.XMT).
    ///
    /// If set, the HBA only supports transmitting messages and does not support receiving
    /// messages.
    #[skip(setters)]
    pub attr_xmt: bool,

    /// Activity LED Hardware Driven (ATTR.ALHD).
    ///
    /// If set, the HBA drives the activity LED for the LED message type in hardware and does not
    /// utilize software settings for this LED.
    #[skip(setters)]
    pub attr_alhd: bool,

    /// Port Multiplier Supprt (ATTR.PM).
    ///
    /// If set, the HBA supports enclosure management messages for device management messages for
    /// devices attached via a Port Multiplier.
    #[skip(setters)]
    pub attr_pm: bool,

    #[skip]
    __: B4,
}

/// This register controls various global actions of the HBA.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct Bohc {
    /// BIOS Owned Semaphore (BOS).
    ///
    /// The BIOS sets this bit to establish ownership of the HBA controller.
    pub bos: bool,

    /// OS Owned Semaphore (OOS).
    ///
    /// THe system software sets this bit to request ownership of the HBA controller.
    pub oos: bool,

    /// SMI on OS Ownership Change Enable (SOOE).
    ///
    /// This bit, when set, enables an SMI when the OOC bit has been set.
    pub sooe: bool,

    /// OS Ownership Change (OCC).
    ///
    /// This bit is set when the OOS bit transitions from `0` to `1`.
    pub ooc: bool,

    /// BIOS Busy (BB).
    ///
    /// This bit is used by the BIOS to indicate that it is busy clearing up for ownership change.
    pub bb: bool,

    #[skip]
    __: B27,
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
    pub is: Is,
    /// Interrupt Enable.
    pub ie: Ie,
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

/// Interrupt Status (IS).
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct Is {
    /// Device to Host Register FIS Interrupt (DHRS).
    ///
    /// A D2H Register FIS has been received with the 'I' bit set, and has been copied into system
    /// memory.
    pub dhrs: bool,

    /// PIO Setup FIS Interrupt (PSS).
    ///
    /// A PIO Setup FIS has been received with the 'i' bit set, it has been copied into system
    /// memory, and the data related to that FIS has been transferred.
    pub pss: bool,

    /// DMA Setup FIS Interrupt (DSS).
    ///
    /// A DMA Setup FIS has been received with the 'i' bit set and has been copied into system
    /// memory.
    pub dss: bool,

    /// Set Device Bits Interrupt (SDBS).
    ///
    /// A Set Device Bits FIS has been received with the 'i' bit set and has been copied into
    /// system memory.
    pub sdbs: bool,

    /// Unknown FIS Interrupt (UFS).
    ///
    /// When set, indicates that an unknwon FIS was received and has been copied into system
    /// memory.
    /// 1=Change in Current Connect Status.
    #[skip(setters)]
    pub ufs: bool,

    /// Descriptor Processed (DPS).
    ///
    /// A PRD with the 'i' bit set has transferred all of its data.
    pub dps: bool,

    /// Port Connect Change Status (PCS).
    ///
    /// 1=Change in Current Connect Status. 0=No change in Current Connecct Status.
    #[skip(setters)]
    pub pcs: bool,

    /// Device Mechaanical Presence Status (DMPS).
    ///
    /// When set, indicates that a mechanical presence switch associated with this port has been
    /// opened or closed, which may lead to a change in the connection state of the device.
    pub dmps: bool,

    #[skip]
    __: B14,

    /// PhyRdy Change Status (PRCS).
    ///
    /// WHen set indicates the internal PhyRdy signal changed state.
    #[skip(setters)]
    pub prcs: bool,

    /// Incorrect Port Multiplier Status (IPMS).
    ///
    /// Indicates that the HBA received a FIS from a device that did not have a command
    /// outstanding.
    pub ipms: bool,

    /// Overflow Status (OFS).
    ///
    /// Indicates that hte HBA received more bytes from a device than was specified in the PRD
    /// table for the command.
    pub ofs: bool,

    #[skip]
    __: B1,

    /// Interface Non-fatal Error Status (INFS).
    ///
    /// Indicates that the HBA encountered an error on the Serial ATA interface but was able to
    /// continue operation.
    pub infs: bool,

    /// Interface Fatal Error Status (IFS).
    ///
    /// Indicates that the HBA encountered an error on the Serial ATA interface but was able to
    /// continue operation.
    pub ifs: bool,

    /// Host Bus Data Error Status (HBDS).
    ///
    /// Indicates that the HBA encountered a data error (uncorerctable ECC / parity) when reading
    /// or writeing to system memory.
    pub hbds: bool,

    /// Host Bus Fatal Error Status (HBFS)>
    ///
    /// Indicates that the HBA encountered a host bus error that it cannont recover from, such as a
    /// bad software pointer.
    pub hbfs: bool,

    /// Task File Error Status (TFES).
    ///
    /// This bit is set whenever the status register is updated by the device and the error bit
    /// (bit 0 of the Status field in the received FIS) is set.
    pub tfes: bool,

    /// Cold Port Detect Status (CPDS).
    ///
    /// When set, a device status has changed as detected by the cold presence detect logic.
    pub cpds: bool,
}

/// This register enables and disables the reporting of the corresponding interrupt to system
/// software.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct Ie {
    /// Device to Host Register FIS Interrupt Enable (DHRE).
    ///
    /// When set, GHC.IS is set, and IS.DHRS is set, the HBA shall generate an interrupt.
    pub dhre: bool,

    /// PIO Setup FIS Interrupt Enable (PSE).
    ///
    /// When set, GHC.IE is set, and IS.PSS is set, the HBA shall generate an interrupt.
    pub pse: bool,

    /// DMA Setup FIS Interrupt Enable (DSE).
    /// When set, GHC.IE is set, and IS.PSS is set, the HBA shall generate an interrupt.
    pub dse: bool,

    /// Set Device Bits FIS Interrupt Enable (SDBE).
    ///
    /// When set, GHC.IS is set, and IS.SDBS is set, the HBA shall generate an interrupt.
    pub sdbe: bool,

    /// Unknown FIS Interrupt Enalbe (UFE).
    ///
    /// When set, GHC.IS is set, and IS.UFS is set, the HBA shall generate an interrupt.
    pub ufe: bool,

    /// Descriptor Processed Interrupt Enalbe (DPE).
    ///
    /// When set, GHC.IE is set, and IS.UFS is set, the HBA shall generate an interrupt.
    pub dpe: bool,

    /// Port Change Interrupt Enable (PCE).
    ///
    /// When set, GHC.IS is set, and IS.DPS is set, the HBA shall generate an interrupt.
    pub pce: bool,

    /// Device Mechanical Presence Enable (DMPE).
    ///
    /// When set, and GHC.IS is set, and IS.DMPS is set, the HBA shall generate an interrupt.
    //
    // TODO: This bit shall be a read-only `0` for systems that do not support a mechanical
    //       presence switch.
    pub dmpe: bool,

    #[skip]
    __: B14,

    /// PhyRdy Change Interrupt Enable (PRCE).
    ///
    /// When set, and GHC.IE is set, and IS.PRCS is set, the HBA shall generate an interrupt.
    pub prce: bool,

    /// Incorrect Port Multiplier Enable (IPME).
    ///
    /// When set, and GHC.IE and IS.IPMS are set, the HBA shall generate an interrupt.
    pub impe: bool,

    /// Overflow Enable (OFE).
    ///
    /// When set, and GHC.IS and IS.OFS are set, the HBA shall generate an interrupt.
    pub ofe: bool,

    #[skip]
    __: bool,

    /// Interface Non-fata Error Enable (INFE).
    ///
    /// When set, GHC.IS is set and IS.INFS is set, the HBA shall generate an interrupt.
    pub infe: bool,

    /// Interface Fatal Error Enable (IFE).
    ///
    /// When Set, GHC.IS is set, and IS.IFS is set, the HBA shall generate an interrupt.
    pub ife: bool,

    /// Host Bus Data Error Enable (HBDE).
    ///
    /// When set, GHC.IS is set, and IS.HBDS is set, the HBA shall generate an interrupt.
    pub hbde: bool,

    /// Host Bus Fatal Error Enable (HBFE).
    ///
    /// When set, GHC.IE is set, and IS.HBFS is set, the HBA shall generate an interrupt.
    pub hbfe: bool,

    /// Task File Error Enable (TFEE).
    ///
    /// When set, GHC.IE is set, IS.TFES is set, the HBA shall generate an interrupt.
    pub tfee: bool,

    /// Cold Presence Detect Enable (CPDE).
    ///
    /// When set, GHC.IE is set, and IS.CPDS is set, the HBA shall generate an interrupt.
    //
    // TODO: This bit shall be a read-only `0` for systems that do not support cold presence
    //       detect.
    pub cpde: bool,
}
