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
    pub fn entry(&self, index: usize) -> Option<u64> {
        if index < self.entries_count() {
            Some(self.entry[index])
        } else {
            None
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

        // Make fat pointer because `Xsdt` contains an unsized field.
        let fat_ptr: &[u64] = unsafe {
            slice::from_raw_parts(
                (header as *const TableHeader).cast(),
                header.entries_count(),
            )
        };
        Ok(unsafe { &*(fat_ptr as *const _ as *const Xsdt) })
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
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count <= self.index {
            None
        } else {
            let ret = unsafe {
                self.start
                    .byte_add(self.index * mem::size_of::<u64>())
                    .read_unaligned()
            };
            self.index += 1;
            Some(ret)
        }
    }
}

/// Sums up `bytes` bytes from `data` head address in bytes.
fn sum_bytes<T>(data: &T, bytes: usize) -> u8 {
    let data: &[u8] = unsafe { slice::from_raw_parts((data as *const T).cast(), bytes) };
    data.iter().sum()
}
