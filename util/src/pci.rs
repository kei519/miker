//! Provides PCI supports.

use core::{
    cmp,
    fmt::{Debug, Display},
    marker::PhantomData,
    mem,
};

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

    /// Returns the collection of all capabilities that the configuration space has.
    pub fn raw_capabilities(&mut self) -> RawCapabilities<'_> {
        let cap_ptr = self.cap_ptr().unwrap_or(0);
        RawCapabilities {
            config: self,
            next_ptr: cap_ptr,
        }
    }
}

/// Represents a capability in a configuration space, but its data is raw pointer.
pub struct RawCapability<'a> {
    /// ID of the capability.
    pub cap_id: u8,

    /// Pointer to the next capability.
    ///
    /// If it is zero, it means it has no next capability.
    pub next: u8,

    /// Raw data of the capability whose structure depends of its ID.
    ///
    /// # Safety
    ///
    /// `raw_data` modulo 4 is 2.
    pub raw_data: *mut u8,

    _lifetime: PhantomData<&'a ()>,
}

/// Represents a apability in a configuration space, and it could be handled as non-raw data.
#[derive(Debug)]
pub enum Capability<'a> {
    /// Message Signaled Interrupt capability.
    Msi(MsiCapability<'a>),
    /// Serial ATA Capability.
    Sata(SataCapability<'a>),
    /// Not supported.
    Unknown(RawCapability<'a>),
}

impl<'a> From<RawCapability<'a>> for Capability<'a> {
    fn from(value: RawCapability<'a>) -> Self {
        // WARNING: We don't know whether id depends on a class.
        match value.cap_id {
            0x05 => Self::Msi(MsiCapability(value.raw_data, value._lifetime)),
            0x12 => Self::Sata(SataCapability(value.raw_data, value._lifetime)),
            _ => Self::Unknown(value),
        }
    }
}

/// Reperesents a MSI (Message Signaled Interrupt) capability.
pub struct MsiCapability<'a>(*mut u8, PhantomData<&'a ()>);

impl MsiCapability<'_> {
    fn msg_ctrl(&self) -> u16 {
        // Safety:
        // * valid
        // * properly aligned because self.0 modulo 4 is 2.
        // * initialized by UEFI.
        unsafe { self.0.cast::<u16>().read() }
    }

    fn msg_ctrl_mut(&mut self) -> &mut u16 {
        // self.0 is not null.
        // Safety:
        // * properly aligned because self.0 modulo 4 is 2 i.e. self.0 modulo 2 is 0.
        // * dereferenceable because it's allocated by UEFI.
        // * meet the aliasing rules by PhantomData
        unsafe { self.0.cast::<u16>().as_mut().unwrap() }
    }

    /// Sets whether MSI is enabled.
    pub fn enable(&mut self, enabled: bool) {
        self.msg_ctrl_mut().set_bit(0, enabled);
    }

    /// Returns whether MSI is enabled.
    pub fn is_enabled(&self) -> bool {
        self.msg_ctrl().get_bit(0)
    }

    /// Returns Multiple Message Capable, which determins the number of requested vectors.
    pub fn multi_msg_capable(&self) -> usize {
        // Multiple Message Capable bits represents log base 2 of it.
        1 << self.msg_ctrl().get_bits(1..4) as usize
    }

    /// Returns the current number of allcoated vectors (equal to or less than the number of
    /// requested vectors).
    pub fn multi_msg_enable(&self) -> usize {
        // Same as Multiple Message Capable.
        1 << self.msg_ctrl().get_bits(4..7) as usize
    }

    /// Updates the number of allocated vectors.
    ///
    /// # Note
    ///
    /// Multiple Message Enable bits, which determins the number of allocated vectors accepts only
    /// power of 2 less than or equal to 32. When given `num` does not meet this requirement, we
    /// set the minimum of 32 and the next power of 2 of `num`.
    pub fn set_multi_message_enable(&mut self, num: usize) {
        self.msg_ctrl_mut()
            .set_bits(4..7, cmp::min(num, 32).next_power_of_two().ilog2() as _);
    }

    /// Returns the capability of sending a 64-bit message address.
    pub fn msg_addr_is_64bit(&self) -> bool {
        self.msg_ctrl().get_bit(7)
    }

    /// Returns whether the function supports MSI per-vector masking.
    ///
    /// # Note
    ///
    /// When per-vector masking is enabled, the function is prohibited from sending the associated
    /// message, and must set the associated Pending bit whenever the function would otherwise send
    /// the message to a masked vector.
    pub fn per_vector_macking_capable(&self) -> bool {
        self.msg_ctrl().get_bit(8)
    }

    /// Returns the system-specified message address.
    pub fn msg_addr(&self) -> u64 {
        // Safety:
        // * not null
        // * aligned to multiple of 4 because self.0 modulo 4 is 2.
        // * valid to read because they are set by UEFI.
        unsafe {
            let addr = self.0.add(2).cast::<u32>().read() as u64;
            if self.msg_addr_is_64bit() {
                addr | (self.0.add(6).cast::<u32>().read() as u64) << 32
            } else {
                addr
            }
        }
    }

    /// Sets the system-specified message address.
    pub fn set_msg_addr(&mut self, addr: u64) {
        // Safety: Same as msg_addr and the ownership of it is controlled by PhantomData.
        unsafe {
            self.0.add(2).cast::<u32>().write(addr as _);
            if self.msg_addr_is_64bit() {
                self.0.add(6).cast::<u32>().write(addr.get_bits(32..) as _);
            }
        }
    }

    /// Returns the system-specified message data.
    pub fn msg_data(&self) -> u16 {
        // Safety:
        // * not null
        // * aligned to multiple of 4 because self.0 modulo 4 is 2.
        // * valid to read because they are set by UEFI.
        unsafe {
            *if self.msg_addr_is_64bit() {
                self.0.add(10)
            } else {
                self.0.add(6)
            }
            .cast()
        }
    }

    /// Sets the system-specified message data.
    pub fn set_msg_data(&mut self, data: u16) {
        unsafe {
            *if self.msg_addr_is_64bit() {
                self.0.add(10)
            } else {
                self.0.add(6)
            }
            .cast() = data;
        }
    }

    // TODO: Support Mask Bits and Pending Bits.
}

impl Debug for MsiCapability<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut ret = f.debug_struct("MsiCapability");

        ret.field("msi_enabled", &self.is_enabled())
            .field("multi_msg_capable", &self.multi_msg_capable())
            .field("multi_msg_enable", &self.multi_msg_enable())
            .field("msg_addr_is_64bit", &self.msg_addr_is_64bit())
            .field(
                "per_vector_masking_capable",
                &self.per_vector_macking_capable(),
            )
            .field("msg_addr", &self.msg_addr())
            .field("msg_data", &self.msg_data());

        if self.per_vector_macking_capable() {
            ret.finish_non_exhaustive()
        } else {
            ret.finish()
        }
    }
}

impl Debug for RawCapability<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RawCapability")
            .field("cap_id", &self.cap_id)
            .field("next", &self.next)
            .finish_non_exhaustive()
    }
}

/// Represents a SATA Capability, which is used for the Index-Data Pair mechanism.
//
// TODO: Add a notation to expalin what Index-Data Pair is (, described in SATA spec section
// 10.14).
pub struct SataCapability<'a>(*mut u8, PhantomData<&'a ()>);

impl SataCapability<'_> {
    fn revision(&self) -> u8 {
        // Safety: it is allocated by UEFI.
        unsafe { self.0.read() }
    }

    /// Retruns the major revision of the SATA Capability.
    pub fn major(&self) -> u8 {
        self.revision().get_bits(4..)
    }

    /// Retruns the minor revision of the SATA Capability.
    pub fn minor(&self) -> u8 {
        self.revision().get_bits(..4)
    }

    /// Capability Register 1.
    fn cr1(&self) -> u32 {
        // Safety:
        // * allocated by UEFI
        // * properly aligned because self.0 modulo 4 is 2.
        unsafe { self.0.add(2).cast::<u32>().read() }
    }

    /// Returns index of the bar containing the Index-Data Pair.
    ///
    /// Returns `11` when Index-Data Pair is implemented in Dwords directly in Serial ATA
    /// Capability (following SATACR1).
    pub fn bar_loc(&self) -> u8 {
        // BAR Location in a SATA Capability indicates the absolute PCI Configuration Register
        // address in Dword (4-byte) granularity. Because it is difficult to use directly, we
        // convert it to the index
        (self.cr1().get_bits(..4) - 4) as _
    }

    /// Returns the offset into the BAR where the Index-Data Pair are located.
    pub fn bar_off(&self) -> u64 {
        // In Dword granularity.
        (self.cr1().get_bits(4..24) * 4) as _
    }
}

impl Debug for SataCapability<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SataCapability")
            .field("major", &self.major())
            .field("minor", &self.minor())
            .field("bar_loc", &self.bar_loc())
            .field("bar_off", &self.bar_off())
            .finish()
    }
}

/// Collection of [RawCapability]\(ies).
pub struct RawCapabilities<'a> {
    config: &'a mut ConfigSpace,
    next_ptr: u8,
}

impl<'a> Iterator for RawCapabilities<'a> {
    type Item = RawCapability<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_ptr == 0 {
            return None;
        }

        unsafe {
            let ptr: *mut u8 = (&raw mut *self.config).cast();

            let mut next_ptr = ptr.add(self.next_ptr as usize + 1).read() & !0b11;
            mem::swap(&mut self.next_ptr, &mut next_ptr);
            let next_ptr = next_ptr as usize;

            Some(RawCapability {
                cap_id: ptr.add(next_ptr).read(),
                next: self.next_ptr,
                // Safety: The remainders when `ptr` and `next_ptr` divided by 4, both are 0 by
                //         their constructions, so `ptr.add(next_ptr + 2)` module 4 is 2.
                raw_data: ptr.add(next_ptr + 2),
                _lifetime: PhantomData,
            })
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

    impl<'a> ConfigSpaceStatus<'a> {
        /// Returns the contained [Usable][Usable] value, consuming the `self` value.
        ///
        /// # Panics
        ///
        /// Panics if the `self` value is not [Usable][Usable].
        ///
        /// [Usable]: ConfigSpaceStatus::Usable
        pub fn unwrap(self) -> ConfigSpaceLock<'a> {
            if let Self::Usable(lock) = self {
                lock
            } else {
                panic!("called `ConfigSpaceStatus::unwrap()` on a non-usable value")
            }
        }
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
