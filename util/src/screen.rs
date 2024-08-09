//! Provides types representing whole screen.

use core::slice;

use crate::graphics::{Color, GrayscalePixelWrite};

#[cfg(not(feature = "alloc"))]
pub use self::_no_alloc::*;

#[cfg(feature = "alloc")]
pub use self::_alloc::*;

/// Represents an information of frame buffer.
#[derive(Debug, Clone)]
pub struct FrameBufferInfo {
    /// Represents the frame buffer pixel format.
    pub format: PixelFormat,
    /// Represents horizontal resolution of the frame buffer.
    pub horizontal_resolution: usize,
    /// Represents vertical resolution of the frame buffer.
    pub vertical_resolution: usize,
    /// Number of pixelf per scanline.
    pub pixels_per_scanline: usize,
    /// Address of the frame buffer.
    pub frame_buffer: usize,
}

/// Represents pixel format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// Represents that a pixel uses 32 bit whose first 8 bits, second and third represent red,
    /// green and blue intensity respectively. 8 bits left are reserved.
    Rgb,
    /// Represents that a pixel uses 32 bit whose first 8 bits, second and third represent blue,
    /// green and red intensity respectively. 8 bits left are reserved.
    Bgr,
    /// Custom pixel format.
    Bitmask,
    /// The graphics mode does not support drawing directly to the frame buffer.
    Bitonly,
}

#[cfg(not(feature = "alloc"))]
mod _no_alloc {
    use super::*;

    /// Represents screen painting with grayscale color.
    ///
    /// Pixelf formats [PixelFormat::Bitmask] and [PixelFormat::Bitonly] are not supported.
    #[derive(Debug)]
    pub struct GrayscaleScreen {
        info: FrameBufferInfo,
    }

    impl GrayscaleScreen {
        /// Constructs [Screen] with [FrameBufferInfo] but if `info.format` is either
        /// [PixelFromat::Bitmask] or [PixelFormat::Bitonly], causes `panic`.
        pub fn new(info: FrameBufferInfo) -> Self {
            if matches!(info.format, PixelFormat::Bitmask | PixelFormat::Bitonly) {
                panic!("PixelFormat bitmask and bitonly are not supported");
            }
            Self { info }
        }

        /// Clear the whole screen.
        pub fn clear(&mut self) {
            let buf = unsafe {
                slice::from_raw_parts_mut(
                    self.info.frame_buffer as *mut u32,
                    self.info.pixels_per_scanline * self.info.vertical_resolution,
                )
            };
            for pixel in buf {
                *pixel = 0;
            }
        }
    }

    impl GrayscalePixelWrite for GrayscaleScreen {
        fn write(&mut self, pos: (usize, usize), color: u8) -> bool {
            if !(0..self.info.pixels_per_scanline).contains(&pos.0)
                || !(0..self.info.vertical_resolution).contains(&pos.1)
            {
                return false;
            }

            // Convert grayscale intensity to RGB color.
            let color = Color::from(color).into();

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
}

#[cfg(feature = "alloc")]
mod _alloc {
    use core::mem;

    use alloc::boxed::Box;

    use super::*;

    /// Represents screen painting with grayscale color.
    ///
    /// Pixelf formats [PixelFormat::Bitmask] and [PixelFormat::Bitonly] are not supported.
    #[derive(Debug)]
    pub struct GrayscaleScreen {
        fb: Box<[u32]>,
        horizontal_resolution: usize,
        pixels_per_scanline: usize,
    }

    impl GrayscaleScreen {
        /// Constructs [Screen] with [FrameBufferInfo] but if `info.format` is either
        /// [PixelFromat::Bitmask] or [PixelFormat::Bitonly], causes `panic`.
        pub fn new(info: FrameBufferInfo) -> Self {
            if matches!(info.format, PixelFormat::Bitmask | PixelFormat::Bitonly) {
                panic!("PixelFormat bitmask and bitonly are not supported");
            }

            let horizontal_resolution = info.horizontal_resolution;
            let pixels_per_scanline = info.pixels_per_scanline;
            let fb_slice = unsafe {
                slice::from_raw_parts_mut(
                    info.frame_buffer as *mut u32,
                    info.pixels_per_scanline * info.vertical_resolution,
                )
            };
            let raw_fb = fb_slice as *mut _;
            let fb = unsafe { Box::from_raw(raw_fb) };
            Self {
                fb,
                horizontal_resolution,
                pixels_per_scanline,
            }
        }

        /// Clear the whole screen.
        pub fn clear(&mut self) {
            for pixel in self.fb.iter_mut() {
                *pixel = 0;
            }
        }
    }

    impl GrayscalePixelWrite for GrayscaleScreen {
        fn write(&mut self, pos: (usize, usize), color: u8) -> bool {
            let Some(pixel) = self.fb.get_mut(self.pixels_per_scanline * pos.1 + pos.0) else {
                return false;
            };
            // Convert grayscale intensity to RGB color.
            let color = Color::from(color).into();
            *pixel = color;
            true
        }

        fn range(&self) -> (usize, usize) {
            let vert_res = self.fb.len() / self.pixels_per_scanline;
            (self.horizontal_resolution, vert_res)
        }
    }

    impl Drop for GrayscaleScreen {
        fn drop(&mut self) {
            // Since frame buffer must not be dropped, leak it.
            let fb = mem::replace(&mut self.fb, Box::new([]));
            Box::leak(fb);
        }
    }
}
