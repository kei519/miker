//! Provides structures used in ELF format.

use crate::bitfield::BitField;

/// Represents a 64-bit ELF executable header.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Elf64Ehdr {
    /// Idetifier.
    ///
    /// | Offset | Description |
    /// | :---: | :--- |
    /// | 3:0 | b"\x7FELF". |
    /// | 4 | 2 (Signifies the 64-bit format). |
    /// | 5 | Signifies the endian. |
    /// | 6 | 1 (original and current version of ELF). |
    /// | 7 | Identifies the target operating system ABI. |
    /// | 8 | Specifies the ABI version. |
    /// | 15:9 | Reserved. |
    pub ident: [u8; 16],
    /// Identifies object file type.
    pub ty: ElfFileType,
    /// Specifies target instruction set architecture.
    pub machine: u16,
    /// 1 for the original version of ELF.
    pub version: u32,
    /// Entry point virtual address.
    pub entry: u64,
    /// Program header table file offset.
    pub phoff: u64,
    /// Section header table file offset.
    pub shoff: u64,
    /// Flags depends on the target architecture.
    pub flags: u32,
    /// Contains the size of this header, normally 64 bytes.
    pub ehssize: u16,
    /// Contains the size of a program header table entry, typically 0x38.
    pub phentsize: u16,
    /// Contains the number of entries in the program header table.
    pub phnum: u16,
    /// Conatains the size of a section header table entry, typically 0x40.
    pub shentsize: u16,
    /// Contains the number of entries in the section header table.
    pub shnum: u16,
    /// Contains index of the section header table entry that contains the section names.
    pub shstrndx: u16,
}

/// Represents ELF file types.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfFileType {
    /// Unknown.
    None = 0,
    /// Relocatable file.
    Rel = 1,
    /// Executable file.
    Exec = 2,
    /// Shared object.
    Dyn = 3,
    /// Core file.
    Core = 4,
}

/// Represents a 64-bit ELF program header.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Elf64Phdr {
    /// Identifies the type of the segment.
    pub ty: ElfProgType,
    /// Segment-dependent flags.
    pub flags: ElfProgFlags,
    /// Segment file offset.
    pub offset: u64,
    /// Segment virtual address.
    pub vaddr: u64,
    /// Segment physical address.
    pub paddr: u64,
    /// Segment size in file.
    pub filesz: u64,
    /// Segment size in memory.
    pub memsz: u64,
    /// Segment alignment, file & memory.
    pub align: u64,
}

/// Represents a segment type in [Elf64Phdr].
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfProgType {
    /// Program header table entry unused.
    Null = 0,
    /// Loadable segment.
    Load = 1,
    /// Dynamic linking information.
    Dynamic = 2,
    /// Interprete information.
    Interp = 3,
    /// Auxilary information.
    Note = 4,
    /// Reserved.
    Shlib = 5,
    /// Segment containing program header table itself.
    Phdr = 6,
    /// Thread-Local Storage template.
    Tls = 7,
}

/// Represents flags of a segment in [Elf64Phdr].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfProgFlags(u32);

impl ElfProgFlags {
    const EXEC_BIT: u32 = 0;
    const WRITE_BIT: u32 = 1;
    const READ_BIT: u32 = 2;

    /// Returns whether the segment is executable.
    pub fn executable(&self) -> bool {
        self.0.get_bit(Self::EXEC_BIT)
    }

    /// Returns whether the segment is writable.
    pub fn writable(&self) -> bool {
        self.0.get_bit(Self::WRITE_BIT)
    }

    /// Returns whether the segment is readable.
    pub fn readable(&self) -> bool {
        self.0.get_bit(Self::READ_BIT)
    }
}
