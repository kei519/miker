use crate::bitfield::BitField;

/// Represents 64-bit ELF executable header.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Elf64Ehdr {
    pub ident: [u8; 16],
    pub ty: ElfFileType,
    pub machine: u16,
    pub version: u32,
    // Entry point virtual address.
    pub entry: u64,
    /// Program header table file offset.
    pub phoff: u64,
    /// Section header table file offset.
    pub shoff: u64,
    pub flags: u32,
    pub ehssize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

/// Represents ELF file types.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfFileType {
    None = 0,
    Rel = 1,
    Exec = 2,
    Dyn = 3,
    Core = 4,
    Loproc = 0xff00,
    Hiproc = 0xffff,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Elf64Phdr {
    pub ty: ElfProgType,
    pub flags: ElfProgFlags,
    /// Segment file offset.
    pub offset: u64,
    /// Segment virtual address.
    pub vaddr: u64,
    /// Segment physical address.
    paddr: u64,
    /// Segment size in file.
    pub filesz: u64,
    /// Segment size in memory.
    pub memsz: u64,
    /// Segment alignment, file & memory.
    pub align: u64,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfProgType {
    Null = 0,
    Load = 1,
    Dynamic = 2,
    Interp = 3,
    Note = 4,
    Shlib = 5,
    Phdr = 6,
    Tls = 7,
    Loos = 0x6000_0000,
    Hios = 0x6fff_ffff,
    Loproc = 0x7000_0000,
    Hiproc = 0x7fff_ffff,
    GnuEhFrame = 0x6474_e550,
    GnuProperty = 0x6474_e553,
    GnuStack = Self::Loos as u32 + 0x0474_e551,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfProgFlags(u32);

impl ElfProgFlags {
    const EXEC_BIT: u32 = 0;
    const WRITE_BIT: u32 = 1;
    const READ_BIT: u32 = 2;

    pub fn executable(&self) -> bool {
        self.0.get_bit(Self::EXEC_BIT)
    }

    pub fn writable(&self) -> bool {
        self.0.get_bit(Self::WRITE_BIT)
    }

    pub fn readable(&self) -> bool {
        self.0.get_bit(Self::READ_BIT)
    }
}
