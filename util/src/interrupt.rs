//! Provides interrupt utilities.

/// Information where the interrupt occurs, passed by a processor.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct InterruptFrame {
    /// RIP.
    pub rip: u64,
    /// CS.
    pub cs: u64,
    /// RFLAGS.
    pub rflags: u64,
    /// RSP.
    pub rsp: u64,
    /// SS.
    pub ss: u64,
}
