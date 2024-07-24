//! Provides functions that are codable only with assembly.

use core::arch::{asm, global_asm};

/// Allows interrupts.
pub fn sti() {
    unsafe { asm!("sti") };
}

/// Disables interrupts.
pub fn cli() {
    unsafe { asm!("cli") };
}

/// Halts a processor.
pub fn hlt() {
    unsafe { asm!("hlt") };
}

/// Returns the current CR3 value.
pub fn get_cr3() -> u64 {
    let cr3;
    unsafe { asm!("mov {}, cr3", out(reg) cr3) };
    cr3
}

/// Sets CR3 to `cr3`.
pub fn set_cr3(cr3: u64) {
    unsafe { asm!("mov cr3, {}", in(reg) cr3) };
}

/// Set all data segment registers (DS, ES, FS, GS) to `segment`.
pub fn set_ds_all(segment: u16) {
    unsafe {
        asm!(
            "mov ds, {seg:x}",
            "mov es, {seg:x}",
            "mov fs, {seg:x}",
            "mov gs, {seg:x}",
            seg = in(reg) segment,
        );
    }
}

/// Load GDT that starts at `base` and whose size is `size`. You do not need to subtract 1 from
/// `size` to make meet the x64 convention.
pub fn load_gdt(base: u64, size: usize) {
    unsafe {
        asm!(
            "push {:r}",
            "push {:x}",
            "lgdt [rsp]",
            "add rsp, 10",
            in(reg) base,
            // GDT's limit is assumed `size - 1`.
            in(reg) size - 1,
        )
    };
}

/// Load IDT that starts at `base` and whose size is `size`. You do not need to subtract 1 from
/// `size` to make meet the x64 convention.
pub fn load_idt(base: u64, size: usize) {
    unsafe {
        asm!(
            "push {:r}",
            "push {:x}",
            "lidt [rsp]",
            "add rsp, 10",
            in(reg) base,
            in(reg) size - 1,
        )
    };
}

/// Load `tr` to TR.
pub fn load_tr(tr: u16) {
    unsafe {
        asm!(
            "ltr {:x}",
            in(reg) tr,
        )
    };
}

/// Set CS and SS to `cs` and `ss`, respectively.
pub fn set_cs_ss(cs: u16, ss: u16) {
    unsafe { _set_cs_ss(cs, ss) };
}

extern "sysv64" {
    fn _set_cs_ss(cs: u16, ss: u16);
}

global_asm! {r#"
.global _set_cs_ss
_set_cs_ss:
    push rbp
    mov rbp, rsp
    mov ss, si
    push rdi
    lea rdi, [rip + .next]
    push rdi
    retfq
.next:
    mov rsp, rbp
    pop rbp
    ret
"#
}
