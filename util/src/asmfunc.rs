//! Provides functions that are codable only with assembly.

use core::arch::asm;

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
