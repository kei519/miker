use crate::graphics::GlayscalePixelWrite;

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

/// Represents screen painting with grayscale color.
///
/// Pixelf formats [PixelFormat::Bitmask] and [PixelFormat::Bitonly] are not supported.
#[derive(Debug)]
pub struct GlayscaleScreen {
    info: FrameBufferInfo,
}

impl GlayscaleScreen {
    /// Constructs [Screen] with [FrameBufferInfo] but if `info.format` is either
    /// [PixelFromat::Bitmask] or [PixelFormat::Bitonly], causes `panic`.
    pub fn new(info: FrameBufferInfo) -> Self {
        if matches!(info.format, PixelFormat::Bitmask | PixelFormat::Bitonly) {
            panic!("PixelFormat bitmask and bitonly are not supported");
        }
        Self { info }
    }
}

impl GlayscalePixelWrite for GlayscaleScreen {
    fn write(&mut self, pos: (usize, usize), color: u8) -> bool {
        if !(0..self.info.pixels_per_scanline).contains(&pos.0)
            || !(0..self.info.vertical_resolution).contains(&pos.1)
        {
            return false;
        }

        // Convert grayscale intensity to RGB color.
        let color = color as u32;
        // Since red, green and blue intensities are all same, we don't have to check format.
        let color = color << 16 | color << 8 | color;

        let addr = self.info.frame_buffer + (self.info.pixels_per_scanline * pos.1 + pos.0) * 4;
        unsafe { *(addr as *mut u32) = color };
        true
    }

    fn range(&self) -> (usize, usize) {
        (
            self.info.horizontal_resolution,
            self.info.vertical_resolution,
        )
    }
}
