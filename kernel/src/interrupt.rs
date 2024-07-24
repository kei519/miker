//! Configure interrupts settings.

use util::{
    descriptor::{self, SystemDescriptor},
    error::Result,
    sync::OnceStatic,
};

/// Declares default interrupt handler function named `int_handler_<arg>` without an error code.
/// Declared function prints capital `arg`, RIP, CS, RFLAGS, RSP and SS on the screen if
/// [`FB_INFO`](kernel::screen::FB_IFNO) is already set.
macro_rules! fault_handler_no_error {
    ($name:ident) => {
        paste::paste! {
            #[util::interrupt_handler]
            fn [<int_handler_ $name:lower>](frame: &util::interrupt::InterruptFrame) {
                use core::fmt::Write as _;
                use util::graphics::GrayscalePrint as _;

                let mut buf = [0; 256];
                let mut buf = util::buffer::StrBuf::new(&mut buf);
                let _ = writeln!(buf, concat!("#", core::stringify!([<$name:upper>])));
                let _ = writeln!(buf, "CS:RIP {:04x} {:016x}", frame.cs, frame.rip);
                let _ = writeln!(buf, "RFLAGS {:016x}", frame.rflags);
                let _ = writeln!(buf, "SS:RSP {:04x} {:016x}", frame.ss, frame.rsp);
                let mut screen =
                    util::screen::GrayscaleScreen::new(crate::screen::FB_INFO.as_ref().clone());
                screen.print(buf.to_str(), (500, 0));
                loop {
                    util::asmfunc::hlt();
                }
            }
        }
    };
}

/// Declares default interrupt handler function named `int_handler_<arg>` with an error code.
/// Declared function prints capital `arg`, RIP, CS, RFLAGS, RSP, SS and error code on the screen if
/// [`FB_INFO`](kernel::screen::FB_IFNO) is already set.
macro_rules! fault_handler_with_error {
    ($name:ident) => {
        paste::paste! {
            #[util::interrupt_handler]
            fn [<int_handler_ $name:lower>](frame: &util::interrupt::InterruptFrame, error_code: u64) {
                use core::fmt::Write as _;
                use util::graphics::GrayscalePrint as _;

                let mut buf = [0; 256];
                let mut buf = util::buffer::StrBuf::new(&mut buf);
                let _ = writeln!(buf, concat!("#", core::stringify!([<$name:upper>])));
                let _ = writeln!(buf, "CS:RIP {:04x} {:016x}", frame.cs, frame.rip);
                let _ = writeln!(buf, "RFLAGS {:016x}", frame.rflags);
                let _ = writeln!(buf, "SS:RSP {:04x} {:016x}", frame.ss, frame.rsp);
                let _ = writeln!(buf, "ERR    {:016x}", error_code);
                let mut screen =
                    util::screen::GrayscaleScreen::new(crate::screen::FB_INFO.as_ref().clone());
                screen.print(buf.to_str(), (500, 0));
                loop {
                    util::asmfunc::hlt();
                }
            }
        }
    };
}

/// Interrupt Descriptor table for kernel initialized at the beggining.
static IDT: OnceStatic<descriptor::IDT> = OnceStatic::new();

/// Initialize IDT for kernel and load it to a processor..
pub fn init() -> Result<()> {
    let mut idt = descriptor::IDT::new();
    idt.set(
        0,
        SystemDescriptor::new_interrupt(int_handler_de as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        1,
        SystemDescriptor::new_interrupt(int_handler_db as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        3,
        SystemDescriptor::new_interrupt(int_handler_bp as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        4,
        SystemDescriptor::new_interrupt(int_handler_of as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        5,
        SystemDescriptor::new_interrupt(int_handler_br as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        6,
        SystemDescriptor::new_interrupt(int_handler_ud as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        7,
        SystemDescriptor::new_interrupt(int_handler_nm as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        8,
        SystemDescriptor::new_interrupt(int_handler_df as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        10,
        SystemDescriptor::new_interrupt(int_handler_ts as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        11,
        SystemDescriptor::new_interrupt(int_handler_np as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        12,
        SystemDescriptor::new_interrupt(int_handler_ss as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        13,
        SystemDescriptor::new_interrupt(int_handler_gp as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        14,
        SystemDescriptor::new_interrupt(int_handler_pf as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        16,
        SystemDescriptor::new_interrupt(int_handler_mf as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        17,
        SystemDescriptor::new_interrupt(int_handler_ac as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        18,
        SystemDescriptor::new_interrupt(int_handler_mc as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        19,
        SystemDescriptor::new_interrupt(int_handler_xm as _, 1 << 3, 0, 0),
    )?;
    idt.set(
        20,
        SystemDescriptor::new_interrupt(int_handler_ve as _, 1 << 3, 0, 0),
    )?;

    IDT.init(idt);
    IDT.as_ref().register();

    Ok(())
}

fault_handler_no_error!(DE);
fault_handler_no_error!(DB);
fault_handler_no_error!(BP);
fault_handler_no_error!(OF);
fault_handler_no_error!(BR);
fault_handler_no_error!(UD);
fault_handler_no_error!(NM);
fault_handler_with_error!(DF);
fault_handler_with_error!(TS);
fault_handler_with_error!(NP);
fault_handler_with_error!(SS);
fault_handler_with_error!(GP);
fault_handler_with_error!(PF);
fault_handler_no_error!(MF);
fault_handler_with_error!(AC);
fault_handler_no_error!(MC);
fault_handler_no_error!(XM);
fault_handler_no_error!(VE);
