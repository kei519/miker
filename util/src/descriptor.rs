//! Provides descriptors and descriptor tables.
// TODO: Add all descriptors and tables in 64-bit mode.

use core::mem;

use crate::bitfield::BitField as _;

#[cfg(feature = "alloc")]
pub use self::alloc::*;

#[cfg(feature = "alloc")]
mod alloc {
    use ::alloc::boxed::Box;
    use ::alloc::{format, vec};

    use super::*;
    use crate::error;
    use crate::{asmfunc, error::Result};

    /// Represents a global descriptor table.
    #[derive(Debug, Clone)]
    pub struct GDT {
        descs: Box<[u64]>,
    }

    impl GDT {
        /// Constructs a new [GDT] with `len` size.
        ///
        /// # Panic
        ///
        /// If `len` is `0` or exceeds `8192`, this function panics because of the x64 GDT
        /// limitation.
        pub fn new(len: usize) -> Self {
            if len == 0 || len * 8 > u16::MAX as usize + 1 {
                panic!(
                    "size of GDT cannot exceed {} bytes: {} bytes specified",
                    u16::MAX as usize + 1,
                    len * 8,
                )
            }
            Self {
                descs: vec![0; len].into_boxed_slice(),
            }
        }

        /// Set `descriptor` as the `index`-th descriptor of [GDT]. If `index` is no less than
        /// `len` specified by [`Self::new()`], this does nothing.
        pub fn set(&mut self, index: usize, descriptor: impl Descriptor) -> Result<()> {
            if index == 0 {
                error!("do not overwrite GDT[0]");
            }
            // Check whether the `descriptor` overflows the length of `GDT`.
            if index + descriptor.required_size() > self.descs.len() {
                error!(format!(
                    "descriptor size {} will overflow GDT[{}..] because the length of GDT is {}",
                    descriptor.required_size(),
                    index,
                    self.descs.len(),
                ));
            }
            descriptor.write_to_table(&mut self.descs[index..]);
            Ok(())
        }

        /// Call `LGDT` (Load GDT) instruction to register [`GDT`] to a processor.
        pub fn register(&self) {
            asmfunc::load_gdt(self.descs.as_ptr() as _, self.descs.len() * 8)
        }
    }

    /// Represents a Interrupt Descriptor Table.
    pub struct IDT {
        /// Descriptors.
        descs: [SystemDescriptor; 256],
    }

    impl IDT {
        /// Constructs a new empty [`IDT`].
        pub const fn new() -> Self {
            Self {
                descs: [SystemDescriptor::null(); 256],
            }
        }

        /// Set `descriptor` as the `index`-th descriptor of [`IDT`]. If `index` is greater than
        /// `255` or the type of `descriptor` is not [`SystemDescriptorType::InterruptGate`],
        /// do nothing and returns an error.
        pub fn set(&mut self, index: usize, descriptor: SystemDescriptor) -> Result<()> {
            if index >= self.descs.len() {
                error!("index out of range");
            } else if descriptor.ty() != SystemDescriptorType::InterruptGate {
                error!("only interrupt gate descriptor can be set to IDT");
            }
            self.descs[index] = descriptor;
            Ok(())
        }

        /// Call `LIDT` (Load IDT) instruction to register [`IDT`] to a processor.
        pub fn register(&self) {
            asmfunc::load_idt(self as *const _ as _, mem::size_of::<Self>());
        }
    }
}

/// Provides methods to register descriptors to descriptor tables.
pub trait Descriptor {
    /// Returns required size to register, in 8-bytes.
    fn required_size(&self) -> usize;

    /// Registers the descriptor to the head of `table`.
    fn write_to_table(&self, table: &mut [u64]);
}

/// Represents a segment type. When 4th bit (bit 3) is set, this is code segment type. Otherwise,
/// data segment.
///
/// The accessed bit indicates whether the segment has been accessed since the last time the bit
/// cleared.
///
/// If expand down bit is set, dynamically changing the segment limit causes segment space to
/// added to the bottom of the segment. If the limit will never change, expand down causes
/// nothing.
///
/// A transfer of execution into a more-previleged conforming segment allows execution to continue
/// at the current privilege level. A transfer into a nonconforming segment at a different
/// privilege level results in a #GP.
///
/// All data segments are nonconroming, meaning that they cannot be accessed by less privileged
/// level. Unlike code segments, however, they can be accessed by more privileged level.
///
/// | Bit | Data Segment | Code Segment |
/// | :---: | :--- | :--- |
/// | 0 | Accessed | Accessed |
/// | 1 | Writable | Readable |
/// | 2 | Expand down | Conforming |
/// | 3 | 0 | 1 |
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentType(u8);

impl SegmentType {
    /// Constructs a new data segment type.
    pub fn data(writable: bool, expand_down: bool) -> Self {
        let mut val = 0;
        val.set_bit(1, writable);
        val.set_bit(2, expand_down);
        Self(val)
    }

    /// Constructs a new code segment type.
    pub fn code(readable: bool, conforming: bool) -> Self {
        let mut val = 0;
        val.set_bit(1, readable);
        val.set_bit(2, conforming);
        val.set_bit(3, true);
        Self(val)
    }

    /// Get accessed bit.
    pub fn accessed(&self) -> bool {
        self.0.get_bit(0)
    }

    /// Returns whether the type is code segment type.
    pub fn is_code(&self) -> bool {
        self.0.get_bit(3)
    }

    /// Returns whether the type is data segment type.
    pub fn is_data(&self) -> bool {
        !self.is_code()
    }
}

/// Segment descriptor that represents a segment selected by segment-selector register.
///
/// In 16-bit or 32-bit mode, an address \[S:REG\] is interpreted as
///
/// ```
/// let limit = if G { S.limit << 12 } else { S.limit };
/// let addr = S.base + REG;
/// if addr <= limit { addr } else { #GP }
/// ```
///
/// `Base` and `Limit` are ignored in 64-bit mode.
///
/// | Bits | abbr. | Description |
/// | :---: | :---: | :--- |
/// | 15:0 | | `Limit[15:0]` |
/// | 39:16 | | `Base[23:0]` |
/// | 43:40 | | Type (see [`SegmentType`]) |
/// | 44 | S | Descriptor type (1 = code or data) |
/// | 46:46 | DPL | Desriptor privilege level |
/// | 47 | P | Segment present |
/// | 51:48 | | `Limit[19:16]` |
/// | 52 | AVL | Available for use by system software |
/// | 53 | L | 64-bit code segment. If this is set, 54 bit must be 0. |
/// | 54 | D/B | Default / Big flag (0 - 16-bit, 1 - 32-bit mode). |
/// | 55 | G | Granalarity (if set, `limit` is interpreted in 4-KB units). |
/// | 63:56 | | `Base[31:24]` |
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SegmentDescriptor {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    etc0: u8,
    etc1: u8,
    base_hi: u8,
}

impl SegmentDescriptor {
    /// Constructs a new 64-bit segment descriptor. Whether the segment is data or code one is
    /// determined by `ty`.
    pub fn new(ty: SegmentType, privilege_level: u8) -> Self {
        let mut etc0 = 0;
        etc0.set_bits(..4, ty.0);
        // Code or data descriptor
        etc0.set_bit(4, true);
        etc0.set_bits(5..7, privilege_level);
        // Present
        etc0.set_bit(7, true);

        let mut etc1 = 0;
        // limit[19:16]
        // This field is not used in long mode.
        etc1.set_bits(..4, 0);

        if ty.is_code() {
            // 64-bit code segment
            etc1.set_bit(5, true);
        }

        Self {
            limit_low: 0,
            base_low: 0,
            base_mid: 0,
            etc0,
            etc1,
            base_hi: 0,
        }
    }
}

impl Descriptor for SegmentDescriptor {
    fn required_size(&self) -> usize {
        1
    }

    fn write_to_table(&self, table: &mut [u64]) {
        table[0].set_bits(..16, self.limit_low as _);
        table[0].set_bits(16..32, self.base_low as _);
        table[0].set_bits(32..40, self.base_mid as _);
        table[0].set_bits(40..48, self.etc0 as _);
        table[0].set_bits(48..56, self.etc1 as _);
        table[0].set_bits(56.., self.base_hi as _);
    }
}

/// Represents a long-mode sysetm descriptor type.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemDescriptorType {
    /// Reserved types.
    Reserved = 0,
    /// Local Descriptor-table segment descriptor.
    LDT = 0b0010,
    /// Task-state segment (TSS) descriptor.
    AvailableTSS = 0b1001,
    /// TSS (busy).
    BusyTSS = 0b1011,
    /// Call-gate descriptor.
    CallGate = 0b1100,
    /// Interrupt-gate descriptor.
    InterruptGate = 0b1110,
    /// Trap-gate descriptor.
    TrapGate = 0b1111,
}

impl From<SystemDescriptorType> for u8 {
    fn from(value: SystemDescriptorType) -> Self {
        value as _
    }
}

impl From<u8> for SystemDescriptorType {
    fn from(value: u8) -> Self {
        match value {
            0b0001 => Self::LDT,
            0b1001 => Self::AvailableTSS,
            0b1011 => Self::BusyTSS,
            0b1100 => Self::CallGate,
            0b1110 => Self::InterruptGate,
            0b1111 => Self::TrapGate,
            _ => Self::Reserved,
        }
    }
}

/// Rperesents a 64-bit system descriptor. See [SystemDescriptorType] for types information.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SystemDescriptor {
    field0: u32,
    field1: u32,
    field2: u32,
    field3: u32,
}

impl SystemDescriptor {
    /// Constructs a new null [`SystemDescriptor`], whose fields are all zero.
    pub const fn null() -> Self {
        Self {
            field0: 0,
            field1: 0,
            field2: 0,
            field3: 0,
        }
    }

    /// Constructs a new interrupt descriptor. Interrupts run after setting RSP to an address
    /// specified by [`TSS::ists`] when `stack_table` is not `0`. Otherwise, RSP does not change.
    pub fn new_interrupt(
        handler: *const unsafe extern "sysv64" fn(),
        segment: u16,
        stack_table: u8,
        privilege_level: u8,
    ) -> Self {
        let addr = handler as u64;

        let mut field0 = 0;
        field0.set_bits(..16, addr as _);
        field0.set_bits(16.., segment as _);

        let mut field1 = 0;
        field1.set_bits(..3, stack_table as _);
        field1.set_bits(8..12, SystemDescriptorType::InterruptGate as _);
        field1.set_bits(13..15, privilege_level as _);
        // present
        field1.set_bit(15, true);
        field1.set_bits(16.., addr.get_bits(16..32) as _);

        Self {
            field0,
            field1,
            field2: addr.get_bits(32..) as _,
            field3: 0,
        }
    }

    /// Constructs a new TSS descriptor from `tss`, a reference to a [TSS].
    pub fn new_tss(tss: &'static TSS, privilege_level: u8) -> Self {
        let base = tss as *const _ as u64;
        let limit = mem::size_of_val(tss) - 1;

        let mut field0 = 0;
        field0.set_bits(..16, limit.get_bits(..16) as _);
        field0.set_bits(16.., base.get_bits(..16) as _);

        let mut field1 = 0;
        field1.set_bits(..8, base.get_bits(16..24) as _);
        field1.set_bits(8..12, SystemDescriptorType::AvailableTSS as _);
        field1.set_bits(13..15, privilege_level as _);
        // Present
        field1.set_bit(15, true);
        field1.set_bits(24.., base.get_bits(24..) as _);

        let field2 = base.get_bits(32..) as _;

        Self {
            field0,
            field1,
            field2,
            field3: 0,
        }
    }

    /// Returs the type of [`SystemDescriptor`].
    pub fn ty(&self) -> SystemDescriptorType {
        let ty = self.field1.get_bits(8..12) as u8;
        ty.into()
    }
}

impl Descriptor for SystemDescriptor {
    fn required_size(&self) -> usize {
        2
    }

    fn write_to_table(&self, table: &mut [u64]) {
        table[0].set_bits(..32, self.field0 as _);
        table[0].set_bits(32.., self.field1 as _);
        table[1].set_bits(..32, self.field2 as _);
        table[1].set_bits(32.., self.field3 as _);
    }
}

/// Represents TSS (Task-State Segment) holding each stack pointer for all privilege level and
/// interrupt stack table pointer used on interrupts.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TSS {
    reserved0: u32,
    rsp0_low: u32,
    rsp0_high: u32,
    rsp1_low: u32,
    rsp1_high: u32,
    rsp2_low: u32,
    rsp2_high: u32,
    reserved1: u32,
    reserved2: u32,
    ists: [u32; 2 * 7],
    reserved3: u32,
    reserved4: u32,
    reserved5: u16,
    io_map_base: u16,
}

impl TSS {
    /// Constructs a new TSS with `rsps`, , and `ists`, a slice of
    ///
    /// # Arguments
    ///
    /// * `rsps` - A slice of \[rsp0, rsp1, rsp2\] that does not have to contains all. `rspN` is
    ///            RSP for privilege level `N`.
    /// * `ists` - A slice of \[ists1, ist2, ..., ist7\] that does not have to contains all. `istN`
    ///            is the interrupt stack table pointers used in a interrupt-gate descriptor.
    pub fn new(rsps: &[u64], ists: &[u64]) -> Self {
        let rsp0 = rsps.first().copied().unwrap_or(0);
        let rsp1 = rsps.get(1).copied().unwrap_or(0);
        let rsp2 = rsps.get(2).copied().unwrap_or(0);

        let mut self_ists = [0; 2 * 7];
        for (i, ist) in ists.iter().take(7).copied().enumerate() {
            self_ists[2 * i] = ist.get_bits(..32) as _;
            self_ists[2 * i + 1] = ist.get_bits(32..) as _;
        }

        Self {
            reserved0: 0,
            rsp0_low: rsp0.get_bits(..32) as _,
            rsp0_high: rsp0.get_bits(32..) as _,
            rsp1_low: rsp1.get_bits(..32) as _,
            rsp1_high: rsp1.get_bits(32..) as _,
            rsp2_low: rsp2.get_bits(..32) as _,
            rsp2_high: rsp2.get_bits(32..) as _,
            reserved1: 0,
            reserved2: 0,
            ists: self_ists,
            reserved3: 0,
            reserved4: 0,
            reserved5: 0,
            io_map_base: 0,
        }
    }
}
