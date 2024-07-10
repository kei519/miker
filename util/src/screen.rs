#[derive(Debug, Clone)]
pub struct FrameBufferInfo {
    pub format: PixelFormat,
    pub horizontal_resolution: usize,
    pub vertical_resolution: usize,
    pub pixels_per_scanline: usize,
    pub frame_buffer: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb,
    Bgr,
    Bitmask,
    Bitonly,
}
