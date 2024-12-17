//! Provides PCI supports.

use core::cell::UnsafeCell;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::{bitfield::BitField, paging::ADDRESS_CONVERTER};

/// Represetns a PCI configuration space.
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
    /// Specifies register-level interface of a device Function.
    pub interface: u8,
    /// Identifies more specifically the type of functionalyty that the device Function provides.
    pub sub_class: u8,
    /// Classifies the type of functionality that the device Function provides.
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
    /// Subsystem Vendor ID.
    pub subsystem_vendor_id: u16,
    /// Subsystem ID.
    pub subsystem_id: u16,
    /// Expansion ROM base address.
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

    /// Fields below are device dependent containing device specific information.
    pub device_dependent_region: [u8; 0xc0],
}

impl ConfigSpace {
    /// Returns exclusive reference to memory at physical address `ptr`.
    ///
    /// # Panic
    ///
    /// If there is no map from physical address `ptr` to virtual one, it causes panic.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null and properly aligned.
    pub unsafe fn from_ptr(ptr: *mut u8) -> &'static mut Self {
        // Safety: caller guarantees.
        unsafe {
            &mut *ADDRESS_CONVERTER
                .as_ref()
                .get_ptr(ptr as u64)
                .unwrap()
                .as_ptr()
        }
    }
    /// Returns [Self::cap_ptr] if it is valid.
    pub fn cap_ptr(&self) -> Option<u8> {
        if self.status.get_bit(4) {
            Some(self.cap_ptr & !0b11)
        } else {
            None
        }
    }
}

/// Represetns a collection of all PCI configuration spaces.
#[derive(Debug, Clone, Copy)]
pub struct ConfigSpaces {
    base: u64,
}

impl ConfigSpaces {
    /// Returns new [ConfigSpaces].
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null and properly aligned.
    pub unsafe fn from_base_ptr(base: *mut u8) -> Self {
        Self { base: base as _ }
    }

    /// Returns the shared reference of [ConfigSpace] if specified `bus`, `dev` and `func` is
    /// valid.
    pub fn get(&self, bus: u8, dev: u8, func: u8) -> Option<&'static ConfigSpace> {
        if !(0..32).contains(&dev) || !(0..8).contains(&func) {
            return None;
        }

        let offset = ((bus as u64) << 16) + ((dev as u64) << 11) + ((func as u64) << 8);
        let config = unsafe { ConfigSpace::from_ptr((self.base + offset) as _) };
        (![0x0000, 0xffff].contains(&config.vendor_id)).then_some(config)
    }

    /// Returns the exclusive reference of [ConfigSpace] if specified `bus`, `dev` and `func` is
    /// valid.
    pub fn get_mut(&mut self, bus: u8, dev: u8, func: u8) -> Option<&'static mut ConfigSpace> {
        if !(0..32).contains(&dev) || !(0..8).contains(&func) {
            return None;
        }

        let offset = ((bus as u64) << 16) + ((dev as u64) << 11) + ((func as u64) << 8);
        let config = unsafe { ConfigSpace::from_ptr((self.base + offset) as _) };
        (![0x0000, 0xffff].contains(&config.vendor_id)).then_some(config)
    }

    /// Returns `(bus, device, function)` whose classes match the specified one.
    #[cfg(feature = "alloc")]
    pub fn match_class(&self, base: u8, sub: u8, interface: u8) -> Vec<(u8, u8, u8)> {
        self.iter()
            .filter(|cfg| {
                cfg.base_class == base && cfg.sub_class == sub && cfg.interface == interface
            })
            .map(|cfg| {
                let offset = (cfg as *const _) as u64 - self.base;
                (
                    offset.get_bits(16..) as _,
                    offset.get_bits(11..16) as _,
                    offset.get_bits(8..11) as _,
                )
            })
            .collect()
    }

    /// Returns [ConfigSpacesIter] to iterate all configuration spaces.
    pub fn iter(&self) -> ConfigSpacesIter {
        ConfigSpacesIter {
            confs: *self,
            bus: 0,
            dev: 0,
            func: 0,
            done: false,
        }
    }
}

/// Iterator for all PCI configuration spaces.
#[derive(Debug, Clone)]
pub struct ConfigSpacesIter {
    confs: ConfigSpaces,
    bus: u8,
    dev: u8,
    func: u8,
    done: bool,
}

impl Iterator for ConfigSpacesIter {
    type Item = &'static ConfigSpace;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.done {
                return None;
            }

            let config = self.confs.get(self.bus, self.dev, self.func);

            self.func += 1;
            if self.func >= 8 {
                self.func = 0;
                self.dev += 1;
            }
            if self.dev >= 32 {
                self.dev = 0;
                self.bus = match self.bus.checked_add(1) {
                    Some(val) => val,
                    None => {
                        self.done = true;
                        self.bus
                    }
                };
            }

            if let Some(config) = config {
                return Some(config);
            }
        }
    }
}
