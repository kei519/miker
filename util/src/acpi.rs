use core::{fmt::Debug, mem, ptr, slice};

type Result<T> = core::result::Result<T, Error>;

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

    /// Returns the reference to [Xsdt] indicated by `Rsdp.xsdt_address` if it is valid Xsdt.
    pub fn xsdt(&'static self) -> Result<&'static Xsdt> {
        Xsdt::from_ptr(self.xsdt_address as _)
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

        // Checks if the fields are valid.
        if sum_bytes(ret, ret.len as _) != 0 {
            return Err(Error::InvalidCheckSum);
        }

        Ok(ret)
    }

    /// Returns the number of entries in the table under this header field.
    fn entries_count(&self) -> usize {
        // Each `entry` field is a pointer to the next table, that is `u64.
        (self.len as usize - mem::size_of::<TableHeader>()) / mem::size_of::<u64>()
    }
}

/// Represents whole description tables.
#[derive(Debug, Clone, Copy)]
pub enum DescriptionTable {
    Xsdt(&'static Xsdt),
    Fadt(&'static Fadt),
}

impl DescriptionTable {
    /// Converts to [DescriptionTable] from address `ptr`.
    fn from_ptr(ptr: usize) -> Result<Self> {
        let header = TableHeader::from_ptr(ptr as *const _)?;
        match &header.sig {
            b"XSDT" => Ok(Self::Xsdt(Xsdt::from_header(header))),
            b"FACP" => Ok(Self::Fadt(Fadt::from_header(header))),
            // TODO: Implement below tables.
            b"APIC" | b"BERT" | b"BGRT" | b"CCEL" | b"CPEP" | b"DSDT" | b"ECDT" | b"EINJ"
            | b"ERST" | b"FACS" | b"GTDT" | b"HEST" | b"MISC" | b"MSCT" | b"MPST" | b"NFIT"
            | b"PCCT" | b"PHAT" | b"PMTT" | b"PPTT" | b"PSDT" | b"RASF" | b"RAS2" | b"RSDT"
            | b"SBST" | b"SDEV" | b"SLIT" | b"SRAT" | b"SSDT" | b"SVKL" | b"AEST" | b"AGDI"
            | b"APMT" | b"BDAT" | b"BOOT" | b"CEDT" | b"CSRT" | b"DBGP" | b"DBG2" | b"DMAR"
            | b"DRTM" | b"DTPR" | b"ETDT" | b"HPET" | b"IBFT" | b"IERS" | b"IORT" | b"IVRS"
            | b"KEYP" | b"LPIT" | b"MCFG" | b"MCHI" | b"MHSP" | b"MPAM" | b"MSDM" | b"NBFT"
            | b"PRMT" | b"PGRT" | b"SDEI" | b"SLIC" | b"SPCR" | b"SPMI" | b"STAO" | b"SWFT"
            | b"TCPA" | b"TPM2" | b"UEFI" | b"WAET" | b"WDAT" | b"WDDT" | b"WDRT" | b"WPBT"
            | b"WSMT" | b"XENV" => Err(Error::NotSupportedTable(header.sig)),
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
        self.header.entries_count()
    }

    /// Returns `index`-th entry if it exists.
    pub fn entry(&self, index: usize) -> Result<DescriptionTable> {
        if index < self.entries_count() {
            Ok(DescriptionTable::from_ptr(unsafe {
                ptr::addr_of!(self.entry[index]).read_unaligned()
            } as _)?)
        } else {
            Err(Error::IndexOutOfRange)
        }
    }

    /// Returns the iterator to the whole entries.
    pub fn entries(&self) -> XsdtEntryIter {
        XsdtEntryIter {
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

    /// Returns a reference to [Xsdt] from the header `header` of it. This funtion assumes `header`
    /// is a proper one for XSDT, so it does NOT check any conditions.
    fn from_header(header: &'static TableHeader) -> &'static Xsdt {
        // Make fat pointer because `Xsdt` contains an unsized field.
        let fat_ptr: &[u64] = unsafe {
            slice::from_raw_parts(
                (header as *const TableHeader).cast(),
                header.entries_count(),
            )
        };
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
pub struct XsdtEntryIter {
    /// Pointer to start of the entries.
    start: *const u64,
    count: usize,
    index: usize,
}

impl Iterator for XsdtEntryIter {
    type Item = DescriptionTable;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.count <= self.index {
                return None;
            } else if let Ok(ret) = DescriptionTable::from_ptr(unsafe {
                self.start
                    .byte_add(self.index * mem::size_of::<u64>())
                    .read_unaligned()
            } as _)
            {
                self.index += 1;
                return Some(ret);
            }
            self.index += 1;
        }
    }
}

#[repr(C, packed)]
pub struct Fadt {
    header: TableHeader,
    pub firmware_ctrl: u32,
    pub dsdt: u32,
    pub reserved0: u8,
    pub preferred_pm_profile: u8,
    pub sci_int: u16,
    pub smi_cmd: u32,
    pub acpi_enable: u8,
    pub acpi_disable: u8,
    pub s4bios_req: u8,
    pub pstate_cnt: u8,
    pub pm1a_evt_blk: u32,
    pub pm1b_evt_blk: u32,
    pub pm1a_cnt_blk: u32,
    pub pm1b_cnt_blk: u32,
    pub pm2_cnt_blk: u32,
    pub pm_tmr_blk: u32,
    pub gpe0_blk: u32,
    pub gpe1_blk: u32,
    pub pm1_evt_len: u8,
    pub pm1_cnt_len: u8,
    pub pm2_cnt_len: u8,
    pub pm_tmr_len: u8,
    pub gpe0_blk_len: u8,
    pub gpe1_blk_len: u8,
    pub gpe1_base: u8,
    pub cst_cnt: u8,
    pub p_lvl2_lat: u16,
    pub p_lvl3_lat: u16,
    pub flush_size: u16,
    pub flush_stride: u16,
    pub duty_offset: u8,
    pub duty_width: u8,
    pub day_alrm: u8,
    pub mon_alrm: u8,
    pub century: u8,
    pub iapc_boot_arch: u16,
    pub reserved1: u8,
    pub flags: u32,
    pub reset_reg: [u8; 12],
    pub reset_value: u8,
    pub arm_boot_arch: u16,
    pub fadt_minor_version: u8,
    pub x_firmware_ctrl: u64,
    pub x_dsdt: u64,
    pub x_pm1a_evt_blk: [u8; 12],
    pub x_pm1b_evt_blk: [u8; 12],
    pub x_pm1a_cnt_blk: [u8; 12],
    pub x_pm1b_cnt_blk: [u8; 12],
    pub x_pm2_cnt_blk: [u8; 12],
    pub x_pm_tmr_blk: [u8; 12],
    pub x_gpe0_blk: [u8; 12],
    pub x_gpe1_blk: [u8; 12],
    pub sleep_control_reg: [u8; 12],
    pub sleep_status_reg: [u8; 12],
    pub hypervisor_vendor_identity: u64,
}

impl Fadt {
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

/// Sums up `bytes` bytes from `data` head address in bytes.
fn sum_bytes<T>(data: &T, bytes: usize) -> u8 {
    let data: &[u8] = unsafe { slice::from_raw_parts((data as *const T).cast(), bytes) };
    data.iter().sum()
}