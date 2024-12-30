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
#[bitfield(bits = 32)]
#[derive(Debug, Default)]
pub struct GlobalHbaControl {
    /// HBA Reset (HR).
    ///
    /// When set by SW, this bit causes an internal reset of the HBA.
    pub hr: bool,

    /// Interrupt Enable (IE).
    ///
    /// This global bit enables interrupts from the HBA. When cleared (reset default), all
    /// interrupt sources from all prots are disabled. When set, interrupts are enabled.
    pub ie: bool,

    #[skip(setters)]
    /// MSI Revert to Single Message (MRSM).
    ///
    /// When set by hardware, indicates that the HBA requested more than one MSI vector but has
    /// reverted to using the first vector only.
    pub msrm: bool,

    #[skip]
    __: B28,

    /// AHCI Enable (AE).
    ///
    /// When set, indicates that communication to the HBA shall be via AHCI mechanisms.
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
    pub cmd: Cmd,
    _reserved0: u32,
    /// Task File Data.
    pub tfd: Tfd,
    /// Signature.
    pub sig: Sig,
    /// Serial ATA Status (SCR0: SStatus).
    pub ssts: SSts,
    /// Serial ATA Control (SCR2: SControl).
    pub sctl: SCtl,
    /// Serial ATA Error (SCR1: SError).
    pub serr: SErr,
    /// Serial ATA Active. (SCR3: SActive).
    pub sact: u32,
    /// Command Issue.
    pub ci: u32,
    /// Serial ATA Notification (SCR4: SNotification).
    pub sntf: u32,
    /// FIS-based Switching Control.
    pub fbs: Fbs,
    /// Device Sleep.
    pub devslp: DevSlp,
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

/// Command and Status.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct Cmd {
    /// Start (ST).
    ///
    /// When set, the HBA may process the command list.
    pub st: bool,

    /// Spin-Up Device (SUD).
    ///
    /// This bit is read/write for HBAs that support staggered spin-up via CAP.SSS. This bit is
    /// read only `1` for HBAs that do not support staggered spin-up. On an edge detect from `0` to
    /// `1`, the HBA shall start a COMRESET initialization sequence to the device.
    pub sud: bool,

    /// Power On Device (POD).
    ///
    /// This bit is read/write for HBAs that support cold presence detection on this port as
    /// indicated by CMD.CPD set. This bit is read only `1` for HBAs that do not support cold
    /// presence detect.
    pub pod: bool,

    /// Commmand List Override (CLO).
    ///
    /// Setting this bit causes TFE.STS.BSY and TFD.STS.DRQ to be cleared.
    pub clo: bool,

    /// FIS Receive Enable (FRE).
    ///
    /// When set, the HBA may post received FISes into the FIS receive area pointed to by FB (and
    /// for 64-bit HBAs, FBU).
    pub fre: bool,

    #[skip]
    __: B3,

    /// Current COmmand Slot (CCS).
    ///
    /// This field is valid when CMD.ST is set and shall be set to the command slot value of the
    /// command that is currently being issued by the HBA.
    #[skip(setters)]
    pub ocs: B5,

    /// Mechanical Presence Switch State (MPSS).
    ///
    /// The MPSS bit reports the state of a mechanical presence switch attached to this port. If
    /// CAP.SMPS is set and the mechanical presence switch is closed then this bit is cleared. If
    /// CAP.SMP is set and the mechanical presence switch is open then this bit is set. If CAP.SMPS
    /// is cleared then this bit is cleared.
    #[skip(setters)]
    pub mpss: bool,

    /// FIS Receive Running (FR).
    ///
    /// When set, the FIS REceive DMA engine for the port is running.
    #[skip(setters)]
    pub fr: bool,

    /// Command List Running (CR).
    ///
    /// When this bit is set, the commnad list DMA engine for the port is running.
    #[skip(setters)]
    pub cr: bool,

    /// Cold Presence State (CPS).
    ///
    /// The CPS bit reports whether a device is currently detected on this port via cold presence
    /// detection.
    #[skip(setters)]
    pub cps: bool,

    /// Port Multiplier Attached (PMA).
    ///
    /// This bit is read/write for HBAs that support a Port Multiplier (CAP.SPM = `1`). When set by
    /// software, a Port Multiplier is attached to the HBA for this port. Software is responsible
    /// for detecting whether a Port Multiplier is present.
    pub pma: bool,

    /// Hot Plug Capable Port (HPCP).
    ///
    /// When set, indicates that this port's signal and power connectors are externally accessible
    /// via a joint signal and power connector for blindmate device hot plug. When cleared,
    /// indicates that this port's signal and power connectors are not externally accessible via a
    /// joint signal and power connector.
    #[skip(setters)]
    pub hpcp: bool,

    /// Mechanial Presence Switch Attatched to Port (MPSP).
    ///
    /// If set, the platform supports an mechanical presence switch attached to this port.
    #[skip(setters)]
    pub mpsp: bool,

    /// Cold Presence Detection (CPD).
    ///
    /// If set, the platform supports cold presence detection on this port.
    #[skip(setters)]
    pub cpd: bool,

    /// External SATA Port (ESP).
    ///
    /// When set, indicates that this port's signal connector is externally accessible on a signal
    /// only connector (e.g. eSATA connector).
    #[skip(setters)]
    pub esp: bool,

    /// FIS-based Switching Capable Port (FBSCP).
    ///
    /// When set, indicates that this port supports Port Multiplier FIS-based switching.
    #[skip(setters)]
    pub fbscp: bool,

    /// Automatic Paritla to Slumber Transitions Enabled (APSTE).
    ///
    /// When set, the HBA may perform Automatic Partial to Slumber Transitions.
    pub apste: bool,

    /// Device is ATAPI (ATAPI).
    ///
    /// When set, the connected device is an ATAPI device. This bit is used by the HBA to control
    /// whether or not to generate the desktop LED when commands are active.
    pub atapi: bool,

    /// Drive LED on ATAPI Enable (DLAE).
    ///
    /// When set, the HBA shall drive the LED pin active for commands regardless of the state of
    /// CMD.ATAPI.
    pub dlae: bool,

    /// Aggressive Link Power Management Enable (ALPE).
    ///
    /// When set, the HBA shall aggressively enter a lower link power state (Partial or Slumber)
    /// based upon the setting of the ASP bit. If CAP.SALP is cleared, software shall treat this
    /// bit as reserved.
    pub alpe: bool,

    /// Aggressive Slumber / Partial (ASP).
    ///
    /// When set, and ALPE is set, the HBA shall aggressively enter the Slumber state when it
    /// cleares the CI register and the SACT register is cleared or when it clears the SACT
    /// register and CI is cleared. If CAP.SALP is cleared, software shall treat this bit as
    /// reserved.
    pub asp: bool,

    /// Interface Communication Control (ICC).
    ///
    /// This field is used to control power management states of the interface.
    ///
    ///
    /// | Value | Definition |
    /// | --- | --- |
    /// | 0x0 | No-Op/Idle |
    /// | 0x1 | Active |
    /// | 0x2 | Partial |
    /// | 0x6 | Slumber |
    /// | 0x8 | DevSleep |
    /// | 0x3-0x5, 0x7, 0x9-0xF | Reserved |
    pub icc: B4,
}

/// This is a 32-bit register that copies specific fields of the task file when FISes are received.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct Tfd {
    /// Status (STS).
    ///
    /// Contains the lates copy of the task file staus register.
    #[skip(setters)]
    pub sts: TfdStatus,

    /// Error (ERR).
    ///
    /// Contains the latest copy of the task file error register.
    #[skip(setters)]
    pub err: u8,

    #[skip]
    __: u16,
}

/// Represents task file status register.
#[bitfield(bits = 8)]
#[derive(Debug, BitfieldSpecifier)]
pub struct TfdStatus {
    /// Indicates an error during the transfer.
    #[skip(setters)]
    pub err: bool,

    /// Command specific.
    #[skip(setters)]
    pub cs0: B2,

    /// Indicates a data transfer is requested.
    #[skip(setters)]
    pub drq: bool,

    /// Command specific.
    #[skip(setters)]
    pub cs1: B3,

    /// Indicates the interface is busy.
    #[skip(setters)]
    pub bsy: bool,
}

impl Default for TfdStatus {
    fn default() -> Self {
        Self::from_bytes([0x7f])
    }
}

/// 32-bit register which contains the initial signature of an attached device when the first D2H
/// Register IFS is received from that device.
#[repr(C)]
#[derive(Debug, Default)]
pub struct Sig {
    count: u8,
    low: u8,
    mid: u8,
    high: u8,
}

impl Sig {
    /// Sector Count Register.
    pub fn count(&self) -> u8 {
        self.count
    }

    /// LBA Low Register.
    pub fn low(&self) -> u8 {
        self.low
    }

    /// LBA Mid Register.
    pub fn mid(&self) -> u8 {
        self.mid
    }

    /// LBA High Register.
    pub fn high(&self) -> u8 {
        self.high
    }
}

/// Conveys the current state of the interface and host.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct SSts {
    /// Device Dtection (DET).
    ///
    /// Indicates the interface device detection and Phy state.
    ///
    /// | Value | Description |
    /// | --- | --- |
    /// | 0 | No device detected and Phy communication not established. |
    /// | 1 | Device presence detected but Phy communication not established. |
    /// | 3 | Device presence detected and Phy communication established. |
    /// | 4 | Phy in offline mode as a result of the interface being disabled or running in a BIST loopback mode. |
    /// | others | reserved. |
    #[skip(setters)]
    pub det: B4,

    /// Cuurent Interface Speed (SPD).
    ///
    /// Indicates the negotiated interface communication speed.
    ///
    /// | Value | Description |
    /// | --- | --- |
    /// | 0 | Device not present or communication not established. |
    /// | 1 | Generation 1 communication rate negotiated. |
    /// | 2 | Generation 2 communication rate negotiated. |
    /// | 3 | Generation 3 communication rate negotiated. |
    /// | others | reserved. |
    #[skip(setters)]
    pub spd: B4,

    /// Interface Power Management (IPM)
    ///
    /// Indicates the current interface state.
    ///
    /// | Value | Description |
    /// | --- | --- |
    /// | 0 | Device not present or communication not established. |
    /// | 1 | Interface in active state. |
    /// | 2 | Interface in Partial power management state. |
    /// | 6 | Interface in Slumber power management state. |
    /// | 8 | Interface in DevSleep power management state. |
    /// | others | reserved. |
    #[skip(setters)]
    pub ipm: B4,

    #[skip]
    __: B20,
}

/// 32-bit register by which software controls SATA capabilities.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct SCtl {
    /// Device Detection Initialization (DET).
    ///
    /// Controls the HBA's device detection and interface initialization.
    ///
    /// | Value | Description |
    /// | --- | --- |
    /// | 0 | No device detection or initialization action requested. |
    /// | 1 | Perform interface communication initialization sequence to establish communication. |
    /// | 4 | Disable the Serial ATA interface and put Phy in offline mode. |
    /// | others | reserved. |
    pub det: B4,

    /// Speed Allowed (SPD).
    ///
    /// Indicates the highest allowable speed of the interface.
    ///
    /// | Value | Description |
    /// | --- | --- |
    /// | 0 | No speed negotiation restrictions |
    /// | 1 | Limit speed negotiation to Generation 1 communication rate. |
    /// | 2 | Limit speed negotiation to rage not greater than Generation 2 communication rate. |
    /// | 3 | Limit speed negotiation to rage not greater than Generation 3 communication rate. |
    /// | others | reserved. |
    pub spd: B4,

    /// Interface Power Management Transitions Allowed (IPM).
    ///
    /// Indicates which power states the HBA is allowed to transition to.
    ///
    /// | Value | Description |
    /// | --- | --- |
    /// | 0 | No intarface restrictions. |
    /// | 1 | Transitions to the Partial state disabled. |
    /// | 2 | Transitions to the Slumber state disabled. |
    /// | 3 | Transitions to the Partial and Slumber states disabled. |
    /// | 4 | Transitions to the DebSleep power management states disabled. |
    /// | 5 | Transitions to the Partial and DevSleep power management states disabled. |
    /// | 6 | Transitions to the Slumber and DevSleep power management states disabled. |
    /// | 7 | Transitions to the Partila, Slumber and DevSleep power management states disabled. |
    /// | others | reserved. |
    pub ipm: B4,

    /// Select Power Managment (SPM).
    ///
    /// This field is not used by AHCI.
    #[skip(setters)]
    pub spm: B4,

    /// Port Multiplier Port (PMP).
    ///
    /// This field is not used by AHCI.
    #[skip(setters)]
    pub pmp: B4,

    #[skip]
    __: B12,
}

/// Serial ATA Error. Consists of the lower 16-bit `ERR` and the upper 8-bit `DIAG`.
///
/// ## Error (ERR).
///
/// Contains error information for use by host software in determining the appropriate response to
/// the error condition.
///
/// ## Diagnostics (DIAG).
///
/// Contains diagnostic error information for use by diagnostic software in validating correct
/// operation of isolating failure modes.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct SErr {
    /// Recovered Data Integrity Error (I)
    ///
    /// A data integrity error occured that was recovered by the interface through a retry
    /// operation or other recovery action.
    pub err_i: bool,

    /// Recovered Communications Error (M).
    ///
    /// Communications between the device and host was temporarily lost but was re-established.
    pub err_m: bool,

    #[skip]
    __: B6,

    /// Transient Data Integrity Error (T).
    ///
    /// A data integrity error occurred that was not recovered by the interface.
    pub err_t: bool,

    /// Persistent Communication or Data Integrity Error (C).
    ///
    /// A communication error that was not recovered occurred that is expected to be persistent.
    pub err_c: bool,

    /// Protocol Error (P).
    ///
    /// A violatio of the Serial ATA protocol was detected.
    pub err_p: bool,

    /// Internal Error (I).
    ///
    /// The host bus adapter experienced an internal error that caused the operation to fail and
    /// may have put the host bus adapter into an error state.
    pub err_e: bool,

    #[skip]
    __: B4,

    /// PhyRdy Change (N).
    ///
    /// Indicates that the PhyRdy signal changed state.
    pub diag_n: bool,

    /// Phy Internal Error (I).
    ///
    /// Indicates that the Phy detected some internal error.
    pub diag_i: bool,

    /// Comm Wake (W).
    ///
    /// Indicates that a Comm Wake signal was detected by the Phy.
    pub diag_w: bool,

    /// 10B to 8B Decode Error (B).
    ///
    /// Indicates that one or more 10B to 8B decoding errors occurred.
    pub diag_b: bool,

    /// Disparity Error (D).
    ///
    /// This field is not used by AHCI.
    pub diag_d: bool,

    /// CRC Error (C).
    ///
    /// Indicates that one or more CRC errors occurred with the Link Layer.
    pub diag_c: bool,

    /// Hnadshake Error (H).
    ///
    /// Indicates that one or more R_ERR handshake response was received in response to frame
    /// transmission.
    pub diag_h: bool,

    /// Link Sequence Error (S).
    ///
    /// Indicates that one or more Link state machine error conditions was encountered.
    pub diag_s: bool,

    /// Transport state transition error (T).
    ///
    /// Indicates that an error has occurred in the transition from one state to another within the
    /// Transport layer since the last time this bit was cleared.
    pub diag_t: bool,

    /// Unknown FIS Type (F).
    ///
    /// Indicates that one or more FISs were received by the Transport layer with good CRC, but had
    /// a type field that was not recognized/known.
    pub diag_f: bool,

    /// Exchanged (X).
    ///
    /// When set, indicates that a change in device presence has been detected since the last time
    /// cleared.
    pub diax_x: bool,

    #[skip]
    __: B5,
}

/// Used to control and obtain status for Port Multiplier FIS-based switching.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct Fbs {
    /// Enable (EN).
    ///
    /// When set, a Port Multiplier is attached and the HBA shall use FIS-based switching to
    /// communicate with it.
    pub en: bool,

    /// Device Error Clear (DEC).
    ///
    /// When set by software, the HBA shall clear the device specific error condition and the HBA
    /// shall flush any commands outstanding for the device that experienced the error, including
    /// clearing th CI and SACT for that device.
    pub dec: bool,

    /// Single Device Error (SDE).
    ///
    /// When set and a fatal error condition has occurred, hardware believes the error is localized
    /// to one device such that software's first error recovery step should be to utilize the
    /// FBS.DEC functionality.
    #[skip(setters)]
    pub sde: bool,

    #[skip]
    __: B5,

    /// Device To Issue (DEV).
    ///
    /// Set by software to the Port Multiplier port value of the next command to be issued to
    /// without fetching the command header.
    pub dev: B4,

    /// Active Device Optimazation (ADO).
    ///
    /// Exposes the number of active devices that the FIS-based switching implementation has bee
    /// optimized for.
    #[skip(setters)]
    pub ado: B4,

    /// Device With Error (DWE).
    ///
    /// Set by harware to the value of the Port Multiplier port number of the device that
    /// experienced a fatal error condition.
    #[skip(setters)]
    pub dwe: B4,

    #[skip]
    __: B12,
}

/// Device Sleep.
#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Debug, Default)]
pub struct DevSlp {
    /// Aggressive Device Sleep Enable (ADSE).
    ///
    /// When set, the HBA shall assert the DEVSLP signal after the port has been idle (CI = 0 and
    /// SACT = 0) for the amount of time specified by the DEVSLP.DITO.
    pub adse: bool,

    /// Device Sleep Present (DSP).
    ///
    /// If set, the platform supports Device Sleep on this port.
    #[skip(setters)]
    pub dsp: bool,

    /// Device Sleep Exit Timeout (DETO).
    ///
    /// Specifies the maximum duration (in approximate 1ms granularity) from DEVSLP de-assertion
    /// until the device is ready to accept OOB.
    ///
    /// Software shall only set this value when CMD.ST is cleared, DEVSLP.ADSE is cleared and prior
    /// to setting CMD.ICC to 8.
    pub deto: u8,

    /// Minimum Device Sleep Assertion Time (MDAT).
    ///
    /// Specifies the minimum amount of time (in 1ms granularity) that the HBA must assert the
    /// DEVSLP signal before it may be de-asserted.
    ///
    /// Software shall only set this value when CMD.ST is cleared, DEVSLP.ADSE is cleared and prior
    /// to setting CMD.ICC to 8.
    pub mdat: B5,

    /// Device Sleep Idle Timeout (DITO).
    ///
    /// Specifies the amount of the time (in approximate 1ms granularity) that the HBA shall wait
    /// before driving the DEVSLP signal.
    ///
    /// Software shall only set this value when CMD.ST is cleared and DEVSLP.ADSE is cleared.
    pub dito: B10,

    /// DITO Multiplier (DM).
    ///
    /// 0's based value that specifies the DITO multiplier that the HBA applies to the specified
    /// DITO value. The HBA computes the total idle timeout as a product of DM and DITO (i.e.
    /// DITOactual = DITO * (DM + 1)).
    #[skip(setters)]
    pub dm: B4,

    #[skip]
    __: B3,
}
