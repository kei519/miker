use core::slice;

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
}

/// Sums up `bytes` bytes from `data` head address in bytes.
fn sum_bytes<T>(data: &T, bytes: usize) -> u8 {
    let data: &[u8] = unsafe { slice::from_raw_parts((data as *const T).cast(), bytes) };
    data.iter().sum()
}
