use core::{
    fmt::{Debug, Display},
    mem, ptr, slice,
};

use crate::bitfield::BitField;

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
    interrupt_controller: [u8],
}

impl Madt {
    pub fn controllers(&self) -> InterruptControllerIter {
        let offset = mem::offset_of!(Self, flags) + mem::size_of::<u32>();
        let len = self.header.len as usize - offset;

        let start = &self.interrupt_controller[0] as _;
        InterruptControllerIter {
            cur: start,
            end: unsafe { start.add(len) },
        }
    }

    pub(super) fn from_header(header: &'static TableHeader) -> &'static Self {
        let fat_ptr = ptr::slice_from_raw_parts(
            header as *const _ as *const u8,
            header.entries_len() / mem::size_of::<u8>(),
        );
        unsafe { &*(fat_ptr as *const _) }
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

pub struct InterruptControllerIter {
    cur: *const u8,
    end: *const u8,
}

impl Iterator for InterruptControllerIter {
    type Item = InterruptController;

    fn next(&mut self) -> Option<Self::Item> {
        if unsafe { self.end.offset_from(self.cur) } <= 0 {
            return None;
        }

        let (ret, next) = InterruptController::from_ptr(self.cur);
        self.cur = next;
        Some(ret)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum InterruptController {
    LocalApic(&'static LocalApic),
    IoApic(&'static IoApic),
    Unsupported(UnsupportedInterruptController),
}

impl InterruptController {
    pub fn as_ptr(&self) -> *const u8 {
        match self {
            Self::LocalApic(apic) => (*apic as *const LocalApic).cast(),
            Self::IoApic(apic) => (*apic as *const IoApic).cast(),
            Self::Unsupported(cont) => unsafe { cont.data.as_ptr().cast::<u8>().byte_sub(2) },
        }
    }

    fn from_ptr(ptr: *const u8) -> (Self, *const u8) {
        match unsafe { *ptr } {
            ty if ty == InterruptControllerType::LOCAL_APIC => {
                let res = LocalApic::from_ptr(ptr);
                (Self::LocalApic(res.0), res.1)
            }
            ty if ty == InterruptControllerType::IO_APIC => {
                let res = IoApic::from_ptr(ptr);
                (Self::IoApic(res.0), res.1)
            }
            _ => {
                let res = UnsupportedInterruptController::from_ptr(ptr);
                (Self::Unsupported(res.0), res.1)
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InterruptControllerType(pub u8);

impl InterruptControllerType {
    pub const LOCAL_APIC: Self = Self(0);
    pub const IO_APIC: Self = Self(1);
}

impl PartialEq<u8> for InterruptControllerType {
    fn eq(&self, other: &u8) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<InterruptControllerType> for u8 {
    fn eq(&self, other: &InterruptControllerType) -> bool {
        self.eq(&other.0)
    }
}

impl Display for InterruptControllerType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let desc = match self.0 {
            0 => "Processor Local APIC",
            1 => "I/O APIC",
            2 => "Interrupt Source Override",
            3 => "Non-maskable Interrupt (NMI) Source",
            4 => "Local APIC NMI",
            5 => "Local APIC Address Override",
            6 => "I/O SAPIC",
            7 => "Local SAPIC",
            8 => "Platform Interrupt Sources",
            9 => "Processor Local x2APIC",
            0xA => "Local x2APIC NMI",
            0xB => "GIC CPU Interface (GICC)",
            0xC => "GIC Distributor (GICD)",
            0xD => "GIC MSI Frame",
            0xE => "GIC Redistributor (GICR)",
            0xF => "GIC Interrupt Translation Service (ITS)",
            0x10 => "Multiprocessor Wakeup",
            0x11 => "Core Programmable Interrupt Controller (CORE PIC)",
            0x12 => "Legacy I/O Programmable Interrupt Controller (LIO PIC)",
            0x13 => "HyperTransport Programmable Interrupt Controller (HT PIC)",
            0x14 => "Extend I/O Programmable Interrupt Controller (EIO PIC)",
            0x15 => "MSI Programmable Interrupt Controller (MSI PIC)",
            0x16 => "Bridge I/O Programmable Interrupt Controller (BIO PIC)",
            0x17 => "Low Pin Count Programmable Interrupt Controller (LPC PIC)",
            0x18..=0x7F => "Reserved. OSPM skips structures of the reserved type.",
            0x80..=0xFF => "Reserved for OEM use",
        };
        f.write_str(desc)
    }
}

impl Debug for InterruptControllerType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Display::fmt(self, f)
    }
}

#[repr(C, packed)]
pub struct LocalApic {
    pub ty: InterruptControllerType,
    /// 8.
    length: u8,
    /// The OS associates this Local APIC Structure with a processor object in the namespace when
    /// the _UID child object of the processor’s device object evaluates to a numeric value that
    /// matches the numeric value in this field.
    pub acpi_processor_uid: u8,
    /// The processor's local APIC ID.
    pub apic_id: u8,
    pub flags: u32,
}

impl LocalApic {
    /// If `true` the processor is ready for use. If `false` and the
    /// [`online_capable()`](Self::online_capable) is `true`, system hardware supports enabling
    /// this processor during OS runtime. If both are `false`, this processor is unusable.
    pub fn enable(&self) -> bool {
        let flags = self.flags;
        flags.get_bit(0)
    }

    /// This information depends on the [`enable()`](Self::enable) value. Read
    /// [`enable()`](Self::enable).
    pub fn online_capable(&self) -> bool {
        let flags = self.flags;
        flags.get_bit(1)
    }

    fn from_ptr(ptr: *const u8) -> (&'static Self, *const u8) {
        unsafe { (&*ptr.cast(), ptr.add(mem::size_of::<Self>())) }
    }
}

impl Debug for LocalApic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LocalApic")
            .field("ty", &self.ty)
            .field("length", &self.length)
            .field("acpi_processor_uid", &self.acpi_processor_uid)
            .field("apic_id", &self.apic_id)
            .field("enable", &self.enable())
            .field("online_capable", &self.online_capable())
            .finish()
    }
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct IoApic {
    pub ty: InterruptControllerType,
    /// 12.
    length: u8,
    /// The I/O APIC's ID.
    pub io_apic_id: u8,
    /// 0.
    pub reserved: u8,
    /// The 32-bit physical address to access this I/O APIC.
    pub io_apic_address: u32,
    /// The global system interrupt number where this I/O APIC's interrupt inputs start. The number
    /// of interrupt inputs is determined by the I/O APIC's Max Redir Entry register.
    pub global_system_interrupt_base: u32,
}

impl IoApic {
    fn from_ptr(ptr: *const u8) -> (&'static Self, *const u8) {
        unsafe { (&*(ptr.cast()), ptr.add(mem::size_of::<Self>())) }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UnsupportedInterruptController {
    pub ty: InterruptControllerType,
    #[allow(unused)]
    data: &'static [u8],
}

impl UnsupportedInterruptController {
    /// Returns [UnsupportedInterruptController] and the pointer to the next field.
    fn from_ptr(ptr: *const u8) -> (Self, *const u8) {
        let ty = InterruptControllerType(unsafe { *ptr });
        // Safety: ptr is aligned to 1, so it is ok.
        let len = unsafe { *ptr.add(1) } as _;
        // Since the length is the length including type and length field, we sub two.
        let data = unsafe { slice::from_raw_parts(ptr.add(2), len - 2) };

        (Self { ty, data }, unsafe { ptr.add(len) })
    }
}
