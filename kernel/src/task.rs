//! Handle scheduling.

use core::mem;

/// Process context that have to save when switching contexts.
#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct Context {
    /// CR3
    pub cr3: u64,
    /// RIP
    pub rip: u64,
    /// RFLAGS
    pub rflags: u64,
    /// RESERVED1
    pub reserved1: u64,
    /// CS
    pub cs: u64,
    /// SS
    pub ss: u64,
    /// FS
    pub fs: u64,
    /// GS
    pub gs: u64,
    /// RAX
    pub rax: u64,
    /// RBX
    pub rbx: u64,
    /// RCX
    pub rcx: u64,
    /// RDX
    pub rdx: u64,
    /// RDI
    pub rdi: u64,
    /// RSI
    pub rsi: u64,
    /// RSP
    pub rsp: u64,
    /// RBP
    pub rbp: u64,
    /// R8
    pub r8: u64,
    /// R9
    pub r9: u64,
    /// R10
    pub r10: u64,
    /// R11
    pub r11: u64,
    /// R12
    pub r12: u64,
    /// R13
    pub r13: u64,
    /// R14
    pub r14: u64,
    /// R15
    pub r15: u64,
    /// FX
    pub fxsave_area: [u8; 512],
}

impl Context {
    /// Construct a new [`Context`] all of whose fields are zero.
    pub const fn new() -> Self {
        // Safety: It is valid `Context` value, whose all fields are zero.
        unsafe { mem::zeroed() }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
