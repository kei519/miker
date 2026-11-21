#![no_std]

pub fn hlt() {
    use core::arch::asm;

    cfg_select! {
        target_arch = "x86_64" => {
            //SAFETY: safe
            unsafe { asm!("hlt") };
        }
        target_arch = "aarch64" => {
            //SAFETY: safe
            unsafe { asm!("wfi") }
        }
    }
}
