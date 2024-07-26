//! Provides Local APIC utilities.

use crate::{bitfield::BitField, paging::ADDRESS_CONVERTER};

const EOI_ADDR: u64 = 0xfee0_00b0;

const LVT_TIMER_ADDR: u64 = 0xfee0_0320;
const INIT_COUNT_ADDR: u64 = 0xfee0_0380;
const CURRENT_COUNT_ADDR: u64 = 0xfee0_0390;
const DIVIDE_CONFIG_ADDR: u64 = 0xfee0_03e0;

/// Notify end of interrupt to Local APIC.
pub fn notify_end_of_interrupt() {
    unsafe { ADDRESS_CONVERTER.as_ref().write_volatile(EOI_ADDR, 1) };
}

/// Configure LVT timer register.
///
/// # Arguments
///
/// * `vector` - Interrupt vector number.
/// * `mask` - If `true`, inhibits reception of the interrupt.
/// * `periodic` - If `true`, reloads initial count registers, then count down again.
//
// TODO: Support TSC-Deadline mode.
pub fn set_lvt_timer(vector: u8, mask: bool, periodic: bool) {
    // Safety: `LVT_TIMER_ADDR` is valid because this is CPU-defined and properly aligned.
    unsafe {
        ADDRESS_CONVERTER.as_ref().write_volatile(
            LVT_TIMER_ADDR,
            (periodic as u32) << 17 | (mask as u32) << 16 | vector as u32,
        )
    };
}

/// Sets `value` to initial count register
pub fn set_init_count(value: u32) {
    // Safety: `INIT_COUNT_ADDR` is valid because this is CPU-defined and properly aligned.
    unsafe {
        ADDRESS_CONVERTER
            .as_ref()
            .write_volatile(INIT_COUNT_ADDR, value)
    };
}

/// Gets the value of initial count register.
///
/// # Panic
///
/// If [`ADDRESS_CONVERTER`] is not valid, causes panic.
pub fn get_init_count() -> u32 {
    // Safety: `INIT_COUNT_ADDR` is valid because this is CPU-defined and properly aligned.
    unsafe { ADDRESS_CONVERTER.as_ref().read(INIT_COUNT_ADDR).unwrap() }
}

/// Gets the value of current count register.
///
/// # Panic
///
/// If [`ADDRESS_CONVERTER`] is not valid, causes panic.
pub fn current_count() -> u32 {
    // Safety: `CURRENT_COUNT_ADDR` is valid because this is CPU-defined and properly aligned.
    unsafe { ADDRESS_CONVERTER.as_ref().read(CURRENT_COUNT_ADDR) }.unwrap()
}

/// Sets the APIC timer frequency as <base_clock> / 2^{`order`}. Max value of `order` is 7.
pub fn set_divide_config(order: u8) {
    let value = if order == 0 { 0b111 } else { order - 1 } as u32;
    // Safety: `CURRENT_COUNT_ADDR` is valid because this is CPU-defined and properly aligned.
    unsafe {
        ADDRESS_CONVERTER.as_ref().write_volatile(
            DIVIDE_CONFIG_ADDR,
            (value.get_bit(2) as u32) << 3 | value.get_bits(0..2),
        )
    };
}
