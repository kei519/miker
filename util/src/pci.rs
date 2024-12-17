//! Provides PCI supports.

use core::fmt::{Debug, Display};

use crate::{bitfield::BitField, paging::ADDRESS_CONVERTER};

#[cfg(feature = "alloc")]
pub use _alloc::*;

/// Represents BDF (bus, device and function) in PCI configuration spaces.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Bdf(u16);

impl Bdf {
    /// Constructs a new [Bdf].
    pub const fn new(bus: u8, device: u8, function: u8) -> Self {
        Self((bus as u16) << 8 | (device as u16) << 3 | function as u16)
    }

    /// The bus value represented by [Bdf].
    pub fn bus(&self) -> u8 {
        self.0.get_bits(8..) as _
    }

    /// The device value represented by [Bdf].
    pub fn device(&self) -> u8 {
        self.0.get_bits(3..8) as _
    }

    /// The function value represented by [Bdf].
    pub fn function(&self) -> u8 {
        self.0.get_bits(..3) as _
    }
}

impl Debug for Bdf {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Bdf")
            .field("bus", &self.bus())
            .field("device", &self.device())
            .field("function", &self.function())
            .finish()
    }
}

impl Display for Bdf {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}.{}",
            self.bus(),
            self.device(),
            self.function()
        )
    }
}

impl From<u16> for Bdf {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<Bdf> for u16 {
    fn from(value: Bdf) -> Self {
        value.0
    }
}

/// Represents a class of a PCI device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PciClass {
    /// Classifies the type of functionality that the device Function provides.
    pub base_class: u8,
    /// Identifies more specifically the type of functionalyty that the device Function provides.
    pub sub_class: u8,
    /// Specifies register-level interface of a device Function.
    pub interface: u8,
}

/// Represetns a PCI configuration space.
#[repr(C)]
#[derive(Debug)]
pub struct ConfigSpace {
    /// Identifies the manufacture of the device. Either 0 or 0xFFFF is invalid.
    ///
    /// READ-ONLY.
    pub vendor_id: u16,
    /// Identifies the particular device.
    ///
    /// READ-ONLY.
    pub device_id: u16,
    /// Provides coarse control over a device's ability to generate and respond to PCI cycles.
    pub command: u16,
    /// Recorded status information for PCI bus related events.
    status: u16,
    /// Specifies a device specific revision identifier. This field should be viewed as a vendor
    /// defined extension to the `device_id`.
    ///
    /// READ-ONLY.
    pub revision_id: u8,
    /// Specifies register-level interface of a device Function.
    ///
    /// READ-ONLY.
    pub interface: u8,
    /// Identifies more specifically the type of functionalyty that the device Function provides.
    ///
    /// READ-ONLY.
    pub sub_class: u8,
    /// Classifies the type of functionality that the device Function provides.
    ///
    /// READ-ONLY.
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
    ///
    /// READ-ONLY.
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
    ///
    /// READ-ONLY.
    pub subsystem_vendor_id: u16,
    /// Subsystem ID.
    ///
    /// READ-ONLY.
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
    #[cfg(not(feature = "alloc"))]
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

    /// Returns the whole class of the [ConfigSpace].
    pub fn class(&self) -> PciClass {
        PciClass {
            base_class: self.base_class,
            sub_class: self.sub_class,
            interface: self.interface,
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

#[cfg(feature = "alloc")]
mod _alloc {
    use alloc::collections::BTreeMap;
    use core::{
        mem,
        ops::{Deref, DerefMut},
    };

    use crate::sync::InterruptFreeMutex;

    use super::*;

    /// Provides access to multi-core safe PCI configuration spaces.
    pub struct ConfigSpaces {
        base: *mut ConfigSpace,
        used_map: InterruptFreeMutex<BTreeMap<u16, bool>>,
    }

    // Safety: ConfigSpaces.used_map is Send and Sync, so we have to consider whether
    //         ConfigSpaces.base is Send and Sync. It is accessed via the ConfigSpaces and its
    //         exclusiveness is controlled by ConfigSpaces.used_map that is Send and Sync.
    unsafe impl Send for ConfigSpaces {}
    unsafe impl Sync for ConfigSpaces {}

    impl ConfigSpaces {
        /// Constructs new [CfgSpaces] from the physical address to the head of [ConfigSpace],
        /// `phys_addr`.
        ///
        /// # Panic
        ///
        /// If there is no map from physical address `phys_addr` to virtual one, it causes panic.
        ///
        /// # Safety
        ///
        /// `phys_addr` must be non-null and properly aligned.
        pub unsafe fn from_ptr(phys_addr: *mut u8) -> Self {
            Self {
                base: ADDRESS_CONVERTER
                    .as_ref()
                    .get_ptr(phys_addr as _)
                    .unwrap()
                    .as_ptr(),
                used_map: InterruptFreeMutex::new(BTreeMap::new()),
            }
        }

        /// Returns exclusive access to the PCI configuration space specified by `bus`, `dev` and
        /// `func`, if a device is connected to the specified BDF and noone owns it.
        pub fn get_config_space(&self, bdf: Bdf) -> ConfigSpaceStatus<'_> {
            let offset = bdf.into();

            let mut used_map = self.used_map.lock();
            match used_map.get_mut(&offset) {
                Some(true) => ConfigSpaceStatus::Used,
                Some(false) => {
                    used_map.insert(offset, true);
                    ConfigSpaceStatus::Usable(ConfigSpaceLock {
                        used_map: &self.used_map,
                        offset,
                        config: unsafe { &mut *(self.base.add(offset as _)) },
                    })
                }
                None => {
                    if pci_is_enabled(unsafe {
                        self.base
                            .byte_add((offset as usize) << 8)
                            .cast::<u16>()
                            .read()
                    }) {
                        used_map.insert(offset, true);
                        ConfigSpaceStatus::Usable(ConfigSpaceLock {
                            used_map: &self.used_map,
                            offset,
                            config: unsafe { &mut *self.base.add(offset as _) },
                        })
                    } else {
                        ConfigSpaceStatus::NotConnected
                    }
                }
            }
        }

        /// Returns the collection of the PCI configuration spaces.
        pub fn valid_bfds_and_classes(&self) -> BfdsAndClasses {
            BfdsAndClasses {
                configs: self,
                bus: 0,
                dev: 0,
                func: 0,
                done: false,
            }
        }
    }

    /// Represents the status of a PCI configuration space.
    pub enum ConfigSpaceStatus<'a> {
        /// You have access to the configuration space because noone is using it.
        Usable(ConfigSpaceLock<'a>),
        /// You cannot take the ownership of the configuration space because someone is using.
        Used,
        /// No PCI device is connected to the specified bus, device and fucntion.
        NotConnected,
    }

    /// Provides exclusive access to a PCI space configuration.
    pub struct ConfigSpaceLock<'a> {
        used_map: &'a InterruptFreeMutex<BTreeMap<u16, bool>>,
        offset: u16,
        config: &'a mut ConfigSpace,
    }

    impl Deref for ConfigSpaceLock<'_> {
        type Target = ConfigSpace;

        fn deref(&self) -> &Self::Target {
            self.config
        }
    }

    impl DerefMut for ConfigSpaceLock<'_> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.config
        }
    }

    impl Drop for ConfigSpaceLock<'_> {
        fn drop(&mut self) {
            *self.used_map.lock().get_mut(&self.offset).unwrap() = false;
        }
    }

    /// Collects up all valid PCI configuration spaces.
    pub struct BfdsAndClasses<'a> {
        configs: &'a ConfigSpaces,
        bus: u8,
        dev: u8,
        func: u8,
        done: bool,
    }

    impl Iterator for BfdsAndClasses<'_> {
        type Item = (PciClass, Bdf);

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                if self.done {
                    return None;
                }

                let offset: u16 = Bdf::new(self.bus, self.dev, self.func).into();
                // Safety:
                // - max(offset) = u16::MAX so max(offset * size_of::<ConfigSpace>()) < isize::MAX.
                // - `ConfigSpaces` is the header of the collection of `ConfigSpace`.
                let ptr = unsafe { self.configs.base.add(offset as _) }.cast::<u8>();

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

                // Safety:
                // - `ptr` must be valid.
                //  - `ptr` is not null because of its construction.
                //  - `ptr` is dereferenceable because its space is allocated by UEFI.
                //  - All accesses to `ptr` aer atomic because Vendor ID in a PCI configuration space
                //    is read-only.
                //  - Casting result is valid because its value is copied (and it has static lifetime).
                //
                // - `ptr` must be properly aligned.
                //  `ptr` is 4-byte aligned because of ist construction.
                //
                // - `ptr` must point to a properly initialized value.
                //  any value is valid for `u16`.
                if pci_is_enabled(unsafe { ptr.cast::<u16>().read() }) {
                    // Safety: same as above.
                    return Some((
                        unsafe {
                            PciClass {
                                base_class: ptr
                                    .byte_add(mem::offset_of!(ConfigSpace, base_class))
                                    .read(),
                                sub_class: ptr
                                    .byte_add(mem::offset_of!(ConfigSpace, sub_class))
                                    .read(),
                                interface: ptr
                                    .byte_add(mem::offset_of!(ConfigSpace, interface))
                                    .read(),
                            }
                        },
                        offset.into(),
                    ));
                }
            }
        }
    }

    fn pci_is_enabled(vendor_id: u16) -> bool {
        ![0, 0xffff].contains(&vendor_id)
    }
}
