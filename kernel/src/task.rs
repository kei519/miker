//! Handle scheduling.

use core::{arch::global_asm, mem, sync::atomic::AtomicI32};

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

/// Switch context from `current` to `next`.
pub fn switch_context(next: &mut Context, current: &mut Context, current_state: &AtomicI32) {
    unsafe { _switch_context(next, current, current_state) };
}

extern "sysv64" {
    fn _switch_context(next: &mut Context, current: &mut Context, current_state: &AtomicI32);
}

global_asm! { r#"
.global _switch_context
_switch_context:
    pushfq
    pop qword ptr [rsi + 0x10] # RFLAGS

    mov [rsi + 0x40], rax
    mov [rsi + 0x48], rbx
    mov [rsi + 0x50], rcx
    mov [rsi + 0x58], rdx
    mov [rsi + 0x60], rdi
    mov [rsi + 0x68], rsi

    mov rax, [rsp]
    mov [rsi + 0x08], rax # RIP

    mov rax, cr3
    mov [rsi + 0x00], rax # CR3

    mov ax, cs
    mov [rsi + 0x20], rax # CS
    mov ax, ss
    mov [rsi + 0x28], rax # SS
    mov ax, fs
    mov [rsi + 0x30], rax # FS
    mov ax, gs
    mov [rsi + 0x38], rax # GS

    lea rax, [rsp + 8]
    mov [rsi + 0x70], rax # RSP
    mov [rsi + 0x78], rbp
    mov [rsi + 0x80], r8
    mov [rsi + 0x88], r9
    mov [rsi + 0x90], r10
    mov [rsi + 0x98], r11
    mov [rsi + 0xa0], r12
    mov [rsi + 0xa8], r13
    mov [rsi + 0xb0], r14
    mov [rsi + 0xb8], r15
    fxsave [rsi + 0xc0]

    # Set current state to ready.
    mov dword ptr [rdx], 0

    # Constructs the next context frame.
    push qword ptr [rdi + 0x28] # SS
    push qword ptr [rdi + 0x70] # RSP
    push qword ptr [rdi + 0x10] # RFLAGS
    push qword ptr [rdi + 0x20] # CS
    push qword ptr [rdi + 0x08] # RIP

    fxrstor [rdi + 0xc0]
    mov r15, [rdi + 0xb8]
    mov r14, [rdi + 0xb0]
    mov r13, [rdi + 0xa8]
    mov r12, [rdi + 0xa0]
    mov r11, [rdi + 0x98]
    mov r10, [rdi + 0x90]
    mov r9, [rdi + 0x88]
    mov r8, [rdi + 0x80]
    mov rbp, [rdi + 0x78]

    mov rax, [rdi + 0x38] # GS
    mov rbx, [rdi + 0x30] # FS
    mov gs, ax
    mov fs, bx

    mov rax, [rdi + 0x00] # CR3
    mov cr3, rax

    mov rsi, [rdi + 0x68]
    mov rdx, [rdi + 0x58]
    mov rcx, [rdi + 0x50]
    mov rbx, [rdi + 0x48]
    mov rax, [rdi + 0x40]

    mov rdi, [rdi + 0x60]

    iretq
"#
}
