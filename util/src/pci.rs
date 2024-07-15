use core::{cell::UnsafeCell, fmt::Debug};

use crate::bitfield::BitField;

#[repr(C)]
#[derive(Debug)]
pub struct ConfigSpace {
    /// Identifies the manufacture of the device. Either 0 or 0xFFFF is invalid.
    pub vendor_id: u16,
    /// Identifies the particular device.
    pub device_id: u16,
    /// Provides coarse control over a device's ability to generate and respond to PCI cycles.
    command: UnsafeCell<u16>,
    /// Recorded status information for PCI bus related events.
    status: u16,
    /// Specifies a device specific revision identifier. This field should be viewed as a vendor
    /// defined extension to the `device_id`.
    pub revision_id: u8,
    pub interface: u8,
    pub sub_class: u8,
    pub base_class: u8,
    /// Specifieds the system cachline size in units of DWORDs (32 bits).
    pub cacheline_size: u8,
    /// Specifies the value of the Latency Timer for this PCI bus master in units of PCI bus clock.
    pub latency_timer: u8,
    /// Identifies the layout of the second part of the predefined header (0x10-0x3F) and also
    /// whether or not the device contains multiple functions.
    ///
    /// | Bits | Description |
    /// | ---: | :--- |
    /// | 6:0 | Identifying the layout of the second part. |
    /// | 7 | If set, the device is single function one. If cleared, multi-function one. |
    pub header_ty: u8,
    /// Buit-in Self Test.
    pub bist: u8,

    // What fields below indicate depend on `hedder_ty`. Now only header 0x00 is supported.
    // TODO: Support header type 0x01 and 0x02.
    /// # For memory maps
    ///
    /// | bit | Description |
    /// | :---: | :--- |
    /// | 0 | 0 (Memory Space Indicator) |
    /// | 2:1 | Type (00 - 32-bit, 10 - 64-bit, others - reserved) |
    /// | 3 | Prefetcheable |
    /// | 31:4 | Base Address |
    ///
    /// # For I/O maps
    ///
    /// | bit | Description |
    /// | :---: | :--- |
    /// | 0 | 1 (I/O Space Indicator) |
    /// | 1 | Reserved |
    /// | 31:2 | Base Address |
    pub bars: [u32; 6],
    /// Points to the Card Information Structure (CIS) for the CardBus card.
    pub cardbus_cis_pointer: u32,
    pub subsystem_vendor_id: u16,
    pub subsystem_id: u16,
    pub ex_rom_base_addr: u32,
    /// Points to a linked list of new capabilities implemented by this device. This register is
    /// only valid if the "Capabilities List" bit in the Status Register is set. The bottom two
    /// bits are reserved and should be set to 0b00. We should mask these bits offf before using.
    cap_ptr: u8,
    reserved: [u8; 7],
    /// Used to comunicate interrupt line routing information.
    pub interrupt_line: u8,
    /// Tells which interrupt pin the device (or device function) uses.
    pub interrupt_pin: u8,
    /// Specifies how long a burst period the device needs assuming a clock rate of 33 MHz in units
    /// of 1/4 microsecond.
    pub min_gnt: u8,
    /// Specifies how often the device needs to gain access to the PCI bus in units of 1/4
    /// microseconds.
    pub max_lat: u8,

    // Fields below are device dependent containing device specific information.
    pub device_dependent_region: [u8; 0xc0],
}

impl ConfigSpace {
    pub fn cap_ptr(&self) -> Option<u8> {
        if self.status.get_bit(4) {
            Some(self.cap_ptr & !0b11)
        } else {
            None
        }
    }
}
