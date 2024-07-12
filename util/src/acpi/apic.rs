use core::{fmt::Debug, mem, slice};

use super::TableHeader;

/// Represents Multiple APIC Description Table.
#[repr(C, packed)]
pub struct Madt {
    header: TableHeader,
    /// The 32-bit physical address at which each processor cah access its local interrupt
    /// contoller.
    local_interrupt_controller_address: u32,
    /// Multiple APIC flags.
    ///
    /// | Multiple APIC Flags | Bit Length | Bit Offset | Description |
    /// | :--- | :--- | :--- | :--- |
    /// | PCAT_COMPAT | 1 | 0 | Indicates that the system has a PC-AT-compatible dual-8259 setup. |
    /// | Reserved | 31 | 1 | This value is zero.|
    flags: u32,
    /// A list of interrupt controller structures for this implementation.
    interrupt_controller_structure: [u8],
}

impl Madt {
    pub(super) fn from_header(header: &'static TableHeader) -> &'static Self {
        let fat_ptr = unsafe {
            slice::from_raw_parts(
                header as *const _ as *const u8,
                header.entries_len() / mem::size_of::<u8>(),
            )
        };
        unsafe { &*(fat_ptr as *const _ as *const _) }
    }
}

impl Debug for Madt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let address = self.local_interrupt_controller_address;
        let flags = self.flags;

        f.debug_struct("Madt")
            .field("header", &self.header)
            .field("local_interrupt_controller_address", &address)
            .field("flags", &flags)
            .finish_non_exhaustive()
    }
}
