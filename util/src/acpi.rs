//! ACPI descrpition tables defined by [ACPI Software Programming Model]. ACPI is the key elemnt in
//! OSPM (Operating System-directed configuration and Power Management.
//!
//! [ACPI Software Programming Model]:
//! https://uefi.org/specs/ACPI/6.5/05_ACPI_Software_Programming_Model.html

use core::{fmt::Debug, mem, ptr, slice};

use crate::paging::ADDRESS_CONVERTER;

use self::apic::Madt;

pub mod apic;

type Result<T> = core::result::Result<T, Error>;

/// Represents an error related to [acpi](self).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Represents the table's signature is invalid.
    InvalidSignature,
    /// Represents the table's checksum is invalid.
    InvalidCheckSum,
    /// Represents the table's revision is invalid.
    InvalidRevision,
    /// Represents the table's length is invalid.
    InvalidTableLength,
    /// Represents this table is not supported now. And `[u8; 4]` is its signature. The signature
    /// is valid ASCII string.
    NotSupportedTable([u8; 4]),
    /// Represents the table's revision is not supported. `u8` value is its revision.
    NotSupportedRevision(u8),
    /// Represents the table is unaligned.
    UnAligned,
    /// Represents the index is out of the range.
    IndexOutOfRange,
}

/// Root System Description Pointer structure.
///
/// Now we only support ACPI 2.0.
#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct Rsdp {
    /// Must be "RSD PTR ".
    sig: [u8; 8],
    /// First 20 bytes including this field must sum to zero.
    check_sum: u8,
    /// Identifies the OEM.
    oemid: [u8; 6],
    /// The revision of the structure. For ACPI version 1.0, this is zero. Current value is 2.
    revision: u8,
    /// 32 bit physical address of the RSDT.
    #[allow(unused)]
    rsdt_address: u32,

    // Fields below are not availabe in the ACPI version 1.0.
    /// The length of the table, in bytes, including the header.
    length: u32,
    /// 64 bit physical address of the XSDT.
    xsdt_address: u64,
    /// This is a checksum of the entire table, including both checksum fields.
    extended_check_sum: u8,
    /// Reserved field.
    reserved: [u8; 3],
}

impl Rsdp {
    /// The length of RSDP for ACPI 2.0.
    const LEN2: usize = 36;

    /// Converts raw pointer `ptr` to Rust valid reference `&Rsdp`. Containing any invalid fields
    /// results in `Err` value.
    ///
    /// # Safety
    ///
    /// `ptr` must be a proper pointer to a RSDP.
    pub unsafe fn from_ptr(ptr: *const Self) -> Result<&'static Self> {
        if ptr.is_aligned() {
            return Err(Error::UnAligned);
        }
        let ret = unsafe { &*ptr };

        // Check if the table fields are valid.
        if ret.sig != *b"RSD PTR " {
            return Err(Error::InvalidSignature);
        }
        if ret.revision != 2 {
            if ret.revision < 2 {
                return Err(Error::NotSupportedRevision(ret.revision));
            } else {
                return Err(Error::InvalidRevision);
            }
        }
        if ret.length != Self::LEN2 as u32 {
            return Err(Error::InvalidTableLength);
        }
        if sum_bytes(ret, 20) != 0 {
            return Err(Error::InvalidCheckSum);
        }
        if sum_bytes(ret, 36) != 0 {
            return Err(Error::InvalidCheckSum);
        }

        Ok(ret)
    }

    /// Returns the reference to [Rsdt] indicated by `Rsdp.rsdt_address` if it is valid Rsdt.
    pub fn rsdt(&'static self) -> Result<&'static Rsdt> {
        Rsdt::from_ptr(
            ADDRESS_CONVERTER
                .as_ref()
                .get_addr(self.rsdt_address as _)
                .unwrap() as _,
        )
    }

    /// Returns the reference to [Xsdt] indicated by `Rsdp.xsdt_address` if it is valid Xsdt.
    pub fn xsdt(&'static self) -> Result<&'static Xsdt> {
        Xsdt::from_ptr(
            ADDRESS_CONVERTER
                .as_ref()
                .get_addr(self.xsdt_address)
                .unwrap() as _,
        )
    }
}

/// Represents ACPI [System Description Table Header].
///
/// [System Description Table Header]:
/// https://uefi.org/specs/ACPI/6.5/05_ACPI_Software_Programming_Model.html#system-description-table-header
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct TableHeader {
    /// The ASCII string representation of the table identifier.
    sig: [u8; 4],
    /// The length of the table.
    len: u32,
    /// The revision of the structure.
    revision: u8,
    /// The entire table, including the checksum field, must add to zero.
    check_sum: u8,
    /// An OEM-supplied string that identifies the OEM.
    oemid: [u8; 6],
    /// An OEM-supplied strig that the OEM uses to identify the particular data table.
    oem_table_id: u64,
    /// An OEM-supplied revision number.
    oem_revision: u32,
    /// Vendor ID.
    creator_id: u32,
    /// Revision of utility that created the table.
    creator_revision: u32,
    // Fields below are `u64` entries up to offset reaching `TableHeader.len`.
}

impl TableHeader {
    /// Constructs a reference to a [TableHeader] from `ptr`. Containing any invalid fields results
    /// in `Err`.
    fn from_ptr(ptr: *const Self) -> Result<&'static Self> {
        let ret = unsafe { &*ptr };
        // let ret: &Self = unsafe {
        //     &*ADDRESS_CONVERTER
        //         .as_ref()
        //         .get_ptr(ptr as _)
        //         .map_or_else(|| ptr, |ptr| ptr.as_ptr())
        // };

        // Checks if the fields are valid.
        if sum_bytes(ret, ret.len as _) != 0 {
            return Err(Error::InvalidCheckSum);
        }

        Ok(ret)
    }

    /// Returns the length of entries, in bytes, in the table under this header field.
    fn entries_len(&self) -> usize {
        self.len as usize - mem::size_of::<TableHeader>()
    }
}

/// Represents whole description tables.
#[derive(Debug, Clone, Copy)]
pub enum DescriptionTable {
    /// [Rsdt].
    Rsdt(&'static Rsdt),
    /// [Xsdt].
    Xsdt(&'static Xsdt),
    /// [Fadt].
    Fadt(&'static Fadt),
    /// [Madt].
    Madt(&'static Madt),
    /// [Mcfg].
    Mcfg(&'static Mcfg),
    /// Represents ACPI Description Tables that are not supported now.
    Unsupported(UnsupportedTable),
}

impl DescriptionTable {
    /// Converts to [DescriptionTable] from address `ptr`.
    fn from_ptr(ptr: usize) -> Result<Self> {
        let header = TableHeader::from_ptr(ptr as *const _)?;
        match &header.sig {
            b"RSDT" => Ok(Self::Rsdt(Rsdt::from_header(header))),
            b"XSDT" => Ok(Self::Xsdt(Xsdt::from_header(header))),
            b"FACP" => Ok(Self::Fadt(Fadt::from_header(header))),
            b"APIC" => Ok(Self::Madt(Madt::from_header(header))),
            b"MCFG" => Ok(Self::Mcfg(Mcfg::from_header(header))),
            // TODO: Implement below tables.
            b"BERT" | b"BGRT" | b"CCEL" | b"CPEP" | b"DSDT" | b"ECDT" | b"EINJ" | b"ERST"
            | b"FACS" | b"GTDT" | b"HEST" | b"MISC" | b"MSCT" | b"MPST" | b"NFIT" | b"PCCT"
            | b"PHAT" | b"PMTT" | b"PPTT" | b"PSDT" | b"RASF" | b"RAS2" | b"SBST" | b"SDEV"
            | b"SLIT" | b"SRAT" | b"SSDT" | b"SVKL" | b"AEST" | b"AGDI" | b"APMT" | b"BDAT"
            | b"BOOT" | b"CEDT" | b"CSRT" | b"DBGP" | b"DBG2" | b"DMAR" | b"DRTM" | b"DTPR"
            | b"ETDT" | b"HPET" | b"IBFT" | b"IERS" | b"IORT" | b"IVRS" | b"KEYP" | b"LPIT"
            | b"MCHI" | b"MHSP" | b"MPAM" | b"MSDM" | b"NBFT" | b"PRMT" | b"PGRT" | b"SDEI"
            | b"SLIC" | b"SPCR" | b"SPMI" | b"STAO" | b"SWFT" | b"TCPA" | b"TPM2" | b"UEFI"
            | b"WAET" | b"WDAT" | b"WDDT" | b"WDRT" | b"WPBT" | b"WSMT" | b"XENV" => {
                Ok(Self::Unsupported(UnsupportedTable(header.sig)))
            }
            _ => {
                if header.sig.starts_with(b"OEM") {
                    Err(Error::NotSupportedTable(header.sig))
                } else {
                    Err(Error::InvalidSignature)
                }
            }
        }
    }
}

/// Represents Extended System Description Table.
#[repr(C, packed)]
pub struct Xsdt {
    header: TableHeader,
    entry: [u64],
}

impl Xsdt {
    /// Returns the number of entries in the table.
    pub fn entries_count(&self) -> usize {
        self.header.entries_len() / mem::size_of::<u64>()
    }

    /// Returns `index`-th entry if it exists.
    pub fn entry(&self, index: usize) -> Result<DescriptionTable> {
        if index < self.entries_count() {
            Ok(DescriptionTable::from_ptr(unsafe {
                ADDRESS_CONVERTER
                    .as_ref()
                    .get_addr(ptr::addr_of!(self.entry[index]).read_unaligned())
                    .unwrap()
            } as _)?)
        } else {
            Err(Error::IndexOutOfRange)
        }
    }

    /// Returns the iterator to the whole entries.
    pub fn entries(&self) -> SystemDescriptionIter<u64> {
        SystemDescriptionIter {
            start: ptr::addr_of!(self.entry[0]),
            count: self.entries_count(),
            index: 0,
        }
    }

    /// Returns a reference to [Xsdt] pointed by `ptr`. Containing any nvalid fields results in
    /// `Err`.
    fn from_ptr(ptr: usize) -> Result<&'static Xsdt> {
        let header = TableHeader::from_ptr(ptr as *const _)?;

        // Checks fields.
        if header.sig != *b"XSDT" {
            return Err(Error::InvalidSignature);
        }

        Ok(Self::from_header(header))
    }

    fn from_header(header: &'static TableHeader) -> &'static Xsdt {
        // Make fat pointer because `Xsdt` contains an unsized field.
        let fat_ptr = ptr::slice_from_raw_parts(
            (header as *const TableHeader).cast::<u64>(),
            header.entries_len() / mem::size_of::<u64>(),
        );
        unsafe { &*(fat_ptr as *const _ as *const Xsdt) }
    }
}

impl Debug for Xsdt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Xsdt")
            .field("header", &self.header)
            .finish_non_exhaustive()
    }
}

/// Iterates entries that `Xsdt` contains.
#[derive(Debug, Clone)]
pub struct SystemDescriptionIter<Entry: Copy + Into<u64>> {
    /// Pointer to start of the entries.
    start: *const Entry,
    count: usize,
    index: usize,
}

impl<Entry: Copy + Into<u64>> Iterator for SystemDescriptionIter<Entry> {
    type Item = DescriptionTable;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.count <= self.index {
                return None;
            } else if let Ok(ret) = DescriptionTable::from_ptr(
                ADDRESS_CONVERTER
                    .as_ref()
                    .get_addr(
                        unsafe {
                            self.start
                                .byte_add(self.index * mem::size_of::<Entry>())
                                .read_unaligned()
                        }
                        .into() as _,
                    )
                    .unwrap() as _,
            ) {
                self.index += 1;
                return Some(ret);
            }
            self.index += 1;
        }
    }
}

/// Represents Root System Description Table.
#[repr(C)]
#[derive(Debug)]
pub struct Rsdt {
    header: TableHeader,
    entry: [u32],
}

impl Rsdt {
    /// Returns the number of entries in the table.
    pub fn entries_count(&self) -> usize {
        self.header.entries_len() / mem::size_of::<u32>()
    }

    /// Returns `index`-th entry if it exists.
    pub fn entry(&self, index: usize) -> Result<DescriptionTable> {
        if index < self.entries_count() {
            Ok(DescriptionTable::from_ptr(
                ADDRESS_CONVERTER
                    .as_ref()
                    .get_addr(unsafe { ptr::addr_of!(self.entry[index]).read_unaligned() } as _)
                    .unwrap() as _,
            )?)
        } else {
            Err(Error::IndexOutOfRange)
        }
    }

    /// Returns the iterator to the whole entries.
    pub fn entries(&self) -> SystemDescriptionIter<u32> {
        SystemDescriptionIter {
            start: ptr::addr_of!(self.entry[0]),
            count: self.entries_count(),
            index: 0,
        }
    }

    /// Returns a reference to [Xsdt] pointed by `ptr`. Containing any nvalid fields results in
    /// `Err`.
    fn from_ptr(ptr: usize) -> Result<&'static Self> {
        let header = TableHeader::from_ptr(ptr as *const _)?;

        // Checks fields.
        if header.sig != *b"RSDT" {
            return Err(Error::InvalidSignature);
        }

        Ok(Self::from_header(header))
    }

    fn from_header(header: &'static TableHeader) -> &'static Self {
        // Make fat pointer because `Rsdt` contains an unsized field.
        let fat_ptr = ptr::slice_from_raw_parts(
            (header as *const TableHeader).cast::<u32>(),
            header.entries_len() / mem::size_of::<u32>(),
        );
        unsafe { &*(fat_ptr as *const Self) }
    }
}

/// Represents [FADT] (Fixed ACPI Description Table).
///
/// [FADT]:
/// https://uefi.org/specs/ACPI/6.5/05_ACPI_Software_Programming_Model.html#fixed-acpi-description-table-fadt
#[repr(C, packed)]
pub struct Fadt {
    header: TableHeader,
    /// Physical memory address of the [FACS], where OSPM and Firmware exchange control informatin.
    ///
    /// [FACS]:
    /// https://uefi.org/specs/ACPI/6.5/05_ACPI_Software_Programming_Model.html#firmware-acpi-control-structure-facs
    pub firmware_ctrl: u32,
    /// Physical memory address of the [DSDT]. If [`Fadt::x_dsdt`] field contains non-zero value,
    /// this field must be ignored.
    ///
    /// [DSDT]:
    /// https://uefi.org/specs/ACPI/6.5/05_ACPI_Software_Programming_Model.html#differentiated-system-description-table-dsdt
    pub dsdt: u32,
    /// ACPI 1.0 defined this offset as a field named INT_MODEL, which was eliminated in ACPI 2.0.
    reserved0: u8,
    /// Conveys the preferred power management profile to OSPM.
    ///
    /// | Value | Description |
    /// | :---: | :--- |
    /// | 0 | Unspecified |
    /// | 1 | Desktop |
    /// | 2 | Mobile |
    /// | 3 | Workstation |
    /// | 4 | Enterprise Server |
    /// | 5 | SOHO Server |
    /// | 6 | Appliance PC |
    /// | 7 | Performance Server |
    /// | 8 | Tablet |
    /// | >8 | Reserved |
    pub preferred_pm_profile: u8,
    /// In 8259 mode, represents system vector the SCI (System Control Interrupt) interrupt wited
    /// to. Otherwise, contains the Global System interrupt number of the SCI interrupt.
    pub sci_int: u16,
    /// System port address of the SMI (System Management Interrupt) Command Port.
    pub smi_cmd: u32,
    /// The value to write to SMI_CMD to disable ownership of the ACPI hardware registers.
    pub acpi_enable: u8,
    /// The value to write to SMI_CMD to re-enable SMI ownership of the ACPI hardware registers.
    pub acpi_disable: u8,
    /// The value to write to SMI_CMD to enter the S4BIOS state. The S4BIOS state provides an
    /// alternate way to enter the S4 state where the firmware saves and restores the memory
    /// context.
    pub s4bios_req: u8,
    /// If non-zero, this field contains the value OSPM writes to the SMI_CMD register to assume
    /// procesor performance state control responsibility.
    pub pstate_cnt: u8,
    /// System root address of the PM1a Event Register Block.
    pub pm1a_evt_blk: u32,
    /// System root address of the PM1b Event Register Block.
    pub pm1b_evt_blk: u32,
    /// System port address of the PM1a Control Registe Block.
    pub pm1a_cnt_blk: u32,
    /// System port address of the PM1b Control Registe Block.
    pub pm1b_cnt_blk: u32,
    /// System port address of the PM2 Control Registe Block.
    pub pm2_cnt_blk: u32,
    /// System port address of the Power Management Timer Control Register Block.
    pub pm_tmr_blk: u32,
    /// System port address of General-Purpose Event 0 Register Block.
    pub gpe0_blk: u32,
    /// System port address of General-Purpose Event 1 Register Block.
    pub gpe1_blk: u32,
    /// Number of bytes decoded by [pm1a_evt_blk](Self::pm1a_evt_blk).
    pub pm1_evt_len: u8,
    /// Number of bytes decoded by [pm1_cnt_blk](Self::pm1_cnt_blk).
    pub pm1_cnt_len: u8,
    /// Number of bytes decoded by [pm2_cnt_blk](Self::pm2_cnt_blk).
    pub pm2_cnt_len: u8,
    /// Number of bytes decoded by [pm_tmr_blk](Self::pm_tmr_blk).
    pub pm_tmr_len: u8,
    /// The length of the register whose address is given by [x_gpe0_blk](Self::x_gpe0_blk).
    pub gpe0_blk_len: u8,
    /// The length of the register whose address is given by [x_gpe1_blk](Self::x_gpe1_blk).
    pub gpe1_blk_len: u8,
    /// Offset within the ACPI general-purpose event model where GPE1 based events start.
    pub gpe1_base: u8,
    /// If non-zero, this field contains the value OSPM writes to the [smi_cmd](Self::smi_cmd)
    /// register to indicate OS support for the _CST object and C States Changed notification.
    pub cst_cnt: u8,
    /// The worst-case hardware latency, in microseconds, to enter and exit a C2 state.
    pub p_lvl2_lat: u16,
    /// The worst-case hardware latency, in microseconds, to enter and exit a C3 state.
    pub p_lvl3_lat: u16,
    /// Skip.
    // TODO: Understand the description below and write a comment.
    //       https://uefi.org/specs/ACPI/6.5/05_ACPI_Software_Programming_Model.html#fixed-acpi-description-table-fadt
    pub flush_size: u16,
    /// Skip.
    pub flush_stride: u16,
    /// The zero-based index of where the processor's duty cycle settings is within the processor's
    /// P_CNT register.
    pub duty_offset: u8,
    /// The bit width of the processor's duty cycle setting value in the P_CNT register.
    pub duty_width: u8,
    /// The RTC CMOS RAM index to the day-of-month alarm value.
    pub day_alrm: u8,
    /// The RTC CMOS RAM index to the month of year alarm avlue.
    pub mon_alrm: u8,
    /// The RTC CMOS RAM index to the century of data value.
    pub century: u8,
    /// IA-PC Boot  Architecture Flags.
    pub iapc_boot_arch: u16,
    /// Must be 0.
    reserved1: u8,
    /// Fixed feature flags. See [here].
    ///
    /// [here]:
    /// https://uefi.org/specs/ACPI/6.5/05_ACPI_Software_Programming_Model.html#fixed-acpi-description-table-fixed-feature-flags
    pub flags: u32,
    /// THe address of the reset register representd in Generic Address Structure format.
    pub reset_reg: [u8; 12],
    /// Indicates the value to write to the [reset_reg](Self::reset_reg) port to reset the system.
    pub reset_value: u8,
    /// ARM Boot Architecture Flags. See [here].
    ///
    /// [here]:
    /// https://uefi.org/specs/ACPI/6.5/05_ACPI_Software_Programming_Model.html#arm-boot-architecture-flags
    pub arm_boot_arch: u16,
    /// Minor Version of this [Fadt] structure.
    pub fadt_minor_version: u8,
    /// Extended physical address of the FACS.
    pub x_firmware_ctrl: u64,
    /// Extended physical address of the DSDT.
    pub x_dsdt: u64,
    /// Extended address of the PM1a Event Register Block, represented in Generic Address Structure
    /// format.
    pub x_pm1a_evt_blk: [u8; 12],
    /// Extended address of the PM1b Event Register Block, represented in Generic Address Structure
    /// format.
    pub x_pm1b_evt_blk: [u8; 12],
    /// Extended address of the PM1a Control Register Block, represented in Generic Address
    /// Structure format.
    pub x_pm1a_cnt_blk: [u8; 12],
    /// Extended address of the PM1b Control Register Block, represented in Generic Address
    /// Structure format.
    pub x_pm1b_cnt_blk: [u8; 12],
    /// Extended address of the PM2 Control Register Block, represented in Generic Address
    /// Structure format.
    pub x_pm2_cnt_blk: [u8; 12],
    /// Extended address of the Power Management Timer Control Register Block, represented in
    /// Generic Address Structure format.
    pub x_pm_tmr_blk: [u8; 12],
    /// Extended address of the General-Purpose Event 0 Register Block, represented in Generic
    /// Address Structure format.
    pub x_gpe0_blk: [u8; 12],
    /// Extended address of the General-Purpose Event 1 Register Block, represented in Generic
    /// Address Structure format.
    pub x_gpe1_blk: [u8; 12],
    /// The address of the Sleep register, represented in Generic Address Structure format.
    pub sleep_control_reg: [u8; 12],
    /// The address of the Sleep status register, represented in Generic Address Structure format.
    pub sleep_status_reg: [u8; 12],
    /// 64-bit identifier of hypervisor vendor.
    pub hypervisor_vendor_identity: u64,
}

impl Fadt {
    /// Returns [Fadt] Major Version.
    pub fn major_version(&self) -> u8 {
        self.header.revision
    }

    fn from_header(header: &'static TableHeader) -> &'static Self {
        unsafe { &*(header as *const TableHeader).cast() }
    }
}

impl Debug for Fadt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Fadt")
            .field("header", &self.header)
            .field("firmware_ctrl", &unsafe {
                ptr::addr_of!(self.firmware_ctrl).read_unaligned()
            })
            .field("dsdt", &unsafe {
                ptr::addr_of!(self.dsdt).read_unaligned()
            })
            .field("reserved0", &unsafe {
                ptr::addr_of!(self.reserved0).read_unaligned()
            })
            .field("preferred_pm_profile", &unsafe {
                ptr::addr_of!(self.preferred_pm_profile).read_unaligned()
            })
            .field("sci_int", &unsafe {
                ptr::addr_of!(self.sci_int).read_unaligned()
            })
            .field("smi_cmd", &unsafe {
                ptr::addr_of!(self.smi_cmd).read_unaligned()
            })
            .field("acpi_enable", &unsafe {
                ptr::addr_of!(self.acpi_enable).read_unaligned()
            })
            .field("acpi_disable", &unsafe {
                ptr::addr_of!(self.acpi_disable).read_unaligned()
            })
            .field("s4bios_req", &unsafe {
                ptr::addr_of!(self.s4bios_req).read_unaligned()
            })
            .field("pstate_cnt", &unsafe {
                ptr::addr_of!(self.pstate_cnt).read_unaligned()
            })
            .field("pm1a_evt_blk", &unsafe {
                ptr::addr_of!(self.pm1a_evt_blk).read_unaligned()
            })
            .field("pm1b_evt_blk", &unsafe {
                ptr::addr_of!(self.pm1b_evt_blk).read_unaligned()
            })
            .field("pm1a_cnt_blk", &unsafe {
                ptr::addr_of!(self.pm1a_cnt_blk).read_unaligned()
            })
            .field("pm1b_cnt_blk", &unsafe {
                ptr::addr_of!(self.pm1b_cnt_blk).read_unaligned()
            })
            .field("pm2_cnt_blk", &unsafe {
                ptr::addr_of!(self.pm2_cnt_blk).read_unaligned()
            })
            .field("pm_tmr_blk", &unsafe {
                ptr::addr_of!(self.pm_tmr_blk).read_unaligned()
            })
            .field("gpe0_blk", &unsafe {
                ptr::addr_of!(self.gpe0_blk).read_unaligned()
            })
            .field("gpe1_blk", &unsafe {
                ptr::addr_of!(self.gpe1_blk).read_unaligned()
            })
            .field("pm1_evt_len", &unsafe {
                ptr::addr_of!(self.pm1_evt_len).read_unaligned()
            })
            .field("pm1_cnt_len", &unsafe {
                ptr::addr_of!(self.pm1_cnt_len).read_unaligned()
            })
            .field("pm2_cnt_len", &unsafe {
                ptr::addr_of!(self.pm2_cnt_len).read_unaligned()
            })
            .field("pm_tmr_len", &unsafe {
                ptr::addr_of!(self.pm_tmr_len).read_unaligned()
            })
            .field("gpe0_blk_len", &unsafe {
                ptr::addr_of!(self.gpe0_blk_len).read_unaligned()
            })
            .field("gpe1_blk_len", &unsafe {
                ptr::addr_of!(self.gpe1_blk_len).read_unaligned()
            })
            .field("gpe1_base", &unsafe {
                ptr::addr_of!(self.gpe1_base).read_unaligned()
            })
            .field("cst_cnt", &unsafe {
                ptr::addr_of!(self.cst_cnt).read_unaligned()
            })
            .field("p_lvl2_lat", &unsafe {
                ptr::addr_of!(self.p_lvl2_lat).read_unaligned()
            })
            .field("p_lvl3_lat", &unsafe {
                ptr::addr_of!(self.p_lvl3_lat).read_unaligned()
            })
            .field("flush_size", &unsafe {
                ptr::addr_of!(self.flush_size).read_unaligned()
            })
            .field("flush_stride", &unsafe {
                ptr::addr_of!(self.flush_stride).read_unaligned()
            })
            .field("duty_offset", &unsafe {
                ptr::addr_of!(self.duty_offset).read_unaligned()
            })
            .field("duty_width", &unsafe {
                ptr::addr_of!(self.duty_width).read_unaligned()
            })
            .field("day_alrm", &unsafe {
                ptr::addr_of!(self.day_alrm).read_unaligned()
            })
            .field("mon_alrm", &unsafe {
                ptr::addr_of!(self.mon_alrm).read_unaligned()
            })
            .field("century", &unsafe {
                ptr::addr_of!(self.century).read_unaligned()
            })
            .field("iapc_boot_arch", &unsafe {
                ptr::addr_of!(self.iapc_boot_arch).read_unaligned()
            })
            .field("reserved1", &unsafe {
                ptr::addr_of!(self.reserved1).read_unaligned()
            })
            .field("flags", &unsafe {
                ptr::addr_of!(self.flags).read_unaligned()
            })
            .field("reset_reg", &unsafe {
                ptr::addr_of!(self.reset_reg).read_unaligned()
            })
            .field("reset_value", &unsafe {
                ptr::addr_of!(self.reset_value).read_unaligned()
            })
            .field("arm_boot_arch", &unsafe {
                ptr::addr_of!(self.arm_boot_arch).read_unaligned()
            })
            .field("fadt_minor_version", &unsafe {
                ptr::addr_of!(self.fadt_minor_version).read_unaligned()
            })
            .field("x_firmware_ctrl", &unsafe {
                ptr::addr_of!(self.x_firmware_ctrl).read_unaligned()
            })
            .field("x_dsdt", &unsafe {
                ptr::addr_of!(self.x_dsdt).read_unaligned()
            })
            .field("x_pm1a_evt_blk", &unsafe {
                ptr::addr_of!(self.x_pm1a_evt_blk).read_unaligned()
            })
            .field("x_pm1b_evt_blk", &unsafe {
                ptr::addr_of!(self.x_pm1b_evt_blk).read_unaligned()
            })
            .field("x_pm1a_cnt_blk", &unsafe {
                ptr::addr_of!(self.x_pm1a_cnt_blk).read_unaligned()
            })
            .field("x_pm1b_cnt_blk", &unsafe {
                ptr::addr_of!(self.x_pm1b_cnt_blk).read_unaligned()
            })
            .field("x_pm2_cnt_blk", &unsafe {
                ptr::addr_of!(self.x_pm2_cnt_blk).read_unaligned()
            })
            .field("x_pm_tmr_blk", &unsafe {
                ptr::addr_of!(self.x_pm_tmr_blk).read_unaligned()
            })
            .field("x_gpe0_blk", &unsafe {
                ptr::addr_of!(self.x_gpe0_blk).read_unaligned()
            })
            .field("x_gpe1_blk", &unsafe {
                ptr::addr_of!(self.x_gpe1_blk).read_unaligned()
            })
            .field("sleep_control_reg", &unsafe {
                ptr::addr_of!(self.sleep_control_reg).read_unaligned()
            })
            .field("sleep_status_reg", &unsafe {
                ptr::addr_of!(self.sleep_status_reg).read_unaligned()
            })
            .field("hypervisor_vendor_identity", &unsafe {
                ptr::addr_of!(self.hypervisor_vendor_identity).read_unaligned()
            })
            .finish()
    }
}

/// Represents PCI Express Memory-mapped Configuration. Each configuration ([PcieMmioConfig]) tells
/// you the base address of MMIO of a host bridge.
#[repr(C, packed)]
pub struct Mcfg {
    header: TableHeader,
    reserved: u64,
    config: [PcieMmioConfig],
}

impl Mcfg {
    /// Returns the number of [PcieMmioConfig] this [Mcfg] contains.
    pub fn entries_count(&self) -> usize {
        // There is an 8-byte reserved space.
        (self.header.entries_len() - mem::size_of::<u64>()) / mem::size_of::<PcieMmioConfig>()
    }

    /// Returns the slice of configurations that this structure contains.
    pub fn configs(&self) -> &[PcieMmioConfig] {
        unsafe { slice::from_raw_parts(ptr::addr_of!(self.config[0]), self.entries_count()) }
    }

    fn from_header(header: &'static TableHeader) -> &'static Self {
        let fat_ptr = ptr::slice_from_raw_parts(
            (header as *const TableHeader).cast::<u8>(),
            header.entries_len() / mem::size_of::<PcieMmioConfig>(),
        );
        unsafe { &*(fat_ptr as *const Self) }
    }
}

impl Debug for Mcfg {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let configs = self.configs();
        f.debug_struct("Mcfg")
            .field("header", &self.header)
            .field("config_base_address_alloc", &configs)
            .finish()
    }
}

/// Represents configuration space base address allocation structures that give you information
/// about host bridges.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct PcieMmioConfig {
    /// Base address of enhanced configuration mechanism, which is the starting physical address of
    /// MMIO.
    pub base_addr: u64,
    /// PCI Segment Group Number.
    pub pci_group: u16,
    /// Start PCI bus number decoded by this host bridge.
    pub start_bus: u8,
    /// End PCI bus number decoded by this host bridge.
    pub end_bus: u8,
    reserved: u32,
}

/// Represents a not yet supported table.
#[derive(Clone, Copy)]
pub struct UnsupportedTable([u8; 4]);

impl Debug for UnsupportedTable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = core::str::from_utf8(&self.0).unwrap_or_else(|_| {
            panic!("util internal error: do not construct UnsupportedTable with non ASCII bytes");
        });
        f.debug_tuple("UnsupportedTable").field(&s).finish()
    }
}

/// Sums up `bytes` bytes from `data` head address in bytes.
fn sum_bytes<T>(data: &T, bytes: usize) -> u8 {
    let data: &[u8] = unsafe { slice::from_raw_parts((data as *const T).cast(), bytes) };
    data.iter().sum()
}
