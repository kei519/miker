//! Configure interrupts settings.

use core::arch::global_asm;

use util::{
    descriptor::{self, SystemDescriptor},
    error::Result,
    sync::OnceStatic,
};

pub const TIMER_INT_VEC: u8 = 0x40;

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
        SystemDescriptor::new_interrupt(int_handler_de, 1 << 3, 0, 0),
    )?;
    idt.set(
        1,
        SystemDescriptor::new_interrupt(int_handler_db, 1 << 3, 0, 0),
    )?;
    idt.set(
        3,
        SystemDescriptor::new_interrupt(int_handler_bp, 1 << 3, 0, 0),
    )?;
    idt.set(
        4,
        SystemDescriptor::new_interrupt(int_handler_of, 1 << 3, 0, 0),
    )?;
    idt.set(
        5,
        SystemDescriptor::new_interrupt(int_handler_br, 1 << 3, 0, 0),
    )?;
    idt.set(
        6,
        SystemDescriptor::new_interrupt(int_handler_ud, 1 << 3, 0, 0),
    )?;
    idt.set(
        7,
        SystemDescriptor::new_interrupt(int_handler_nm, 1 << 3, 0, 0),
    )?;
    idt.set(
        8,
        SystemDescriptor::new_interrupt(int_handler_df, 1 << 3, 0, 0),
    )?;
    idt.set(
        10,
        SystemDescriptor::new_interrupt(int_handler_ts, 1 << 3, 0, 0),
    )?;
    idt.set(
        11,
        SystemDescriptor::new_interrupt(int_handler_np, 1 << 3, 0, 0),
    )?;
    idt.set(
        12,
        SystemDescriptor::new_interrupt(int_handler_ss, 1 << 3, 0, 0),
    )?;
    idt.set(
        13,
        SystemDescriptor::new_interrupt(int_handler_gp, 1 << 3, 0, 0),
    )?;
    idt.set(
        14,
        SystemDescriptor::new_interrupt(int_handler_pf, 1 << 3, 0, 0),
    )?;
    idt.set(
        16,
        SystemDescriptor::new_interrupt(int_handler_mf, 1 << 3, 0, 0),
    )?;
    idt.set(
        17,
        SystemDescriptor::new_interrupt(int_handler_ac, 1 << 3, 0, 0),
    )?;
    idt.set(
        18,
        SystemDescriptor::new_interrupt(int_handler_mc, 1 << 3, 0, 0),
    )?;
    idt.set(
        19,
        SystemDescriptor::new_interrupt(int_handler_xm, 1 << 3, 0, 0),
    )?;
    idt.set(
        20,
        SystemDescriptor::new_interrupt(int_handler_ve, 1 << 3, 0, 0),
    )?;
    idt.set(
        TIMER_INT_VEC as _,
        SystemDescriptor::new_interrupt(int_handler_timer, 1 << 3, 1, 0),
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

extern "sysv64" {
    /// Saves context before interrupt, and call [`_int_handler_tiemr`] with an argument, the
    /// reference to the context.
    fn int_handler_timer();
}

global_asm! { r#"
.global int_handler_timer
int_handler_timer:
    cli
    push rbp
    mov rbp, rsp

    # Direction flag should be unset before calling interrupt handler.
    cld

    # Construct `Context` on the stack
    sub rsp, 512
    fxsave [rsp]
    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push qword ptr [rbp]        # RBP
    push qword ptr [rbp + 0x20] # RSP
    push rsi
    push rdi
    push rdx
    push rcx
    push rbx
    push rax

    mov ax, gs
    mov bx, fs

    push rax                    # GS
    push rbx                    # FS
    push qword ptr [rbp + 0x28] # SS
    push qword ptr [rbp + 0x10] # CS
    push rbp                    # reserved1
    push qword ptr [rbp + 0x18] # RFLAGS
    push qword ptr [rbp + 0x08] # RIP

    mov rax, cr3
    push rax                    # CR3

    # Pass the reference to previous context as the first argument.
    lea rdi, [rsp]
    call _int_handler_timer

    # Discard up to GS
    add rsp, 8 * 8

    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rdi
    pop rsi

    # Discard RSP and RBP
    add rsp, 8 * 2

    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15
    fxrstor [rsp]

    mov rsp, rbp
    pop rbp
    iretq
"# }
