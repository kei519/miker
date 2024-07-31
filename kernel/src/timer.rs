use core::sync::atomic::{AtomicU64, Ordering::*};

use util::{apic, asmfunc, bitfield::BitField as _, error::Result, sync::OnceStatic};

use crate::{
    acpi::FADT,
    interrupt::TIMER_INT_VEC,
    task::{Context, TASK_MANAGER},
};

/// Timer interrupt frequency in Hz.
// NOTE: It should be greater than `TASK_SWITCH_FREQ`. Otherwise, some task swithing would be
//       skiped.
pub const TIMER_INT_FREQ: u32 = 100;

/// Task swithing frequency in Hz.
pub const TASK_SWITCH_FREQ: u32 = 2;

// NOTE: We define it as `u64` to avoid overflowing when calculating how many count wait for
// `wait_for_msec`.
const PM_TIMER_FREQ: u64 = 3579545;

pub static APIC_TIMER_FREQ: OnceStatic<u32> = OnceStatic::new();

static COUNT: AtomicU64 = AtomicU64::new(0);

pub fn init() -> Result<()> {
    apic::set_divide_config(0);
    apic::start_count();
    wait_for_msec(1000);
    APIC_TIMER_FREQ.init(apic::elapsed_count());
    apic::stop_count();

    apic::set_lvt_timer(TIMER_INT_VEC, false, true);
    apic::set_divide_config(0);
    apic::set_init_count(APIC_TIMER_FREQ.get() / TIMER_INT_FREQ);

    Ok(())
}

#[no_mangle]
pub fn _int_handler_timer(prev_ctx: &Context) {
    let current = COUNT.fetch_add(1, Relaxed) + 1;
    apic::notify_end_of_interrupt();
    if current % (TIMER_INT_FREQ / TASK_SWITCH_FREQ) as u64 == 0 {
        // Safety: This is in interrupt handler and IF is not set.
        unsafe { TASK_MANAGER.switch(prev_ctx) };
    }
}

/// Wait for `msec` with ACPI PM timer.
pub fn wait_for_msec(msec: u32) {
    let fadt = FADT.get();

    // Avoid creating a reference to an unaligned field.
    let flags = fadt.flags;
    // If `ture`, pm_timer uses 32-bit number to count. Otherwise use 24-bit.
    let pm_timer_32 = flags.get_bit(8);

    let pm_tmr_blk_port = fadt.pm_tmr_blk as _;
    let start = asmfunc::io_in(pm_tmr_blk_port);
    let end = start + (PM_TIMER_FREQ * msec as u64 / 1000) as u32;
    let end = if pm_timer_32 { end } else { end.get_bits(..24) };

    if end < start {
        while asmfunc::io_in(pm_tmr_blk_port) >= start {}
    }
    while asmfunc::io_in(pm_tmr_blk_port) < end {}
}
