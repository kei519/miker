use core::{mem, num::NonZeroU64, ptr};

use bitflags::bitflags;

pub struct Elf64<'a> {
    data: &'a [u8],
}

impl<'a> Elf64<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, ParseError> {
        Elf64Header::check_ident(data)?;
        if data.len() < mem::size_of::<Elf64Header>() {
            return Err(ParseError::TooSmall);
        }
        // SAFETY:
        // * data is valid because of followings:
        //   * The pointer is derived from the reference to the slice.
        //   * The allocated size is sufficient.
        // * All value is valid for.
        let header = unsafe { ptr::read_unaligned(data.as_ptr().cast::<Elf64Header>()) };
        if header.entry_header_size as usize != mem::size_of::<Elf64Header>() {
            return Err(ParseError::InvalidHeaderSize);
        }
        if header.ident[Elf64Header::VERSION_INDEX] as u32 != header.version {
            return Err(ParseError::VersionMismatch);
        }

        // TODO: Check all header is contained in data.
        Ok(Self { data })
    }

    pub fn os_abi(&self) -> OsAbi {
        self.read_program_header().os_abi()
    }

    pub fn abi_version(&self) -> AbiVersion {
        self.read_program_header().abi_version()
    }

    pub fn ty(&self) -> Type {
        self.read_program_header().ty
    }

    pub fn machine(&self) -> Machine {
        self.read_program_header().machine
    }

    pub fn version(&self) -> Version {
        Version(self.read_program_header().version as u8)
    }

    pub fn entry(&self) -> Option<NonZeroU64> {
        NonZeroU64::new(self.read_program_header().entry)
    }

    pub fn flags(&self) -> MachineFlags {
        self.read_program_header().flags
    }

    fn read_program_header(&self) -> Elf64Header {
        // SAFETY:
        // * data is valid because of followings:
        //   * The pointer is derived from the reference to the slice.
        //   * The allocated size is sufficient.
        // * All value is valid for.
        unsafe { ptr::read_unaligned(self.data.as_ptr().cast()) }
    }
}

impl<'a> core::fmt::Debug for Elf64<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Elf64")
            .field("os_abi", &self.os_abi())
            .field("abi_version", &self.abi_version())
            .field("ty", &self.ty())
            .field("machine", &self.machine())
            .field("version", &self.version())
            .field("entry", &self.entry())
            .field("flags", &self.flags())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParseError {
    TooSmall,
    InvalidMagic,
    InvalidHeaderSize,
    PaddingNotZero,
    VersionMismatch,
    InvalidClass(Class),
    UnsupportedEndian(Endian),
    UnsupportedVersion(Version),
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Class(u8);

impl Class {
    pub const NONE: Self = Self(0);
    pub const ELF32: Self = Self(1);
    pub const ELF64: Self = Self(2);

    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl core::fmt::Debug for Class {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NONE => f.write_str("Class::NONE"),
            Self::ELF32 => f.write_str("Class::ELF32"),
            Self::ELF64 => f.write_str("Class::ELF64"),
            class => f.debug_tuple("Class::Unknown").field(&class.0).finish(),
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Endian(u8);

impl Endian {
    pub const NONE: Self = Self(0);
    pub const LSB: Self = Self(1);
    pub const MSB: Self = Self(2);

    pub const fn as_u8(self) -> u8 {
        self.0
    }

    pub const fn native() -> Self {
        cfg_select! {
            target_endian = "little" => {
                Self::LSB
            }
            target_endian = "big" => {
                Self::MSB
            }
            _ => {
                Self::None
            }
        }
    }
}

impl core::fmt::Debug for Endian {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NONE => f.write_str("Endian::NONE"),
            Self::LSB => f.write_str("Endian::LSB"),
            Self::MSB => f.write_str("Endian::MSB"),
            endian => f.debug_tuple("Endian::Unknown").field(&endian.0).finish(),
        }
    }
}

#[repr(transparent)]
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OsAbi(u8);

impl OsAbi {
    pub const NONE: Self = Self(0);
    pub const GNU: Self = Self(3);
    pub const LINUX: Self = Self(4);

    pub const fn as_u8(self) -> u8 {
        self.0
    }

    pub const fn is_arch_specific(self) -> bool {
        64 <= self.0
    }
}

impl core::fmt::Debug for OsAbi {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NONE => f.write_str("OsAbi::NONE"),
            Self::GNU => f.write_str("OsAbi::GNU"),
            Self::LINUX => f.write_str("OsAbi::LINUX"),
            abi => f.debug_tuple("OsAbi::Unknown").field(&abi.0).finish(),
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbiVersion(pub u8);

#[repr(transparent)]
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Type(u16);

impl Type {
    /// No file type.
    pub const NONE: Self = Self(0);
    /// Relocation file.
    pub const REL: Self = Self(1);
    /// Executable file.
    pub const EXEC: Self = Self(2);
    /// Shared object file.
    pub const DYN: Self = Self(3);
    /// Core file.
    pub const CORE: Self = Self(4);

    /// Operating system-specific
    const LOOS: Self = Self(0xfe00);
    /// Operating system-specific
    const HIOS: Self = Self(0xfeff);
    /// Processor-specific.
    const LOPROC: Self = Self(0xff00);
    /// Processor-specific.
    const HIPROC: Self = Self(0xffff);

    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// Whether this is an operating system-specific type.
    pub const fn is_os_specific(self) -> bool {
        Self::LOOS.0 <= self.0 && self.0 <= Self::HIOS.0
    }

    /// Whether this is a process system-specific type.
    pub const fn is_processor_specific(self) -> bool {
        #[allow(clippy::absurd_extreme_comparisons)]
        {
            Self::LOPROC.0 <= self.0 && self.0 <= Self::HIPROC.0
        }
    }
}

impl core::fmt::Debug for Type {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NONE => f.write_str("Type::NONE"),
            Self::REL => f.write_str("Type::REL"),
            Self::EXEC => f.write_str("Type::EXEC"),
            Self::DYN => f.write_str("Type::DYN"),
            Self::CORE => f.write_str("Type::CORE"),
            ty if ty.is_os_specific() => f.debug_tuple("Type::OsSpecific").field(&ty.0).finish(),
            ty if ty.is_processor_specific() => f
                .debug_tuple("Type::ProcessorSpecific")
                .field(&ty.0)
                .finish(),
            ty => f.debug_tuple("Type::Unknown").field(&ty.0).finish(),
        }
    }
}

#[repr(transparent)]
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Machine(u16);

impl Machine {
    /// No machine.
    pub const NONE: Self = Self(0);
    /// Intel 80386
    pub const I386: Self = Self(3);
    /// ARM 32-bit architecture.
    pub const ARM: Self = Self(40);
    /// AMD x86-64 architecture.
    pub const X86_64: Self = Self(62);
    /// ARM 64-bit architecture.
    pub const AARCH64: Self = Self(183);
    /// RISC-V
    pub const RISCV: Self = Self(243);

    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// Whether this is a reserved architecture.
    pub const fn is_reserved(self) -> bool {
        (11 <= self.0 && self.0 <= 14)
            || self.0 == 16
            || (24 <= self.0 && self.0 <= 35)
            || (121 <= self.0 && self.0 <= 130)
            || (145 <= self.0 && self.0 <= 159)
            || self.0 == 182
            || self.0 == 184
            || (225 <= self.0 && self.0 <= 242)
    }
}

impl core::fmt::Debug for Machine {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NONE => f.write_str("Machine::NONE"),
            Self::I386 => f.write_str("Machine::I386"),
            Self::ARM => f.write_str("Machine::ARM"),
            Self::X86_64 => f.write_str("Machine::X86_64"),
            Self::AARCH64 => f.write_str("Machine::AARCH64"),
            Self::RISCV => f.write_str("Machine::RISCV"),
            machine if machine.is_reserved() => f
                .debug_tuple("Machine::Reserved")
                .field(&machine.0)
                .finish(),
            machine => f.debug_tuple("Machine::Unknown").field(&machine.0).finish(),
        }
    }
}

#[repr(transparent)]
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(u8);

impl Version {
    pub const NONE: Self = Self(0);
    pub const CURRENT: Self = Self(1);

    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

impl core::fmt::Debug for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NONE => f.write_str("Version::NONE"),
            Self::CURRENT => f.write_str("Version::CURRENT"),
            version => f
                .debug_tuple("Version::Unsupported")
                .field(&version.0)
                .finish(),
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MachineFlags(pub u32);

#[repr(C)]
struct Elf64Header {
    ident: [u8; Self::INDENT_SIZE],
    ty: Type,
    machine: Machine,
    version: u32,
    entry: u64,
    program_offset: u64,
    section_offset: u64,
    flags: MachineFlags,
    entry_header_size: u16,
    program_header_entry_size: u16,
    program_header_number: u16,
    section_header_entry_size: u16,
    section_header_number: u16,
    section_header_string_index: u16,
}
const _: () = {
    use core::mem::size_of;
    ["Elf64Header size"][size_of::<Elf64Header>() - 64];
};

impl Elf64Header {
    const INDENT_SIZE: usize = 16;
    const MAGIC: [u8; 4] = *b"\x7fELF";
    const CLASS_INDEX: usize = 4;
    const DATA_INDEX: usize = 5;
    const VERSION_INDEX: usize = 6;
    const OSABI_INDEX: usize = 7;
    const ABIVERSION_INDEX: usize = 8;
    const PAD_START: usize = 9;

    fn check_ident(data: &[u8]) -> Result<(), ParseError> {
        if data.len() < Self::MAGIC.len() {
            return Err(ParseError::TooSmall);
        }
        if data[0..Self::MAGIC.len()] != Self::MAGIC {
            return Err(ParseError::InvalidMagic);
        }
        if data.len() < Self::INDENT_SIZE {
            return Err(ParseError::TooSmall);
        }

        let class = Class(data[Self::CLASS_INDEX]);
        if class != Class::ELF64 {
            return Err(ParseError::InvalidClass(class));
        }

        let endian = Endian(data[Self::DATA_INDEX]);
        if endian != Endian::native() {
            return Err(ParseError::UnsupportedEndian(endian));
        }

        let version = Version(data[Self::VERSION_INDEX]);
        if version != Version::CURRENT {
            return Err(ParseError::UnsupportedVersion(version));
        }

        if data[Self::PAD_START..Self::INDENT_SIZE]
            .iter()
            .any(|&b| b != 0)
        {
            return Err(ParseError::PaddingNotZero);
        }

        Ok(())
    }

    const fn os_abi(&self) -> OsAbi {
        OsAbi(self.ident[Self::OSABI_INDEX])
    }

    const fn abi_version(&self) -> AbiVersion {
        AbiVersion(self.ident[Self::ABIVERSION_INDEX])
    }
}

#[repr(C)]
struct Elf64SectionHeader {
    name_index: u32,
    ty: SectionType,
    flags: SectionFlags,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addr_align: u64,
    entry_size: u64,
}
const _: () = {
    ["Elf64SectionHeader size"][64 - mem::size_of::<Elf64SectionHeader>()];
};

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SectionType(u32);

impl SectionType {
    pub const NULL: Self = Self(0);
    pub const PROGBITS: Self = Self(1);
    pub const SYMTAB: Self = Self(2);
    pub const STRTAB: Self = Self(3);
    pub const RELA: Self = Self(4);
    pub const HASH: Self = Self(5);
    pub const DYNAMIC: Self = Self(6);
    pub const NOTE: Self = Self(7);
    pub const NOBITS: Self = Self(8);
    pub const REL: Self = Self(9);
    pub const SHLIB: Self = Self(10);
    pub const DYNSYM: Self = Self(11);
    pub const INIT_ARRAY: Self = Self(14);
    pub const FINI_ARRAY: Self = Self(15);
    pub const PREINIT_ARRAY: Self = Self(16);
    pub const GROUP: Self = Self(17);
    pub const SYMTAB_SHNDX: Self = Self(18);

    const LOOS: Self = Self(0x6000_0000);
    const HIOS: Self = Self(0x6fff_ffff);
    const LOPROC: Self = Self(0x7000_0000);
    const HIPROC: Self = Self(0x7fff_ffff);
    const LOUSER: Self = Self(0x8000_0000);
    const HIUSER: Self = Self(0xffff_ffff);

    pub const fn as_u32(self) -> u32 {
        self.0
    }

    pub const fn is_os_specific(self) -> bool {
        Self::LOOS.0 <= self.0 && self.0 <= Self::HIOS.0
    }

    pub const fn is_processor_specific(self) -> bool {
        Self::LOPROC.0 <= self.0 && self.0 <= Self::HIPROC.0
    }

    pub const fn is_application_specific(self) -> bool {
        #[allow(clippy::absurd_extreme_comparisons)]
        {
            Self::LOUSER.0 <= self.0 && self.0 <= Self::HIUSER.0
        }
    }
}

bitflags! {
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SectionFlags : u64 {
        const WRITE = 0x1;
        const ALLOC = 0x2;
        const EXECINSTR = 0x4;
        const MERGE = 0x10;
        const STRINGS = 0x20;
        const INFO_LINK = 0x40;
        const LINK_ORDER = 0x80;
        const OS_NONCONFORMING = 0x100;
        const GROUP = 0x200;
        const TLS = 0x400;
        const COMPRESSED = 0x800;
        const MASKOS = 0x0ff0_0000;
        const MASKPROC = 0xf000_0000;

        // The source may set any bits.
        const _ = !0;
    }
}

impl SectionFlags {
    pub const fn as_u64(self) -> u64 {
        self.bits()
    }
}
