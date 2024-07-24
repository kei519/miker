//! Provides tools for screen operations.

use util::{screen::FrameBufferInfo, sync::OnceStatic};

/// Frame buffer information initialized at the begining of kernel.
pub static FB_INFO: OnceStatic<FrameBufferInfo> = OnceStatic::new();
